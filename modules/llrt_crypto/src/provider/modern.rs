// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use chacha20poly1305::{
    aead::{Aead, Payload},
    ChaCha20Poly1305, KeyInit, Nonce,
};
use ml_dsa::{
    pkcs8::{
        der::AnyRef, spki::AssociatedAlgorithmIdentifier, DecodePrivateKey, DecodePublicKey,
        EncodePrivateKey, EncodePublicKey,
    },
    Keypair, MlDsa44, MlDsa65, MlDsa87, MlDsaParams, Seed, Signature, SigningKey, VerifyingKey,
};
use ml_kem::{
    Decapsulate, EncapsulationKey as MlKemEncapsulationKey, Key as MlKemKey,
    KeyExport as MlKemKeyExport, MlKem1024, MlKem512, MlKem768, Seed as MlKemSeed,
    B32 as MlKemRandomness,
};

use super::{CryptoError, HybridKemVariant, MlDsaVariant, MlKemVariant};

trait MlDsaParameterSet: MlDsaParams + AssociatedAlgorithmIdentifier<Params = AnyRef<'static>> {}

impl MlDsaParameterSet for MlDsa44 {}
impl MlDsaParameterSet for MlDsa65 {}
impl MlDsaParameterSet for MlDsa87 {}

macro_rules! dispatch_ml_dsa {
    ($variant:expr, $function:ident $(, $argument:expr)* $(,)?) => {
        match $variant {
            MlDsaVariant::MlDsa44 => $function::<MlDsa44>($($argument),*),
            MlDsaVariant::MlDsa65 => $function::<MlDsa65>($($argument),*),
            MlDsaVariant::MlDsa87 => $function::<MlDsa87>($($argument),*),
        }
    };
}

macro_rules! dispatch_ml_kem {
    ($variant:expr, |$kem:ident| $body:block) => {
        match $variant {
            MlKemVariant::MlKem512 => {
                type $kem = MlKem512;
                $body
            },
            MlKemVariant::MlKem768 => {
                type $kem = MlKem768;
                $body
            },
            MlKemVariant::MlKem1024 => {
                type $kem = MlKem1024;
                $body
            },
        }
    };
}

pub(crate) fn chacha20_poly1305_encrypt(
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey(None))?;
    let nonce = Nonce::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: data,
                aad: additional_data.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed(None))
}

pub(crate) fn chacha20_poly1305_decrypt(
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey(None))?;
    let nonce = Nonce::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: data,
                aad: additional_data.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed(None))
}

fn ml_dsa_signing_key<P: MlDsaParameterSet>(seed: &[u8]) -> Result<SigningKey<P>, CryptoError> {
    let seed = Seed::try_from(seed).map_err(|_| CryptoError::InvalidKey(None))?;
    Ok(SigningKey::from_seed(&seed))
}

fn ml_dsa_verifying_key<P: MlDsaParameterSet>(
    public_key: &[u8],
) -> Result<VerifyingKey<P>, CryptoError> {
    let encoded = ml_dsa::EncodedVerifyingKey::<P>::try_from(public_key)
        .map_err(|_| CryptoError::InvalidKey(None))?;
    Ok(VerifyingKey::decode(&encoded))
}

fn generate_ml_dsa_key_for<P: MlDsaParameterSet>() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let seed = crate::random_byte_array(32);
    let signing_key = ml_dsa_signing_key::<P>(&seed)?;
    let public_key = signing_key.verifying_key().encode().to_vec();
    Ok((seed, public_key))
}

pub(crate) fn generate_ml_dsa_key(
    variant: MlDsaVariant,
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    dispatch_ml_dsa!(variant, generate_ml_dsa_key_for)
}

fn ml_dsa_public_key_for<P: MlDsaParameterSet>(seed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    Ok(ml_dsa_signing_key::<P>(seed)?
        .verifying_key()
        .encode()
        .to_vec())
}

pub(crate) fn ml_dsa_public_key(
    variant: MlDsaVariant,
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, ml_dsa_public_key_for, seed)
}

