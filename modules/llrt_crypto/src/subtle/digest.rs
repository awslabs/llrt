use std::future::Future;

// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Object, Result, Value};
use sha3::{Digest, Sha3_256, Sha3_384, Sha3_512};

use crate::{
    hash::HashAlgorithm,
    provider::{CryptoProvider, SimpleDigest},
    CRYPTO_PROVIDER,
};

use super::{
    algorithm_not_supported_error, enforce_range_u32, enforce_range_u8,
    get_optional_dictionary_value, get_required_dictionary_value,
};

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
    let (name, object) = if let Some(s) = algorithm.as_string() {
        (s.to_string().or_throw(ctx)?, None)
    } else if let Some(object) = algorithm.into_object() {
        let name = object.get_required::<_, String>("name", "algorithm")?;
        (name, Some(object))
    } else {
        return Err(rquickjs::Exception::throw_type(
            ctx,
            "Algorithm 'name' property required",
        ));
    };
    let name = name.to_ascii_uppercase();
    let algorithm = match name.as_str() {
        "CSHAKE128" | "CSHAKE256" => {
            let object = required_object(ctx, object)?;
            let output_length = required_u32(ctx, &object, "outputLength")?;
            if u64::from(output_length).div_ceil(8) * 8 > u64::from(u32::MAX) {
                return Err(llrt_exceptions::DOMException::operation_error(
                    ctx,
                    "Invalid cSHAKE outputLength",
                ));
            }
            let function_name = optional_bytes(ctx, &object, "functionName")?;
            let customization = optional_bytes(ctx, &object, "customization")?;
            DigestAlgorithm::CShake {
                strength: if name == "CSHAKE128" { 128 } else { 256 },
                output_length,
                function_name,
                customization,
            }
        },
        "TURBOSHAKE128" | "TURBOSHAKE256" => {
            let object = required_object(ctx, object)?;
            let output_length = required_u32(ctx, &object, "outputLength")?;
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
                strength: if name == "TURBOSHAKE128" { 128 } else { 256 },
                output_length,
                domain_separation,
            }
        },
        "SHA3-256" => DigestAlgorithm::Sha3_256,
        "SHA3-384" => DigestAlgorithm::Sha3_384,
        "SHA3-512" => DigestAlgorithm::Sha3_512,
        name => match HashAlgorithm::from_strict_str(name) {
            Ok(hash) => DigestAlgorithm::Fixed(hash),
            Err(_) => return algorithm_not_supported_error(ctx),
        },
    };
    let input = data.as_bytes_opt().map(<[u8]>::to_vec).unwrap_or_default();
    Ok((algorithm, input))
}

fn required_object<'js>(ctx: &Ctx<'js>, object: Option<Object<'js>>) -> Result<Object<'js>> {
    object.ok_or_else(|| rquickjs::Exception::throw_type(ctx, "algorithm must be an object"))
}

fn required_u32<'js>(ctx: &Ctx<'js>, object: &Object<'js>, name: &str) -> Result<u32> {
    let value = get_required_dictionary_value(object, name, "algorithm")?;
    enforce_range_u32(ctx, value, name)
}

fn optional_bytes<'js>(ctx: &Ctx<'js>, object: &Object<'js>, name: &str) -> Result<Vec<u8>> {
    let value = object.get::<_, Value>(name)?;
    if value.is_undefined() {
        return Ok(Vec::new());
    }
    if value.is_null() {
        return Err(rquickjs::Exception::throw_type(
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
