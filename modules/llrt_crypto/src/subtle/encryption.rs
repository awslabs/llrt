use std::borrow::Cow;

// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};

use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Result};

use crate::{
    provider::{AesMode, CryptoProvider},
    CRYPTO_PROVIDER,
};

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
                    let tag_len = (*tag_length as usize) / 8;
                    if data.len() < tag_len {
                        return Err(Exception::throw_message(ctx, "Invalid ciphertext length"));
                    }
                    // Pass the full data (ciphertext + tag) to the decrypt function
                    CRYPTO_PROVIDER
                        .aes_decrypt(
                            AesMode::Gcm {
                                tag_length: *tag_length,
                            },
                            handle,
                            iv,
                            data,
                            aad,
                        )
                        .or_throw(ctx)?
                },
            }
        },
        EncryptionAlgorithm::AesKw => {
            let padding = match mode {
                EncryptionMode::Encryption => {
                    return Err(Exception::throw_message(
                        ctx,
                        "AES-KW can only be used for wrapping keys",
                    ));
                },
                EncryptionMode::Wrapping(padding) => padding,
            };

            match operation {
                EncryptionOperation::Encrypt => {
                    // Pad data to multiple of 8 bytes if needed
                    let mut padded_data = Cow::Borrowed(data);
                    if !data.len().is_multiple_of(8) {
                        let pad_len = 8 - (data.len() % 8);
                        let mut padded = data.to_vec();
                        padded.extend(std::iter::repeat_n(padding, pad_len));
                        padded_data = Cow::Owned(padded)
                    }
                    CRYPTO_PROVIDER
                        .aes_kw_wrap(handle, &padded_data)
                        .or_throw(ctx)?
                },
                EncryptionOperation::Decrypt => {
                    let unwrapped = CRYPTO_PROVIDER.aes_kw_unwrap(handle, data).or_throw(ctx)?;
                    // Remove padding if present
                    if padding != 0 {
                        let trimmed: Vec<u8> = unwrapped
                            .into_iter()
                            .rev()
                            .skip_while(|&b| b == padding)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                        trimmed
                    } else {
                        unwrapped
                    }
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