fn ml_dsa_sign_for<P: MlDsaParameterSet>(
    seed: &[u8],
    data: &[u8],
    context: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let signing_key = ml_dsa_signing_key::<P>(seed)?;
    let mut rng = rand::rng();
    signing_key
        .expanded_key()
        .sign_randomized(data, context, &mut rng)
        .map(|signature| signature.encode().to_vec())
        .map_err(|_| CryptoError::SigningFailed(None))
}

pub(crate) fn ml_dsa_sign(
    variant: MlDsaVariant,
    seed: &[u8],
    data: &[u8],
    context: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, ml_dsa_sign_for, seed, data, context)
}

fn ml_dsa_verify_for<P: MlDsaParameterSet>(
    public_key: &[u8],
    signature: &[u8],
    data: &[u8],
    context: &[u8],
) -> Result<bool, CryptoError> {
    let verifying_key = ml_dsa_verifying_key::<P>(public_key)?;
    let signature =
        Signature::<P>::try_from(signature).map_err(|_| CryptoError::InvalidSignature(None))?;
    Ok(verifying_key.verify_with_context(data, context, &signature))
}

pub(crate) fn ml_dsa_verify(
    variant: MlDsaVariant,
    public_key: &[u8],
    signature: &[u8],
    data: &[u8],
    context: &[u8],
) -> Result<bool, CryptoError> {
    dispatch_ml_dsa!(
        variant,
        ml_dsa_verify_for,
        public_key,
        signature,
        data,
        context,
    )
}

fn import_ml_dsa_public_key_for<P: MlDsaParameterSet>(
    data: &[u8],
    spki: bool,
) -> Result<Vec<u8>, CryptoError> {
    let key = if spki {
        VerifyingKey::<P>::from_public_key_der(data).map_err(|_| CryptoError::InvalidKey(None))?
    } else {
        ml_dsa_verifying_key::<P>(data)?
    };
    Ok(key.encode().to_vec())
}

pub(crate) fn import_ml_dsa_public_key(
    variant: MlDsaVariant,
    data: &[u8],
    spki: bool,
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, import_ml_dsa_public_key_for, data, spki)
}

fn import_ml_dsa_private_key_for<P: MlDsaParameterSet>(
    data: &[u8],
    pkcs8: bool,
) -> Result<Vec<u8>, CryptoError> {
    let key = if pkcs8 {
        SigningKey::<P>::from_pkcs8_der(data).map_err(|_| CryptoError::InvalidKey(None))?
    } else {
        ml_dsa_signing_key::<P>(data)?
    };
    Ok(key.to_seed().to_vec())
}

pub(crate) fn import_ml_dsa_private_key(
    variant: MlDsaVariant,
    data: &[u8],
    pkcs8: bool,
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, import_ml_dsa_private_key_for, data, pkcs8)
}

fn export_ml_dsa_public_key_spki_for<P: MlDsaParameterSet>(
    public_key: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    ml_dsa_verifying_key::<P>(public_key)?
        .to_public_key_der()
        .map(|document| document.as_bytes().to_vec())
        .map_err(|_| CryptoError::InvalidKey(None))
}

pub(crate) fn export_ml_dsa_public_key_spki(
    variant: MlDsaVariant,
    public_key: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, export_ml_dsa_public_key_spki_for, public_key,)
}

fn export_ml_dsa_private_key_pkcs8_for<P: MlDsaParameterSet>(
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    ml_dsa_signing_key::<P>(seed)?
        .to_pkcs8_der()
        .map(|document| document.as_bytes().to_vec())
        .map_err(|_| CryptoError::InvalidKey(None))
}

pub(crate) fn export_ml_dsa_private_key_pkcs8(
    variant: MlDsaVariant,
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_dsa!(variant, export_ml_dsa_private_key_pkcs8_for, seed,)
}

