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

use super::{CryptoError, MlDsaVariant, MlKemVariant};

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
