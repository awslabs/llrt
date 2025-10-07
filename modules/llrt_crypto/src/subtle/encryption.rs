// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::typenum::U12;
use aes_gcm::Nonce;
use aes_kw::{KeyInit, KwAes128, KwAes192, KwAes256};
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};

use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Result};
use rsa::{
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    Oaep, RsaPrivateKey, RsaPublicKey,
};

use crate::sha_hash::ShaAlgorithm;

use super::{
    algorithm_mismatch_error, encryption_algorithm::EncryptionAlgorithm, extract_aes_length,
    key_algorithm::KeyAlgorithm, AesCbcDecVariant, AesCbcEncVariant, AesCtrVariant, AesGcmVariant,
    CryptoKey, EncryptionMode,
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
            let length = extract_aes_length(ctx, key, "AES-CBC")?;
            match operation {
                EncryptionOperation::Encrypt => {
                    let variant = AesCbcEncVariant::new(length, handle, iv).or_throw(ctx)?;
                    variant.encrypt(data)
                },
                EncryptionOperation::Decrypt => {
                    let variant = AesCbcDecVariant::new(length, handle, iv).or_throw(ctx)?;
                    variant.decrypt(data).or_throw(ctx)?
                },
            }
        },
        EncryptionAlgorithm::AesCtr {
            counter,
            length: encryption_length,
        } => {
            let length = extract_aes_length(ctx, key, "AES-CTR")?;

            let mut variant =
                AesCtrVariant::new(length, *encryption_length, handle, counter).or_throw(ctx)?;
            match operation {
                EncryptionOperation::Encrypt => variant.encrypt(data).or_throw(ctx)?,
                EncryptionOperation::Decrypt => variant.decrypt(data).or_throw(ctx)?,
            }
        },
        EncryptionAlgorithm::AesGcm {
            iv,
            tag_length,
            additional_data,
        } => {
            let length = extract_aes_length(ctx, key, "AES-GCM")?;

            let nonce: &ctr::cipher::Array<_, _> =
                &Nonce::<U12>::try_from(iv.as_ref()).or_throw(ctx)?;

            let variant = AesGcmVariant::new(length, *tag_length, handle).or_throw(ctx)?;
            match operation {
                EncryptionOperation::Encrypt => variant
                    .encrypt(nonce, data, additional_data.as_deref())
                    .or_throw(ctx)?,
                EncryptionOperation::Decrypt => variant
                    .decrypt(nonce, data, additional_data.as_deref())
                    .or_throw(ctx)?,
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

            let is_encrypt = matches!(operation, EncryptionOperation::Encrypt);

            //Only create new vec if padding is needed, otherwise use original slice
            let data = if !data.len().is_multiple_of(8) && is_encrypt && padding != 0 {
                let padding_size = (8 - (data.len() % 8)) % 8;
                let mut padded = Vec::with_capacity(data.len() + padding_size);
                padded.extend_from_slice(data);
                padded.resize(data.len() + padding_size, padding);
                std::borrow::Cow::Owned(padded)
            } else {
                std::borrow::Cow::Borrowed(data)
            };

            match handle.len() {
                16 => {
                    let kek = KwAes128::new(handle.try_into().or_throw(ctx)?);
                    match operation {
                        EncryptionOperation::Encrypt => {
                            let mut buf = vec![0u8; data.len() + 8];
                            let result = kek.wrap_key(&data, &mut buf).or_throw(ctx)?;
                            rquickjs::Result::Ok(result.to_vec())
                        },
                        EncryptionOperation::Decrypt => {
                            let mut buf = vec![0u8; data.len()];
                            let result = kek.unwrap_key(&data, &mut buf).or_throw(ctx)?;
                            Ok(result.to_vec())
                        },
                    }
                },
                24 => {
                    let kek = KwAes192::new(handle.try_into().or_throw(ctx)?);
                    match operation {
                        EncryptionOperation::Encrypt => {
                            let mut buf = vec![0u8; data.len() + 8];
                            let result = kek.wrap_key(&data, &mut buf).or_throw(ctx)?;
                            Ok(result.to_vec())
                        },
                        EncryptionOperation::Decrypt => {
                            let mut buf = vec![0u8; data.len()];
                            let result = kek.unwrap_key(&data, &mut buf).or_throw(ctx)?;
                            Ok(result.to_vec())
                        },
                    }
                },
                32 => {
                    let kek = KwAes256::new(handle.try_into().or_throw(ctx)?);
                    match operation {
                        EncryptionOperation::Encrypt => {
                            let mut buf = vec![0u8; data.len() + 8];
                            let result = kek.wrap_key(&data, &mut buf).or_throw(ctx)?;
                            Ok(result.to_vec())
                        },
                        EncryptionOperation::Decrypt => {
                            let mut buf = vec![0u8; data.len()];
                            let result = kek.unwrap_key(&data, &mut buf).or_throw(ctx)?;
                            Ok(result.to_vec())
                        },
                    }
                },
                _ => return Err(Exception::throw_message(ctx, "Invalid AES-KW key length")),
            }
            .or_throw(ctx)?
        },
        EncryptionAlgorithm::RsaOaep { label } => {
            let hash = match &key.algorithm {
                KeyAlgorithm::Rsa { hash, .. } => hash,
                _ => return algorithm_mismatch_error(ctx, "RSA-OAEP"),
            };
            let padding = rsa_oaep_padding(ctx, label, hash)?;
            match operation {
                EncryptionOperation::Encrypt => {
                    let public_key = RsaPublicKey::from_pkcs1_der(handle).or_throw(ctx)?;
                    let mut rng = rand::rng();
                    public_key.encrypt(&mut rng, padding, data).or_throw(ctx)?
                },
                EncryptionOperation::Decrypt => {
                    let private_key = RsaPrivateKey::from_pkcs1_der(handle).or_throw(ctx)?;

                    private_key.decrypt(padding, data).or_throw(ctx)?
                },
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