pub(crate) fn generate_ml_kem_key(
    variant: MlKemVariant,
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let seed = crate::random_byte_array(64);
    let seed = MlKemSeed::try_from(seed.as_slice()).map_err(|_| CryptoError::InvalidKey(None))?;
    dispatch_ml_kem!(variant, |Kem| {
        let private_key = ml_kem::DecapsulationKey::<Kem>::from_seed(seed);
        let public_key = private_key.encapsulation_key().to_bytes().to_vec();
        Ok((seed.to_vec(), public_key))
    })
}

pub(crate) fn ml_kem_public_key(
    variant: MlKemVariant,
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let seed = MlKemSeed::try_from(seed).map_err(|_| CryptoError::InvalidKey(None))?;
    dispatch_ml_kem!(variant, |Kem| {
        let private_key = ml_kem::DecapsulationKey::<Kem>::from_seed(seed);
        Ok(private_key.encapsulation_key().to_bytes().to_vec())
    })
}

pub(crate) fn ml_kem_encapsulate(
    variant: MlKemVariant,
    public_key: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let randomness = crate::random_byte_array(32);
    let randomness = MlKemRandomness::try_from(randomness.as_slice())
        .map_err(|_| CryptoError::OperationFailed(None))?;
    dispatch_ml_kem!(variant, |Kem| {
        let encoded = MlKemKey::<MlKemEncapsulationKey<Kem>>::try_from(public_key)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let public_key = MlKemEncapsulationKey::<Kem>::new(&encoded)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let (ciphertext, shared_key) = public_key.encapsulate_deterministic(&randomness);
        Ok((ciphertext.to_vec(), shared_key.to_vec()))
    })
}

pub(crate) fn ml_kem_decapsulate(
    variant: MlKemVariant,
    seed: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let seed = MlKemSeed::try_from(seed).map_err(|_| CryptoError::InvalidKey(None))?;
    dispatch_ml_kem!(variant, |Kem| {
        let private_key = ml_kem::DecapsulationKey::<Kem>::from_seed(seed);
        private_key
            .decapsulate_slice(ciphertext)
            .map(|shared_key| shared_key.to_vec())
            .map_err(|_| CryptoError::OperationFailed(Some("Invalid ML-KEM ciphertext".into())))
    })
}

pub(crate) fn import_ml_kem_public_key(
    variant: MlKemVariant,
    data: &[u8],
    spki: bool,
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_kem!(variant, |Kem| {
        let key = if spki {
            MlKemEncapsulationKey::<Kem>::from_public_key_der(data)
                .map_err(|_| CryptoError::InvalidKey(None))?
        } else {
            let encoded = MlKemKey::<MlKemEncapsulationKey<Kem>>::try_from(data)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            MlKemEncapsulationKey::<Kem>::new(&encoded)
                .map_err(|_| CryptoError::InvalidKey(None))?
        };
        Ok(key.to_bytes().to_vec())
    })
}

pub(crate) fn import_ml_kem_private_key(
    variant: MlKemVariant,
    data: &[u8],
    pkcs8: bool,
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_kem!(variant, |Kem| {
        let key = if pkcs8 {
            ml_kem::DecapsulationKey::<Kem>::from_pkcs8_der(data)
                .map_err(|_| CryptoError::InvalidKey(None))?
        } else {
            let seed = MlKemSeed::try_from(data).map_err(|_| CryptoError::InvalidKey(None))?;
            ml_kem::DecapsulationKey::<Kem>::from_seed(seed)
        };
        key.to_seed()
            .map(|seed| seed.to_vec())
            .ok_or(CryptoError::InvalidKey(None))
    })
}

pub(crate) fn export_ml_kem_public_key_spki(
    variant: MlKemVariant,
    public_key: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    dispatch_ml_kem!(variant, |Kem| {
        let encoded = MlKemKey::<MlKemEncapsulationKey<Kem>>::try_from(public_key)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        MlKemEncapsulationKey::<Kem>::new(&encoded)
            .map_err(|_| CryptoError::InvalidKey(None))?
            .to_public_key_der()
            .map(|document| document.as_bytes().to_vec())
            .map_err(|_| CryptoError::InvalidKey(None))
    })
}

