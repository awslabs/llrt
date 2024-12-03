// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use aes_gcm::{aead::Aead, KeyInit, Nonce};
use ctr::{cipher::StreamCipher, Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};
use rsa::{pkcs1::DecodeRsaPrivateKey, rand_core::OsRng, sha2::Sha256, Oaep, RsaPrivateKey};

use crate::subtle::{
    check_supported_usage, extract_algorithm_object, Aes256Gcm, Algorithm, CryptoKey,
};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

pub async fn subtle_encrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: CryptoKey<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    check_supported_usage(&ctx, &key.usages(), "encrypt")?;

    let algorithm = extract_algorithm_object(&ctx, &algorithm)?;

    let bytes = encrypt(&ctx, &algorithm, key.get_handle(), data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn encrypt(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::AesCbc { iv } => Ok(Aes256CbcEnc::new(key.into(), iv.as_slice().into())
            .encrypt_padded_vec_mut::<Pkcs7>(data)),
        Algorithm::AesCtr { counter, length } => match length {
            32 => encrypt_aes_ctr_gen::<Ctr32BE<aes::Aes256>>(ctx, key, counter, data),
            64 => encrypt_aes_ctr_gen::<Ctr64BE<aes::Aes256>>(ctx, key, counter, data),
            128 => encrypt_aes_ctr_gen::<Ctr128BE<aes::Aes256>>(ctx, key, counter, data),
            _ => Err(Exception::throw_message(
                ctx,
                "invalid counter length. Currently supported 32/64/128 bits",
            )),
        },
        Algorithm::AesGcm { iv } => {
            let cipher = Aes256Gcm::new_from_slice(key).or_throw(ctx)?;
            let nonce = Nonce::from_slice(iv);

            cipher.encrypt(nonce, data.as_ref()).or_throw(ctx)
        },
        Algorithm::RsaOaep { label } => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key)
                .or_throw(ctx)?
                .to_public_key();
            let padding = label.as_ref().map_or(Oaep::new::<Sha256>(), |buf| {
                Oaep::new_with_label::<Sha256, _>(&String::from_utf8_lossy(buf))
            });
            let mut rng = OsRng;

            Ok(public_key.encrypt(&mut rng, padding, data).or_throw(ctx)?)
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}

fn encrypt_aes_ctr_gen<T>(ctx: &Ctx<'_>, key: &[u8], counter: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    T: KeyIvInit + StreamCipher,
{
    let mut cipher = T::new(key.into(), counter.into());

    let mut ciphertext = data.to_vec();
    cipher.try_apply_keystream(&mut ciphertext).or_throw(ctx)?;

    Ok(ciphertext)
}
