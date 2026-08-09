// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, future::Future};

use llrt_exceptions::DOMException;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, FromJs, Result, Value};

use crate::{
    provider::{AesMode, CryptoProvider},
    CRYPTO_PROVIDER,
};

use super::{
    algorithm_mismatch_error,
    encryption_algorithm::EncryptionAlgorithm,
    key_algorithm::{AesAlgorithm, KeyAlgorithm},
    util::ResultDomExt,
    CryptoKey, EncryptionMode,
};

pub fn subtle_decrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    let prepared = prepare_encrypt_decrypt(&ctx, algorithm, key, data);

    async move {
        let (algorithm, key, input) = prepared?;

        let key = key.borrow();
        key.check_validity("decrypt").or_throw_dom(&ctx)?;

        let bytes = encrypt_decrypt(
            &ctx,
            &algorithm,
            &key,
            &input,
            EncryptionMode::Encryption,
            EncryptionOperation::Decrypt,
        )?;
        ArrayBuffer::new(ctx, bytes)
    }
}

pub fn subtle_encrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    let prepared = prepare_encrypt_decrypt(&ctx, algorithm, key, data);

    async move {
        let (algorithm, key, input) = prepared?;

        let key = key.borrow();
        key.check_validity("encrypt").or_throw_dom(&ctx)?;

        let bytes = encrypt_decrypt(
            &ctx,
            &algorithm,
            &key,
            &input,
            EncryptionMode::Encryption,
            EncryptionOperation::Encrypt,
        )?;
        ArrayBuffer::new(ctx, bytes)
    }
}

fn prepare_encrypt_decrypt<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    data: ObjectBytes<'js>,
) -> Result<(EncryptionAlgorithm, Class<'js, CryptoKey<'js>>, Vec<u8>)> {
    let algorithm = EncryptionAlgorithm::from_js(ctx, algorithm)?;
    let input = data.as_bytes_opt().map(<[u8]>::to_vec).unwrap_or_default();
    Ok((algorithm, key, input))
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
            validate_aes_length(ctx, key, handle, AesAlgorithm::Cbc)?;

            match operation {
                EncryptionOperation::Encrypt => CRYPTO_PROVIDER
                    .aes_encrypt(AesMode::Cbc, handle, iv, data, None)
                    .or_throw_dom(ctx)?,
                EncryptionOperation::Decrypt => CRYPTO_PROVIDER
                    .aes_decrypt(AesMode::Cbc, handle, iv, data, None)
                    .or_throw_dom(ctx)?,
            }
        },
        EncryptionAlgorithm::AesCtr {
            counter,
            length: encryption_length,
        } => {
            validate_aes_length(ctx, key, handle, AesAlgorithm::Ctr)?;
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
                    .or_throw_dom(ctx)?,
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
                    .or_throw_dom(ctx)?,
            }
        },
        EncryptionAlgorithm::AesGcm {
            iv,
            tag_length,
            additional_data,
        } => {
            validate_aes_length(ctx, key, handle, AesAlgorithm::Gcm)?;
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
                    .or_throw_dom(ctx)?,
                EncryptionOperation::Decrypt => {
                    let tag_len = (*tag_length as usize) / 8;
                    if data.len() < tag_len {
                        return Err(DOMException::operation_error(
                            ctx,
                            "Invalid ciphertext length",
                        ));
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
                        .or_throw_dom(ctx)?
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
                        .or_throw_dom(ctx)?
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
                    .or_throw_dom(ctx)?,
                EncryptionOperation::Decrypt => CRYPTO_PROVIDER
                    .rsa_oaep_decrypt(handle, data, *hash, label.as_deref())
                    .or_throw_dom(ctx)?,
            }
        },
    };
    Ok(bytes)
}

pub fn validate_aes_length(
    ctx: &Ctx<'_>,
    key: &CryptoKey,
    handle: &[u8],
    expected_algorithm: AesAlgorithm,
) -> Result<()> {
    match &key.algorithm {
        KeyAlgorithm::Aes { algorithm, length } if *algorithm == expected_algorithm => {
            if *length != handle.len() as u16 * 8 {
                return Err(DOMException::operation_error(ctx, "Invalid AES key length"));
            }
            Ok(())
        },
        KeyAlgorithm::Aes { .. } => Err(DOMException::invalid_access_error(
            ctx,
            "AES algorithm mismatch",
        )),

        _ => algorithm_mismatch_error(ctx, "AES"),
    }
}
