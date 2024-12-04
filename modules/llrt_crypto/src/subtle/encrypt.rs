// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, typenum::U12, KeyIvInit};
use aes_gcm::{aead::Aead, KeyInit, Nonce};
use ctr::{
    cipher::{BlockEncryptMut, StreamCipher},
    Ctr128BE, Ctr32BE, Ctr64BE,
};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};
use rsa::{pkcs1::DecodeRsaPrivateKey, rand_core::OsRng, sha2::Sha256, Oaep, RsaPrivateKey};

use crate::subtle::{
    check_supported_usage, extract_algorithm_object, Aes128Gcm, Aes192Gcm, Aes256Gcm, Algorithm,
    CryptoKey,
};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes192CbcEnc = cbc::Encryptor<aes::Aes192>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

type Aes32CtrEnc = Ctr32BE<aes::Aes256>;
type Aes64CtrEnc = Ctr64BE<aes::Aes256>;
type Aes128CtrEnc = Ctr128BE<aes::Aes256>;

pub async fn subtle_encrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: CryptoKey<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    check_supported_usage(&ctx, &key.usages(), "encrypt")?;

    let algorithm = extract_algorithm_object(&ctx, &algorithm)?;

    let bytes = encrypt(&ctx, &algorithm, &key, data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn encrypt(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::AesCbc { iv } => {
            let length = key.algorithm().get_optional("length")?.unwrap_or(0);
            match length {
                128 => encrypt_aes_cbc_gen::<Aes128CbcEnc>(ctx, key.get_handle(), iv, data),
                192 => encrypt_aes_cbc_gen::<Aes192CbcEnc>(ctx, key.get_handle(), iv, data),
                256 => encrypt_aes_cbc_gen::<Aes256CbcEnc>(ctx, key.get_handle(), iv, data),
                _ => unreachable!(), // 'length' has already been sanitized.
            }
        },
        Algorithm::AesCtr { counter, length } => match length {
            32 => encrypt_aes_ctr_gen::<Aes32CtrEnc>(ctx, key.get_handle(), counter, data),
            64 => encrypt_aes_ctr_gen::<Aes64CtrEnc>(ctx, key.get_handle(), counter, data),
            128 => encrypt_aes_ctr_gen::<Aes128CtrEnc>(ctx, key.get_handle(), counter, data),
            _ => unreachable!(), // 'length' has already been sanitized.
        },
        Algorithm::AesGcm { iv } => {
            let nonce = Nonce::<U12>::from_slice(iv);
            let length = key.algorithm().get_optional("length")?.unwrap_or(0);
            match length {
                128 => {
                    let cipher = Aes128Gcm::new_from_slice(key.get_handle()).or_throw(ctx)?;
                    cipher.encrypt(nonce, data.as_ref()).or_throw(ctx)
                },
                192 => {
                    let cipher = Aes192Gcm::new_from_slice(key.get_handle()).or_throw(ctx)?;
                    cipher.encrypt(nonce, data.as_ref()).or_throw(ctx)
                },
                256 => {
                    let cipher = Aes256Gcm::new_from_slice(key.get_handle()).or_throw(ctx)?;
                    cipher.encrypt(nonce, data.as_ref()).or_throw(ctx)
                },
                _ => unreachable!(), // 'length' has already been sanitized.
            }
        },
        Algorithm::RsaOaep { label } => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key.get_handle())
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

fn encrypt_aes_cbc_gen<T>(_ctx: &Ctx<'_>, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    T: KeyIvInit + BlockEncryptMut,
{
    Ok(T::new(key.into(), iv.into()).encrypt_padded_vec_mut::<Pkcs7>(data))
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
