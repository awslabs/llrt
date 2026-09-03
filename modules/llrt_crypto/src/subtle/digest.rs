use std::future::Future;

// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{ArrayBuffer, Ctx, Exception, Object, Result, Value};
use sha3::{Digest, Sha3_256, Sha3_384, Sha3_512};

use crate::{
    hash::HashAlgorithm,
    provider::{CryptoProvider, SimpleDigest},
    CRYPTO_PROVIDER,
};

use super::{
    algorithm_not_supported_error, enforce_range_u32, enforce_range_u8,
    get_optional_dictionary_value, get_required_dictionary_value, to_name_and_maybe_object,
};

enum DigestAlgorithmName {
    Fixed(HashAlgorithm),
    Sha3_256,
    Sha3_384,
    Sha3_512,
    CShake(u16),
    TurboShake(u16),
}

impl TryFrom<&str> for DigestAlgorithmName {
    type Error = ();

    fn try_from(name: &str) -> std::result::Result<Self, Self::Error> {
        let name = name.to_ascii_uppercase();
        Ok(match name.as_str() {
            "CSHAKE128" => Self::CShake(128),
            "CSHAKE256" => Self::CShake(256),
            "TURBOSHAKE128" => Self::TurboShake(128),
            "TURBOSHAKE256" => Self::TurboShake(256),
            "SHA3-256" => Self::Sha3_256,
            "SHA3-384" => Self::Sha3_384,
            "SHA3-512" => Self::Sha3_512,
            _ => Self::Fixed(HashAlgorithm::from_strict_str(&name).map_err(|_| ())?),
        })
    }
}

enum DigestAlgorithm {
    Fixed(HashAlgorithm),
    Sha3_256,
    Sha3_384,
    Sha3_512,
    CShake {
        strength: u16,
        output_length: u32,
        function_name: Vec<u8>,
        customization: Vec<u8>,
    },
    TurboShake {
        strength: u16,
        output_length: u32,
        domain_separation: u8,
    },
}

pub fn subtle_digest<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    // Snapshot inputs synchronously so mutating/detaching the buffer after the call can't affect the result (WPT digest.https.any.js).
    let prepared = prepare_digest(&ctx, algorithm, data);

    async move {
        let (algorithm, input) = prepared?;
        let bytes = match algorithm {
            DigestAlgorithm::Fixed(hash) => digest(&hash, &input),
            DigestAlgorithm::Sha3_256 => Sha3_256::digest(&input).to_vec(),
            DigestAlgorithm::Sha3_384 => Sha3_384::digest(&input).to_vec(),
            DigestAlgorithm::Sha3_512 => Sha3_512::digest(&input).to_vec(),
            DigestAlgorithm::CShake {
                strength,
                output_length,
                function_name,
                customization,
            } => cshake_digest(
                &ctx,
                strength,
                output_length,
                &function_name,
                &customization,
                &input,
            )?,
            DigestAlgorithm::TurboShake {
                strength,
                output_length,
                domain_separation,
            } => turbo_shake_digest(&ctx, strength, output_length, domain_separation, &input)?,
        };
        ArrayBuffer::new(ctx, bytes)
    }
}

fn prepare_digest<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> Result<(DigestAlgorithm, Vec<u8>)> {
    let (name, object) = to_name_and_maybe_object(ctx, algorithm)?;
    let Ok(name) = DigestAlgorithmName::try_from(name.as_str()) else {
        return algorithm_not_supported_error(ctx);
    };
    let algorithm = match name {
        DigestAlgorithmName::CShake(strength) => {
            let object = object?;
            let value = get_required_dictionary_value(&object, "outputLength", "algorithm")?;
            let output_length = enforce_range_u32(ctx, value, "outputLength")?;
            if u64::from(output_length).div_ceil(8) * 8 > u64::from(u32::MAX) {
                return Err(llrt_exceptions::DOMException::operation_error(
                    ctx,
                    "Invalid cSHAKE outputLength",
                ));
            }
            let function_name = optional_bytes(ctx, &object, "functionName")?;
            let customization = optional_bytes(ctx, &object, "customization")?;
            DigestAlgorithm::CShake {
                strength,
                output_length,
                function_name,
                customization,
            }
        },
        DigestAlgorithmName::TurboShake(strength) => {
            let object = object?;
            let value = get_required_dictionary_value(&object, "outputLength", "algorithm")?;
            let output_length = enforce_range_u32(ctx, value, "outputLength")?;
            if output_length == 0 || !output_length.is_multiple_of(8) {
                return Err(llrt_exceptions::DOMException::operation_error(
                    ctx,
                    "TurboSHAKE outputLength must be a positive multiple of 8",
                ));
            }
            let domain_separation = get_optional_dictionary_value(&object, "domainSeparation")?
                .map(|value| enforce_range_u8(ctx, value, "domainSeparation"))
                .transpose()?
                .unwrap_or(0x1f);
            if !(0x01..=0x7f).contains(&domain_separation) {
                return Err(llrt_exceptions::DOMException::operation_error(
                    ctx,
                    "Invalid TurboSHAKE domainSeparation",
                ));
            }
            DigestAlgorithm::TurboShake {
                strength,
                output_length,
                domain_separation,
            }
        },
        DigestAlgorithmName::Sha3_256 => DigestAlgorithm::Sha3_256,
        DigestAlgorithmName::Sha3_384 => DigestAlgorithm::Sha3_384,
        DigestAlgorithmName::Sha3_512 => DigestAlgorithm::Sha3_512,
        DigestAlgorithmName::Fixed(hash) => DigestAlgorithm::Fixed(hash),
    };
    let input = data.as_bytes_opt().map(<[u8]>::to_vec).unwrap_or_default();
    Ok((algorithm, input))
}

