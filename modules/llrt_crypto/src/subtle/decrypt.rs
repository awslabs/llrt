// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use aes_gcm::{aead::Aead, KeyInit, Nonce};
use ctr::{cipher::StreamCipher, Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};
use rsa::{
    sha2::Sha256,
    {pkcs1::DecodeRsaPrivateKey, Oaep, RsaPrivateKey},
};

use crate::subtle::{check_supported_usage, extract_algorithm_object, Aes256Gcm, Algorithm};

use super::CryptoKey;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub async fn subtle_decrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: CryptoKey<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    check_supported_usage(&ctx, &key.usages(), "decrypt")?;

    let algorithm = extract_algorithm_object(&ctx, &algorithm)?;

    let bytes = decrypt(&ctx, &algorithm, key.get_handle(), data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn decrypt(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::AesCbc(iv) => Aes256CbcDec::new(key.into(), iv.as_slice().into())
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .or_throw(ctx),
        Algorithm::AesCtr(counter, length) => match length {
            32 => decrypt_aes_ctr_gen::<Ctr32BE<aes::Aes256>>(ctx, key, counter, data),
            64 => decrypt_aes_ctr_gen::<Ctr64BE<aes::Aes256>>(ctx, key, counter, data),
            128 => decrypt_aes_ctr_gen::<Ctr128BE<aes::Aes256>>(ctx, key, counter, data),
            _ => Err(Exception::throw_message(
                ctx,
                "invalid counter length. Currently supported 32/64/128 bits",
            )),
        },
        Algorithm::AesGcm(iv) => {
            let cipher = Aes256Gcm::new_from_slice(key).or_throw(ctx)?;
            let nonce = Nonce::from_slice(iv);

            cipher.decrypt(nonce, data.as_ref()).or_throw(ctx)
        },
        Algorithm::RsaOaep(label) => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let padding = label.as_ref().map_or(Oaep::new::<Sha256>(), |buf| {
                Oaep::new_with_label::<Sha256, _>(&String::from_utf8_lossy(buf))
            });

            private_key.decrypt(padding, data).or_throw(ctx)
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
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
