// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use aes_gcm::{aead::Aead, KeyInit, Nonce};
use ctr::{cipher::StreamCipher, Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Exception, Result};
use rsa::{pkcs1::DecodeRsaPrivateKey, Oaep, RsaPrivateKey};
use sha2::Sha256;

use crate::subtle::{Aes256Gcm, Algorithm};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn decrypt(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::AesGcm(iv) => {
            let cipher = Aes256Gcm::new_from_slice(key).or_throw(ctx)?;
            let nonce = Nonce::from_slice(iv);

            match cipher.decrypt(nonce, data.as_ref()) {
                Ok(result) => Ok(result),
                Err(_) => Err(Exception::throw_message(ctx, "Decryption failed"))?,
            }
        },
        Algorithm::AesCbc(iv) => {
            match Aes256CbcDec::new(key.into(), iv.as_slice().into())
                .decrypt_padded_vec_mut::<Pkcs7>(data)
            {
                Ok(result) => Ok(result),
                Err(_) => Err(Exception::throw_message(ctx, "Decryption failed")),
            }
        },
        Algorithm::AesCtr(counter, length) => match length {
            32 => decrypt_aes_ctr_gen::<Ctr32BE<aes::Aes256>>(ctx, key, counter, data),
            64 => decrypt_aes_ctr_gen::<Ctr64BE<aes::Aes256>>(ctx, key, counter, data),
            128 => decrypt_aes_ctr_gen::<Ctr128BE<aes::Aes256>>(ctx, key, counter, data),
            _ => Err(Exception::throw_message(
                ctx,
                "invalid counter length. Currently supported 32/64/128 bits",
            )),
        },
        Algorithm::RsaOaep(label) => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let padding = match label {
                Some(buf) => {
                    Oaep::new_with_label::<Sha256, String>(String::from_utf8(buf.to_vec())?)
                },
                None => Oaep::new::<Sha256>(),
            };

            private_key
                .decrypt(padding, data)
                .map_err(|e| Exception::throw_message(ctx, e.to_string().as_str()))
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}

fn decrypt_aes_ctr_gen<B>(ctx: &Ctx<'_>, key: &[u8], counter: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    B: KeyIvInit + StreamCipher,
{
    let mut cipher = B::new(key.into(), counter.into());

    let mut plaintext = data.to_vec();
    cipher
        .try_apply_keystream(&mut plaintext)
        .map_err(|_| Exception::throw_message(ctx, "tried to decrypt too much data"))?;

    Ok(plaintext)
}
