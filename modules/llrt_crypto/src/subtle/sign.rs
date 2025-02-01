// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rand::rngs::OsRng;
use ring::{
    hmac::{Context as HmacContext, Key as HmacKey},
    signature::{EcdsaKeyPair, Ed25519KeyPair},
};
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Result};
use rsa::{
    pss::Pss,
    sha2::{Sha256, Sha384, Sha512},
    Pkcs1v15Sign,
};

use crate::{sha_hash::ShaAlgorithm, subtle::CryptoKey, SYSTEM_RANDOM};

use super::{
    algorithm_mismatch_error, key_algorithm::KeyAlgorithm, rsa_private_key,
    sign_algorithm::SigningAlgorithm,
};

pub async fn subtle_sign<'js>(
    ctx: Ctx<'js>,
    algorithm: SigningAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("sign").or_throw(&ctx)?;

    let bytes = sign(&ctx, &algorithm, &key, data.as_bytes(&ctx)?)?;
    ArrayBuffer::new(ctx, bytes)
}

fn sign(
    ctx: &Ctx<'_>,
    algorithm: &SigningAlgorithm,
    key: &CryptoKey,
    data: &[u8],
) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();
    Ok(match algorithm {
        SigningAlgorithm::Ecdsa { hash } => {
            // Get hash algorithm from key
            if !matches!(&key.algorithm, KeyAlgorithm::Ec { .. }) {
                return algorithm_mismatch_error(ctx, "ECDSA");
            };

            let hash_alg = match hash {
                ShaAlgorithm::SHA256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                ShaAlgorithm::SHA384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                _ => {
                    return Err(Exception::throw_message(
                        ctx,
                        "Ecdsa.hash only support Sha256 or Sha384",
                    ))
                },
            };
            let rng = &(*SYSTEM_RANDOM);
            let key_pair = EcdsaKeyPair::from_pkcs8(hash_alg, handle, rng).or_throw(ctx)?;
            let signature = key_pair.sign(rng, data).or_throw(ctx)?;

            signature.as_ref().to_vec()
        },
        SigningAlgorithm::Ed25519 => {
            // Verify key algorithm
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_mismatch_error(ctx, "Ed25519");
            }
            let key_pair = Ed25519KeyPair::from_pkcs8(handle).or_throw(ctx)?;
            let signature = key_pair.sign(data);

            signature.as_ref().to_vec()
        },
        SigningAlgorithm::Hmac => {
            let hash = if let KeyAlgorithm::Hmac { hash, .. } = &key.algorithm {
                hash
            } else {
                return algorithm_mismatch_error(ctx, "HMAC");
            };

            let hmac_alg = hash.hmac_algorithm();

            let key = HmacKey::new(*hmac_alg, handle);
            let mut hmac = HmacContext::with_key(&key);
            hmac.update(data);

            hmac.sign().as_ref().to_vec()
        },
        SigningAlgorithm::RsaPss { salt_length } => {
            let salt_length = *salt_length as usize;

            let mut rng = OsRng;

            rsa_sign(ctx, key, "RSA-PSS", data, |hash, digest, private_key| {
                // let key_size = key.handle.len();
                // let digest_size = digest.len();
                // let max_salt_len =
                //     ((key_size as f64 - 1.0) / 8.0).ceil() as usize - digest_size - 2;
                match hash {
                    ShaAlgorithm::SHA256 => private_key
                        .sign_with_rng(
                            &mut rng,
                            Pss::new_with_salt::<rsa::sha2::Sha256>(salt_length),
                            digest,
                        )
                        .or_throw(ctx),
                    ShaAlgorithm::SHA384 => private_key
                        .sign_with_rng(
                            &mut rng,
                            Pss::new_with_salt::<rsa::sha2::Sha384>(salt_length),
                            digest,
                        )
                        .or_throw(ctx),
                    ShaAlgorithm::SHA512 => private_key
                        .sign_with_rng(
                            &mut rng,
                            Pss::new_with_salt::<rsa::sha2::Sha512>(salt_length),
                            digest,
                        )
                        .or_throw(ctx),
                    ShaAlgorithm::SHA1 => unreachable!(),
                }
            })?
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let mut rng = OsRng;

            rsa_sign(
                ctx,
                key,
                "RSA-PSS",
                data,
                |hash, digest, private_key| match hash {
                    ShaAlgorithm::SHA256 => private_key
                        .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha256>(), digest)
                        .or_throw(ctx),
                    ShaAlgorithm::SHA384 => private_key
                        .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha384>(), digest)
                        .or_throw(ctx),
                    ShaAlgorithm::SHA512 => private_key
                        .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha512>(), digest)
                        .or_throw(ctx),
                    ShaAlgorithm::SHA1 => unreachable!(),
                },
            )?

            // let hash = match &key.algorithm {
            //     KeyAlgorithm::Rsa { hash, .. } => hash,
            //     _ => {
            //         return Err(Exception::throw_message(
            //             ctx,
            //             "Invalid key algorithm for RSASSA-PKCS1-v1_5",
            //         ))
            //     },
            // };

            // let private_key = RsaPrivateKey::from_pkcs1_der(handle).or_throw(ctx)?;

            // match hash {
            //     ShaAlgorithm::SHA256 => {
            //         private_key.sign(Pkcs1v15Sign::new::<rsa::sha2::Sha256>(), digest)
            //     },

            //     ShaAlgorithm::SHA384 => {
            //         private_key.sign(Pkcs1v15Sign::new::<rsa::sha2::Sha384>(), digest)
            //     },
            //     ShaAlgorithm::SHA512 => {
            //         private_key.sign(Pkcs1v15Sign::new::<rsa::sha2::Sha512>(), digest)
            //     },
            //     _ => {
            //         return Err(Exception::throw_message(
            //             ctx,
            //             "Unsupported hash algorithm for RSA-PSS",
            //         ))
            //     },
            // }
            // .or_throw(ctx)?
        },
    })
}

// Helper function for RSA signing
fn rsa_sign<F>(
    ctx: &Ctx<'_>,
    key: &CryptoKey,
    algorithm_name: &str,
    data: &[u8],
    sign_fn: F,
) -> Result<Vec<u8>>
where
    F: FnOnce(&ShaAlgorithm, &[u8], &rsa::RsaPrivateKey) -> Result<Vec<u8>>,
{
    let (private_key, hash, digest) = rsa_private_key(ctx, key, data, algorithm_name)?;

    if !matches!(
        hash,
        ShaAlgorithm::SHA256 | ShaAlgorithm::SHA384 | ShaAlgorithm::SHA512
    ) {
        return Err(Exception::throw_message(
            ctx,
            "Invalid hash algorithm for RSA verification",
        ));
    }

    sign_fn(hash, digest.as_ref(), &private_key)
}
