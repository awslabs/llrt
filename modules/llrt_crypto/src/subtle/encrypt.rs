// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use aes::cipher::typenum::U12;
use aes_gcm::Nonce;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Class, Ctx, Result};
use rsa::rand_core::OsRng;

use super::{
    algorithm_mismatch_error, encryption_algorithm::EncryptionAlgorithm,
    key_algorithm::KeyAlgorithm, rsa_private_key, AesCbcEncVariant, AesCtrVariant, AesGcmVariant,
    CryptoKey,
};

pub async fn subtle_encrypt<'js>(
    ctx: Ctx<'js>,
    algorithm: EncryptionAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("encrypt").or_throw(&ctx)?;

    let bytes = encrypt(&ctx, &algorithm, &key, data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn encrypt(
    ctx: &Ctx<'_>,
    algorithm: &EncryptionAlgorithm,
    key: &CryptoKey,
    data: &[u8],
) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();
    match algorithm {
        EncryptionAlgorithm::AesCbc { iv } => {
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                let variant = AesCbcEncVariant::new(length, handle, iv).or_throw(ctx)?;
                Ok(variant.encrypt(data))
            } else {
                algorithm_mismatch_error(ctx)
            }
        },
        EncryptionAlgorithm::AesCtr { counter, length } => {
            let encryption_length = length;
            if let KeyAlgorithm::Aes { length } = key.algorithm {
                let mut variant = AesCtrVariant::new(length, *encryption_length, handle, counter)
                    .or_throw(ctx)?;
                variant.encrypt(data).or_throw(ctx)
            } else {
                algorithm_mismatch_error(ctx)
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
                    .encrypt(nonce, data, additional_data.as_deref())
                    .or_throw(ctx)
            } else {
                algorithm_mismatch_error(ctx)
            }
        },
        EncryptionAlgorithm::RsaOaep { label } => {
            if let KeyAlgorithm::Rsa { hash, .. } = &key.algorithm {
                let (private_key, padding) = rsa_private_key(ctx, handle, label, hash)?;
                let public_key = private_key.to_public_key();
                let mut rng = OsRng;

                public_key.encrypt(&mut rng, padding, data).or_throw(ctx)
            } else {
                algorithm_mismatch_error(ctx)
            }
        },
    }
}
