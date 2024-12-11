// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, typenum::U12, BlockDecryptMut, KeyIvInit};
use aes_gcm::Nonce;
use ctr::cipher::StreamCipher;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Class, Ctx, Result};

use crate::subtle::{
    Aes128Ctr128, Aes128Ctr32, Aes128Ctr64, Aes192Ctr128, Aes192Ctr32, Aes192Ctr64, Aes256Ctr128,
    Aes256Ctr32, Aes256Ctr64, CryptoKey,
};

use super::{
    algorithm_missmatch_error, encryption_algorithm::EncryptionAlgorithm,
    key_algorithm::KeyAlgorithm, rsa_private_key, AesGcmVariant,
};

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes192CbcDec = cbc::Decryptor<aes::Aes192>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub async fn subtle_decrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: EncryptionAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("decrypt").or_throw(&ctx)?;
    let bytes = decrypt(&ctx, &algorithm, &key, data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn decrypt(
    ctx: &Ctx<'_>,
    algorithm: &EncryptionAlgorithm,
    key: &CryptoKey,
    data: &[u8],
) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();
    match algorithm {
        EncryptionAlgorithm::AesCbc { iv } => {
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                match length {
                    128 => decrypt_aes_cbc_gen::<Aes128CbcDec>(ctx, handle, iv, data),
                    192 => decrypt_aes_cbc_gen::<Aes192CbcDec>(ctx, handle, iv, data),
                    256 => decrypt_aes_cbc_gen::<Aes256CbcDec>(ctx, handle, iv, data),
                    _ => unreachable!(), // 'length' has already been sanitized.
                }
            } else {
                algorithm_missmatch_error(ctx)
            }
        },
        EncryptionAlgorithm::AesCtr { counter, length } => {
            let encryption_length = length;
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                match (length, encryption_length) {
                    (128, 32) => decrypt_aes_ctr_gen::<Aes128Ctr32>(ctx, handle, counter, data),
                    (128, 64) => decrypt_aes_ctr_gen::<Aes128Ctr64>(ctx, handle, counter, data),
                    (128, 128) => decrypt_aes_ctr_gen::<Aes128Ctr128>(ctx, handle, counter, data),
                    (192, 32) => decrypt_aes_ctr_gen::<Aes192Ctr32>(ctx, handle, counter, data),
                    (192, 64) => decrypt_aes_ctr_gen::<Aes192Ctr64>(ctx, handle, counter, data),
                    (192, 128) => decrypt_aes_ctr_gen::<Aes192Ctr128>(ctx, handle, counter, data),
                    (256, 32) => decrypt_aes_ctr_gen::<Aes256Ctr32>(ctx, handle, counter, data),
                    (256, 64) => decrypt_aes_ctr_gen::<Aes256Ctr64>(ctx, handle, counter, data),
                    (256, 128) => decrypt_aes_ctr_gen::<Aes256Ctr128>(ctx, handle, counter, data),
                    _ => unreachable!(), // 'length' has already been sanitized.
                }
            } else {
                algorithm_missmatch_error(ctx)
            }
        },
        EncryptionAlgorithm::AesGcm {
            iv,
            tag_length,
            additional_data,
        } => {
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                let nonce = Nonce::<U12>::from_slice(iv);

                let variant = AesGcmVariant::new(length, *tag_length, handle).or_throw(ctx)?;
                variant
                    .decrypt(nonce, data, additional_data.as_deref())
                    .or_throw(ctx)
            } else {
                algorithm_missmatch_error(ctx)
            }
        },
        EncryptionAlgorithm::RsaOaep { label } => {
            if let KeyAlgorithm::Rsa { hash, .. } = &key.algorithm {
                let (private_key, padding) = rsa_private_key(ctx, handle, label, hash)?;

                private_key.decrypt(padding, data).or_throw(ctx)
            } else {
                algorithm_missmatch_error(ctx)
            }
        },
    }
}

fn decrypt_aes_cbc_gen<T>(ctx: &Ctx<'_>, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    T: KeyIvInit + BlockDecryptMut,
{
    T::new(key.into(), iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .or_throw(ctx)
}

fn decrypt_aes_ctr_gen<T>(ctx: &Ctx<'_>, key: &[u8], counter: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    T: KeyIvInit + StreamCipher,
{
    let mut cipher = T::new(key.into(), counter.into());

    let mut plaintext = data.to_vec();
    cipher.try_apply_keystream(&mut plaintext).or_throw(ctx)?;

    Ok(plaintext)
}
