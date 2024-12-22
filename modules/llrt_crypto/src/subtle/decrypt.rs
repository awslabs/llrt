// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::typenum::U12;
use aes_gcm::Nonce;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Class, Ctx, Result};

use super::{
    algorithm_missmatch_error, encryption_algorithm::EncryptionAlgorithm,
    key_algorithm::KeyAlgorithm, rsa_private_key, AesCbcDecVariant, AesCtrVariant, AesGcmVariant,
    CryptoKey,
};

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
                let variant = AesCbcDecVariant::new(length, handle, iv).or_throw(ctx)?;
                variant.decrypt(data).or_throw(ctx)
            } else {
                algorithm_missmatch_error(ctx)
            }
        },
        EncryptionAlgorithm::AesCtr { counter, length } => {
            let encryption_length = length;
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                let mut variant = AesCtrVariant::new(length, *encryption_length, handle, counter)
                    .or_throw(ctx)?;
                variant.decrypt(data).or_throw(ctx)
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