pub(crate) fn export_ml_kem_private_key_pkcs8(
    variant: MlKemVariant,
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let seed = MlKemSeed::try_from(seed).map_err(|_| CryptoError::InvalidKey(None))?;
    dispatch_ml_kem!(variant, |Kem| {
        ml_kem::DecapsulationKey::<Kem>::from_seed(seed)
            .to_pkcs8_der()
            .map(|document| document.as_bytes().to_vec())
            .map_err(|_| CryptoError::InvalidKey(None))
    })
}

enum TraditionalPrivateKey {
    P256(p256::SecretKey),
    X25519(x25519_dalek::StaticSecret),
    P384(p384::SecretKey),
}

struct HybridKeyPair {
    pq_seed: Vec<u8>,
    traditional_private_key: TraditionalPrivateKey,
    traditional_public_key: Vec<u8>,
    public_key: Vec<u8>,
}

fn shake256(input: &[u8], output_length: usize) -> Vec<u8> {
    use sha3::digest::{ExtendableOutput, Update, XofReader};

    let mut output = vec![0; output_length];
    let mut hash = sha3::Shake256::default();
    hash.update(input);
    hash.finalize_xof().read(&mut output);
    output
}

fn p256_private_key(seed: &[u8]) -> Result<p256::SecretKey, CryptoError> {
    seed.as_chunks::<32>()
        .0
        .iter()
        .find_map(|candidate| p256::SecretKey::from_slice(candidate).ok())
        .ok_or(CryptoError::OperationFailed(Some(
            "P-256 rejection sampling failed".into(),
        )))
}

fn p384_private_key(seed: &[u8]) -> Result<p384::SecretKey, CryptoError> {
    seed.as_chunks::<48>()
        .0
        .iter()
        .find_map(|candidate| p384::SecretKey::from_slice(candidate).ok())
        .ok_or(CryptoError::OperationFailed(Some(
            "P-384 rejection sampling failed".into(),
        )))
}

fn derive_hybrid_key_pair(
    variant: HybridKemVariant,
    seed: &[u8],
) -> Result<HybridKeyPair, CryptoError> {
    if seed.len() != 32 {
        return Err(CryptoError::InvalidKey(None));
    }
    let traditional_seed_length = match variant {
        HybridKemVariant::MlKem768P256 => 128,
        HybridKemVariant::MlKem768X25519 => 32,
        HybridKemVariant::MlKem1024P384 => 48,
    };
    let expanded = shake256(seed, 64 + traditional_seed_length);
    let (pq_seed, traditional_seed) = expanded.split_at(64);
    let pq_public_key = ml_kem_public_key(variant.ml_kem_variant(), pq_seed)?;

    let (traditional_private_key, traditional_public_key) = match variant {
        HybridKemVariant::MlKem768P256 => {
            let private_key = p256_private_key(traditional_seed)?;
            let public_key = private_key.public_key().to_sec1_bytes().to_vec();
            (TraditionalPrivateKey::P256(private_key), public_key)
        },
        HybridKemVariant::MlKem768X25519 => {
            let private_key = x25519_dalek::StaticSecret::from(
                <[u8; 32]>::try_from(traditional_seed)
                    .map_err(|_| CryptoError::OperationFailed(None))?,
            );
            let public_key = x25519_dalek::PublicKey::from(&private_key)
                .as_bytes()
                .to_vec();
            (TraditionalPrivateKey::X25519(private_key), public_key)
        },
        HybridKemVariant::MlKem1024P384 => {
            let private_key = p384_private_key(traditional_seed)?;
            let public_key = private_key.public_key().to_sec1_bytes().to_vec();
            (TraditionalPrivateKey::P384(private_key), public_key)
        },
    };

    let mut public_key = pq_public_key;
    public_key.extend_from_slice(&traditional_public_key);
    Ok(HybridKeyPair {
        pq_seed: pq_seed.to_vec(),
        traditional_private_key,
        traditional_public_key,
        public_key,
    })
}

