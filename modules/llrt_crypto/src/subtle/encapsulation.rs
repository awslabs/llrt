// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;

use llrt_utils::bytes::ObjectBytes;
use rquickjs::{Array, ArrayBuffer, Class, Ctx, Object, Result, Value};

use crate::provider::{modern, HybridKemVariant, MlKemVariant};

use super::import_key::import_key;
use super::{
    algorithm_invalid_access_error, algorithm_not_supported_error,
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::{KeyAlgorithm, KeyFormatData},
    normalize_algorithm_name, to_name_and_maybe_object,
    util::ResultDomExt,
};

#[derive(Clone, Copy)]
pub(super) enum EncapsulationAlgorithm {
    MlKem(MlKemVariant),
    HybridKem(HybridKemVariant),
}

impl EncapsulationAlgorithm {
    fn name(self) -> &'static str {
        match self {
            Self::MlKem(variant) => variant.as_str(),
            Self::HybridKem(variant) => variant.as_str(),
        }
    }

    pub(super) const fn shared_key_length(self) -> usize {
        match self {
            Self::MlKem(_) | Self::HybridKem(_) => 32,
        }
    }

    fn encapsulate(
        self,
        public_key: &[u8],
    ) -> std::result::Result<(Vec<u8>, Vec<u8>), crate::provider::CryptoError> {
        match self {
            Self::MlKem(variant) => modern::ml_kem_encapsulate(variant, public_key),
            Self::HybridKem(variant) => modern::hybrid_kem_encapsulate(variant, public_key),
        }
    }

    fn decapsulate(
        self,
        private_key: &[u8],
        ciphertext: &[u8],
    ) -> std::result::Result<Vec<u8>, crate::provider::CryptoError> {
        match self {
            Self::MlKem(variant) => modern::ml_kem_decapsulate(variant, private_key, ciphertext),
            Self::HybridKem(variant) => {
                modern::hybrid_kem_decapsulate(variant, private_key, ciphertext)
            },
        }
    }
}

pub(super) fn normalize_encapsulation_algorithm<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
) -> Result<EncapsulationAlgorithm> {
    let (name, _) = to_name_and_maybe_object(ctx, value)?;
    let name = normalize_algorithm_name(&name);
    if let Ok(variant) = MlKemVariant::try_from(name.as_str()) {
        Ok(EncapsulationAlgorithm::MlKem(variant))
    } else if let Ok(variant) = HybridKemVariant::try_from(name.as_str()) {
        Ok(EncapsulationAlgorithm::HybridKem(variant))
    } else {
        algorithm_not_supported_error(ctx)
    }
}

fn import_shared_key<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    usages: Array<'js>,
    data: Vec<u8>,
) -> Result<Class<'js, CryptoKey<'js>>> {
    import_key(
        ctx.clone(),
        KeyFormatData::RawSecret(ObjectBytes::Vec(data)),
        algorithm,
        extractable,
        usages,
    )
}

fn check_encapsulation_key(
    ctx: &Ctx<'_>,
    key: &CryptoKey<'_>,
    algorithm: EncapsulationAlgorithm,
    usage: &str,
    kind: KeyKind,
) -> Result<()> {
    let matches_algorithm = match (algorithm, &key.algorithm) {
        (EncapsulationAlgorithm::MlKem(a), KeyAlgorithm::MlKem(b)) => a == *b,
        (EncapsulationAlgorithm::HybridKem(a), KeyAlgorithm::HybridKem(b)) => a == *b,
        _ => false,
    };
    if key.name.as_ref() != algorithm.name() || !matches_algorithm {
        return algorithm_invalid_access_error(ctx, algorithm.name());
    }
    key.check_validity(usage).or_throw_dom(ctx)?;
    key.check_kind(kind).or_throw_dom(ctx)
}

pub fn subtle_encapsulate_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
) -> impl Future<Output = Result<Object<'js>>> + 'js {
    let prepared = normalize_encapsulation_algorithm(&ctx, algorithm);

    async move {
        let algorithm = prepared?;
        let (ciphertext, shared_key) = {
            let key = key.borrow();
            check_encapsulation_key(&ctx, &key, algorithm, "encapsulateBits", KeyKind::Public)?;
            algorithm.encapsulate(&key.handle).or_throw_dom(&ctx)?
        };

        let result = Object::new(ctx.clone())?;
        result.set("sharedKey", ArrayBuffer::new(ctx.clone(), shared_key)?)?;
        result.set("ciphertext", ArrayBuffer::new(ctx, ciphertext)?)?;
        Ok(result)
    }
}

pub fn subtle_decapsulate_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    ciphertext: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    let prepared: Result<_> = (|| {
        let algorithm = normalize_encapsulation_algorithm(&ctx, algorithm)?;
        let ciphertext = ciphertext
            .as_bytes_opt()
            .map(<[u8]>::to_vec)
            .unwrap_or_default();
        Ok((algorithm, ciphertext))
    })();

    async move {
        let (algorithm, ciphertext) = prepared?;
        let shared_key = {
            let key = key.borrow();
            check_encapsulation_key(&ctx, &key, algorithm, "decapsulateBits", KeyKind::Private)?;
            algorithm
                .decapsulate(&key.handle, &ciphertext)
                .or_throw_dom(&ctx)?
        };
        ArrayBuffer::new(ctx, shared_key)
    }
}

pub fn subtle_encapsulate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    shared_key_algorithm: Value<'js>,
    extractable: bool,
    usages: Array<'js>,
) -> impl Future<Output = Result<Object<'js>>> + 'js {
    let prepared = normalize_encapsulation_algorithm(&ctx, algorithm);

    async move {
        let algorithm = prepared?;
        let (ciphertext, shared_key) = {
            let key = key.borrow();
            check_encapsulation_key(&ctx, &key, algorithm, "encapsulateKey", KeyKind::Public)?;
            algorithm.encapsulate(&key.handle).or_throw_dom(&ctx)?
        };
        let shared_key =
            import_shared_key(&ctx, shared_key_algorithm, extractable, usages, shared_key)?;
        let result = Object::new(ctx.clone())?;
        result.set("sharedKey", shared_key)?;
        result.set("ciphertext", ArrayBuffer::new(ctx, ciphertext)?)?;
        Ok(result)
    }
}

pub fn subtle_decapsulate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    ciphertext: ObjectBytes<'js>,
    shared_key_algorithm: Value<'js>,
    extractable: bool,
    usages: Array<'js>,
) -> impl Future<Output = Result<Class<'js, CryptoKey<'js>>>> + 'js {
    let prepared: Result<_> = (|| {
        let algorithm = normalize_encapsulation_algorithm(&ctx, algorithm)?;
        let ciphertext = ciphertext
            .as_bytes_opt()
            .map(<[u8]>::to_vec)
            .unwrap_or_default();
        Ok((algorithm, ciphertext))
    })();

    async move {
        let (algorithm, ciphertext) = prepared?;
        let shared_key = {
            let key = key.borrow();
            check_encapsulation_key(&ctx, &key, algorithm, "decapsulateKey", KeyKind::Private)?;
            algorithm
                .decapsulate(&key.handle, &ciphertext)
                .or_throw_dom(&ctx)?
        };
        import_shared_key(&ctx, shared_key_algorithm, extractable, usages, shared_key)
    }
}