pub(super) fn supports_digest_algorithm<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
) -> Result<bool> {
    prepare_digest(ctx, algorithm, ObjectBytes::Vec(Vec::new()))?;
    Ok(true)
}

fn optional_bytes<'js>(ctx: &Ctx<'js>, object: &Object<'js>, name: &str) -> Result<Vec<u8>> {
    let value = object.get::<_, Value>(name)?;
    if value.is_undefined() {
        return Ok(Vec::new());
    }
    if value.is_null() {
        return Err(Exception::throw_type(
            ctx,
            &[name, " must be a BufferSource"].concat(),
        ));
    }
    ObjectBytes::from(ctx, &value)?.into_bytes(ctx)
}

fn try_allocate_digest_output(
    length: usize,
) -> std::result::Result<Vec<u8>, std::collections::TryReserveError> {
    let mut output = Vec::new();
    output.try_reserve_exact(length)?;
    output.resize(length, 0);
    Ok(output)
}

fn allocate_digest_output(ctx: &Ctx<'_>, length: usize) -> Result<Vec<u8>> {
    try_allocate_digest_output(length).map_err(|_| {
        llrt_exceptions::DOMException::operation_error(ctx, "Digest output allocation failed")
    })
}

fn cshake_digest(
    ctx: &Ctx<'_>,
    strength: u16,
    output_length: u32,
    function_name: &[u8],
    customization: &[u8],
    input: &[u8],
) -> Result<Vec<u8>> {
    use cshake::digest::{ExtendableOutput, Update, XofReader};

    let mut output = allocate_digest_output(ctx, (output_length as usize).div_ceil(8))?;
    if output.is_empty() {
        return Ok(output);
    }
    if strength == 128 {
        let mut hasher = cshake::CShake128::new_with_function_name(function_name, customization);
        hasher.update(input);
        hasher.finalize_xof().read(&mut output);
    } else {
        let mut hasher = cshake::CShake256::new_with_function_name(function_name, customization);
        hasher.update(input);
        hasher.finalize_xof().read(&mut output);
    }
    if !output_length.is_multiple_of(8) {
        let mask = u8::MAX << (8 - output_length % 8);
        *output.last_mut().unwrap() &= mask;
    }
    Ok(output)
}

fn turbo_shake_digest(
    ctx: &Ctx<'_>,
    strength: u16,
    output_length: u32,
    domain_separation: u8,
    input: &[u8],
) -> Result<Vec<u8>> {
    if strength == 128 {
        turbo_shake::<168>(ctx, domain_separation, input, output_length as usize / 8)
    } else {
        turbo_shake::<136>(ctx, domain_separation, input, output_length as usize / 8)
    }
}

fn turbo_shake<const RATE: usize>(
    ctx: &Ctx<'_>,
    domain: u8,
    input: &[u8],
    length: usize,
) -> Result<Vec<u8>> {
    use keccak::{Keccak, State1600};
    use sponge_cursor::SpongeCursor;

    let keccak = Keccak::new();
    let mut state = State1600::default();
    let mut cursor: SpongeCursor<RATE> = Default::default();
    keccak.with_p1600::<12>(|permutation| {
        cursor.absorb_u64_le(&mut state, permutation, input);
        let position = cursor.pos();
        state[position / 8] ^= u64::from(domain) << (8 * (position % 8));
        state[RATE / 8 - 1] ^= 1 << 63;
    });

    let mut output = allocate_digest_output(ctx, length)?;
    let mut reader: SpongeCursor<RATE> = Default::default();
    keccak.with_p1600::<12>(|permutation| {
        reader.squeeze_read_u64_le(&mut state, permutation, &mut output);
    });
    Ok(output)
}

pub fn digest(hash_algorithm: &HashAlgorithm, data: &[u8]) -> Vec<u8> {
    let mut hasher = CRYPTO_PROVIDER.digest(*hash_algorithm);
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::try_allocate_digest_output;

    #[test]
    fn digest_output_allocation_is_fallible() {
        assert!(try_allocate_digest_output(usize::MAX).is_err());
    }
}
