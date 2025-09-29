// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};

use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Result};

use crate::{
    provider::{AesMode, CryptoProvider},
    CRYPTO_PROVIDER,
};

use crate::sha_hash::ShaAlgorithm;

pub(super) enum OaepPadding {
    Sha256(Oaep<rsa::sha2::Sha256>),
    Sha384(Oaep<rsa::sha2::Sha384>),
    Sha512(Oaep<rsa::sha2::Sha512>),
}

use super::{
    algorithm_mismatch_error, encryption_algorithm::EncryptionAlgorithm,
    key_algorithm::KeyAlgorithm, validate_aes_length, CryptoKey, EncryptionMode,
};

pub async fn subtle_decrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: EncryptionAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("decrypt").or_throw(&ctx)?;
    let bytes = encrypt_decrypt(
        &ctx,
        &algorithm,
        &key,
        data.as_bytes(&ctx)?,
        EncryptionMode::Encryption,
        EncryptionOperation::Decrypt,
    )?;
    ArrayBuffer::new(ctx, bytes)
}

pub async fn subtle_encrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: EncryptionAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("encrypt").or_throw(&ctx)?;

    let bytes = encrypt_decrypt(
        &ctx,
        &algorithm,
        &key,
        data.as_bytes(&ctx)?,
        EncryptionMode::Encryption,
        EncryptionOperation::Encrypt,
    )?;
    ArrayBuffer::new(ctx, bytes)
}

pub enum EncryptionOperation {
    Encrypt,
    Decrypt,
}

pub fn encrypt_decrypt(
    ctx: &Ctx<'_>,
    algorithm: &EncryptionAlgorithm,
    key: &CryptoKey,
    data: &[u8],
    mode: EncryptionMode,
    operation: EncryptionOperation,
) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();
    let bytes = match algorithm {
        EncryptionAlgorithm::AesCbc { iv } => {
            validate_aes_length(ctx, key, handle, "AES-CBC")?;

            match operation {
                EncryptionOperation::Encrypt => CRYPTO_PROVIDER
                    .aes_encrypt(AesMode::Cbc, handle, iv, data, None)
                    .or_throw(ctx)?,
                EncryptionOperation::Decrypt => CRYPTO_PROVIDER
                    .aes_decrypt(AesMode::Cbc, handle, iv, data, None)
                    .or_throw(ctx)?,
            }
        },
        EncryptionAlgorithm::AesCtr {
            counter,
            length: encryption_length,
        } => {
            validate_aes_length(ctx, key, handle, "AES-CTR")?;
            match operation {
                EncryptionOperation::Encrypt => CRYPTO_PROVIDER
                    .aes_encrypt(
                        AesMode::Ctr {
                            counter_length: *encryption_length,
                        },
                        handle,
                        counter,
                        data,
                        None,
                    )
                    .or_throw(ctx)?,
                EncryptionOperation::Decrypt => CRYPTO_PROVIDER
                    .aes_decrypt(
                        AesMode::Ctr {
                            counter_length: *encryption_length,
                        },
                        handle,
                        counter,
                        data,
                        None,
                    )
                    .or_throw(ctx)?,
            }
        },
        EncryptionAlgorithm::AesGcm {
            iv,
            tag_length,
            additional_data,
        } => {
            validate_aes_length(ctx, key, handle, "AES-GCM")?;
            let aad = additional_data.as_deref();

            match operation {
                EncryptionOperation::Encrypt => CRYPTO_PROVIDER
                    .aes_encrypt(
                        AesMode::Gcm {
                            tag_length: *tag_length,
                        },
                        handle,
                        iv,
                        data,
                        aad,
                    )
                    .or_throw(ctx)?,
                EncryptionOperation::Decrypt => {
                    if data.len() < 16 {
                        return Err(Exception::throw_message(ctx, "Invalid ciphertext length"));
                    }
                    let (ciphertext, tag) = data.split_at(data.len() - 16);
                    CRYPTO_PROVIDER
                        .aes_decrypt(
                            AesMode::Gcm {
                                tag_length: *tag_length,
                            },
                            handle,
                            iv,
                            ciphertext,
                            aad,
                        )
                        .or_throw(ctx)?
                },
            }
        },
        EncryptionAlgorithm::AesKw => {
            let _padding = match mode {
                EncryptionMode::Encryption => {
                    return Err(Exception::throw_message(
                        ctx,
                        "AES-KW can only be used for wrapping keys",
                    ));
                },
                EncryptionMode::Wrapping(_padding) => _padding,
            };

            match operation {
                EncryptionOperation::Encrypt => {
                    CRYPTO_PROVIDER.aes_kw_wrap(handle, data).or_throw(ctx)?
                },
                EncryptionOperation::Decrypt => {
                    CRYPTO_PROVIDER.aes_kw_unwrap(handle, data).or_throw(ctx)?
                },
            }
        },
        EncryptionAlgorithm::RsaOaep { label } => {
            let hash = match &key.algorithm {
                KeyAlgorithm::Rsa { hash, .. } => hash,
                _ => return algorithm_mismatch_error(ctx, "RSA-OAEP"),
            };

            match operation {
                EncryptionOperation::Encrypt => CRYPTO_PROVIDER
                    .rsa_oaep_encrypt(handle, data, *hash, label.as_deref())
                    .or_throw(ctx)?,
                EncryptionOperation::Decrypt => CRYPTO_PROVIDER
                    .rsa_oaep_decrypt(handle, data, *hash, label.as_deref())
                    .or_throw(ctx)?,
            }
        },
    };
    Ok(bytes)
}

pub fn rsa_oaep_padding(
    ctx: &Ctx<'_>,
    label: &Option<Box<[u8]>>,
    hash: &ShaAlgorithm,
) -> Result<Oaep> {
    let mut padding = match hash {
        ShaAlgorithm::SHA1 => {
            return Err(Exception::throw_message(
                ctx,
                "SHA-1 is not supported for RSA-OAEP",
            ));
        },
        ShaAlgorithm::SHA256 => Oaep::new::<rsa::sha2::Sha256>(),
        ShaAlgorithm::SHA384 => Oaep::new::<rsa::sha2::Sha384>(),
        ShaAlgorithm::SHA512 => Oaep::new::<rsa::sha2::Sha512>(),
    };
    if let Some(label) = label {
        if !label.is_empty() {
            padding.label = Some(label.to_owned());
        }
    }

    Ok(padding)
}