fn hybrid_kem_combiner(
    variant: HybridKemVariant,
    pq_shared_key: &[u8],
    traditional_shared_key: &[u8],
    traditional_ciphertext: &[u8],
    traditional_public_key: &[u8],
) -> Vec<u8> {
    use sha3::Digest;

    let label: &[u8] = match variant {
        HybridKemVariant::MlKem768P256 => b"MLKEM768-P256",
        HybridKemVariant::MlKem768X25519 => b"\\.//^\\",
        HybridKemVariant::MlKem1024P384 => b"MLKEM1024-P384",
    };
    let mut input = Vec::with_capacity(
        pq_shared_key.len()
            + traditional_shared_key.len()
            + traditional_ciphertext.len()
            + traditional_public_key.len()
            + label.len(),
    );
    input.extend_from_slice(pq_shared_key);
    input.extend_from_slice(traditional_shared_key);
    input.extend_from_slice(traditional_ciphertext);
    input.extend_from_slice(traditional_public_key);
    input.extend_from_slice(label);
    sha3::Sha3_256::digest(input).to_vec()
}

fn traditional_encapsulate(
    variant: HybridKemVariant,
    public_key: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    match variant {
        HybridKemVariant::MlKem768P256 => {
            let recipient = p256::PublicKey::from_sec1_bytes(public_key)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            let ephemeral = p256_private_key(&crate::random_byte_array(128))?;
            let ciphertext = ephemeral.public_key().to_sec1_bytes().to_vec();
            let shared_key = p256::elliptic_curve::ecdh::diffie_hellman(
                ephemeral.to_nonzero_scalar(),
                recipient.as_affine(),
            )
            .raw_secret_bytes()
            .to_vec();
            Ok((ciphertext, shared_key))
        },
        HybridKemVariant::MlKem768X25519 => {
            let recipient = x25519_dalek::PublicKey::from(
                <[u8; 32]>::try_from(public_key).map_err(|_| CryptoError::InvalidKey(None))?,
            );
            let ephemeral = x25519_dalek::StaticSecret::from(
                <[u8; 32]>::try_from(crate::random_byte_array(32))
                    .map_err(|_| CryptoError::OperationFailed(None))?,
            );
            let ciphertext = x25519_dalek::PublicKey::from(&ephemeral)
                .as_bytes()
                .to_vec();
            let shared_key = ephemeral.diffie_hellman(&recipient);
            if shared_key.as_bytes().iter().all(|byte| *byte == 0) {
                return Err(CryptoError::OperationFailed(None));
            }
            Ok((ciphertext, shared_key.as_bytes().to_vec()))
        },
        HybridKemVariant::MlKem1024P384 => {
            let recipient = p384::PublicKey::from_sec1_bytes(public_key)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            let ephemeral = p384_private_key(&crate::random_byte_array(48))?;
            let ciphertext = ephemeral.public_key().to_sec1_bytes().to_vec();
            let shared_key = p384::elliptic_curve::ecdh::diffie_hellman(
                ephemeral.to_nonzero_scalar(),
                recipient.as_affine(),
            )
            .raw_secret_bytes()
            .to_vec();
            Ok((ciphertext, shared_key))
        },
    }
}

fn traditional_decapsulate(
    private_key: &TraditionalPrivateKey,
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    match private_key {
        TraditionalPrivateKey::P256(private_key) => {
            let public_key = p256::PublicKey::from_sec1_bytes(ciphertext)
                .map_err(|_| CryptoError::OperationFailed(None))?;
            Ok(p256::elliptic_curve::ecdh::diffie_hellman(
                private_key.to_nonzero_scalar(),
                public_key.as_affine(),
            )
            .raw_secret_bytes()
            .to_vec())
        },
        TraditionalPrivateKey::X25519(private_key) => {
            let public_key = x25519_dalek::PublicKey::from(
                <[u8; 32]>::try_from(ciphertext).map_err(|_| CryptoError::OperationFailed(None))?,
            );
            let shared_key = private_key.diffie_hellman(&public_key);
            if shared_key.as_bytes().iter().all(|byte| *byte == 0) {
                return Err(CryptoError::OperationFailed(None));
            }
            Ok(shared_key.as_bytes().to_vec())
        },
        TraditionalPrivateKey::P384(private_key) => {
            let public_key = p384::PublicKey::from_sec1_bytes(ciphertext)
                .map_err(|_| CryptoError::OperationFailed(None))?;
            Ok(p384::elliptic_curve::ecdh::diffie_hellman(
                private_key.to_nonzero_scalar(),
                public_key.as_affine(),
            )
            .raw_secret_bytes()
            .to_vec())
        },
    }
}

pub(crate) fn generate_hybrid_kem_key(
    variant: HybridKemVariant,
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let seed = crate::random_byte_array(32);
    let public_key = derive_hybrid_key_pair(variant, &seed)?.public_key;
    Ok((seed, public_key))
}

pub(crate) fn hybrid_kem_public_key(
    variant: HybridKemVariant,
    seed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    Ok(derive_hybrid_key_pair(variant, seed)?.public_key)
}

pub(crate) fn import_hybrid_kem_public_key(
    variant: HybridKemVariant,
    data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if data.len() != variant.public_key_length() {
        return Err(CryptoError::InvalidKey(None));
    }
    let (pq_public_key, traditional_public_key) = data.split_at(variant.pq_public_key_length());
    let pq_public_key = import_ml_kem_public_key(variant.ml_kem_variant(), pq_public_key, false)?;
    match variant {
        HybridKemVariant::MlKem768P256 => {
            p256::PublicKey::from_sec1_bytes(traditional_public_key)
                .map_err(|_| CryptoError::InvalidKey(None))?;
        },
        HybridKemVariant::MlKem768X25519 => {
            <[u8; 32]>::try_from(traditional_public_key)
                .map_err(|_| CryptoError::InvalidKey(None))?;
        },
        HybridKemVariant::MlKem1024P384 => {
            p384::PublicKey::from_sec1_bytes(traditional_public_key)
                .map_err(|_| CryptoError::InvalidKey(None))?;
        },
    }
    let mut normalized = pq_public_key;
    normalized.extend_from_slice(traditional_public_key);
    Ok(normalized)
}

pub(crate) fn import_hybrid_kem_private_key(
    variant: HybridKemVariant,
    data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    derive_hybrid_key_pair(variant, data)?;
    Ok(data.to_vec())
}

pub(crate) fn hybrid_kem_encapsulate(
    variant: HybridKemVariant,
    public_key: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let public_key = import_hybrid_kem_public_key(variant, public_key)?;
    let (pq_public_key, traditional_public_key) =
        public_key.split_at(variant.pq_public_key_length());
    let (pq_ciphertext, pq_shared_key) =
        ml_kem_encapsulate(variant.ml_kem_variant(), pq_public_key)?;
    let (traditional_ciphertext, traditional_shared_key) =
        traditional_encapsulate(variant, traditional_public_key)?;
    let shared_key = hybrid_kem_combiner(
        variant,
        &pq_shared_key,
        &traditional_shared_key,
        &traditional_ciphertext,
        traditional_public_key,
    );
    let mut ciphertext = pq_ciphertext;
    ciphertext.extend_from_slice(&traditional_ciphertext);
    Ok((ciphertext, shared_key))
}

pub(crate) fn hybrid_kem_decapsulate(
    variant: HybridKemVariant,
    seed: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() != variant.ciphertext_length() {
        return Err(CryptoError::OperationFailed(Some(
            "Invalid hybrid KEM ciphertext".into(),
        )));
    }
    let key_pair = derive_hybrid_key_pair(variant, seed)?;
    let (pq_ciphertext, traditional_ciphertext) =
        ciphertext.split_at(variant.pq_ciphertext_length());
    let pq_shared_key =
        ml_kem_decapsulate(variant.ml_kem_variant(), &key_pair.pq_seed, pq_ciphertext)?;
    let traditional_shared_key =
        traditional_decapsulate(&key_pair.traditional_private_key, traditional_ciphertext)?;
    Ok(hybrid_kem_combiner(
        variant,
        &pq_shared_key,
        &traditional_shared_key,
        traditional_ciphertext,
        &key_pair.traditional_public_key,
    ))
}
