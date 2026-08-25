// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_exceptions::DOMException;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Array, Class, Ctx, FromJs, Result, Value};

use super::{
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::{
        KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyFormat, KeyFormatData,
    },
};

pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormat,
    key_data: Value<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey<'js>>> {
    validate_import_algorithm(
        &ctx,
        &format,
        algorithm.clone(),
        extractable,
        key_usages.clone(),
    )?;
    let format = match format {
        KeyFormat::Raw => KeyFormatData::Raw(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Pkcs8 => KeyFormatData::Pkcs8(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Spki => KeyFormatData::Spki(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Jwk => KeyFormatData::Jwk(key_data.into_object_or_throw(&ctx, "keyData")?),
    };

    import_key(ctx, format, algorithm, extractable, key_usages)
}

pub fn import_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormatData<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey<'js>>> {
    let jwk = match &format {
        KeyFormatData::Jwk(value) => Some(value.clone()),
        _ => None,
    };

    let mut kind = KeyKind::Public;
    let mut data = Vec::new();

    let KeyAlgorithmWithUsages {
        name,
        algorithm: key_algorithm,
        public_usages,
        private_usages,
    } = KeyAlgorithm::from_js(
        &ctx,
        KeyAlgorithmMode::Import {
            kind: &mut kind,
            data: &mut data,
            format,
        },
        algorithm,
        key_usages,
    )?;

    let usages = match kind {
        KeyKind::Public | KeyKind::Secret => public_usages,
        KeyKind::Private => private_usages,
    };
    if let Some(jwk) = jwk {
        if let Some(key_ops) = parse_jwk_key_ops(&ctx, &jwk)? {
            validate_requested_jwk_key_ops(&ctx, &key_ops, &usages)?;
        }
        validate_jwk_extractable(&ctx, &jwk, extractable)?;
    }

    Class::instance(
        ctx,
        CryptoKey::new(kind, name, extractable, key_algorithm, usages, data),
    )
}

fn validate_import_algorithm<'js>(
    ctx: &Ctx<'js>,
    format: &KeyFormat,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<()> {
    let normalized =
        KeyAlgorithm::from_js(ctx, KeyAlgorithmMode::ValidateImport, algorithm, key_usages)?;
    if matches!(
        normalized.algorithm,
        super::key_algorithm::KeyAlgorithm::HkdfImport
            | super::key_algorithm::KeyAlgorithm::Pbkdf2Import
    ) {
        if !matches!(format, KeyFormat::Raw) {
            return Err(DOMException::not_supported_error(
                ctx,
                [
                    normalized.name.as_str(),
                    " only supports 'raw' import format",
                ]
                .concat(),
            ));
        }
        if extractable {
            return Err(DOMException::syntax_error(
                ctx,
                format!("{} keys must not be extractable", normalized.name),
            ));
        }
    }
    Ok(())
}

fn validate_jwk_extractable(
    ctx: &Ctx<'_>,
    jwk: &rquickjs::Object<'_>,
    extractable: bool,
) -> Result<()> {
    if extractable && matches!(jwk.get_optional::<_, bool>("ext")?, Some(false)) {
        return Err(DOMException::data_error(ctx, "JWK is not extractable"));
    }
    Ok(())
}

fn parse_jwk_key_ops(ctx: &Ctx<'_>, jwk: &rquickjs::Object<'_>) -> Result<Option<Vec<String>>> {
    let Some(key_ops) = jwk.get_optional::<_, Array>("key_ops")? else {
        return Ok(None);
    };
    let mut operations = Vec::with_capacity(key_ops.len());
    for operation in key_ops.iter::<String>() {
        let operation = operation?;
        if operations.contains(&operation) {
            return Err(DOMException::data_error(
                ctx,
                "JWK 'key_ops' contains a duplicate operation",
            ));
        }
        operations.push(operation);
    }
    Ok(Some(operations))
}

fn validate_requested_jwk_key_ops(
    ctx: &Ctx<'_>,
    operations: &[String],
    requested_usages: &[String],
) -> Result<()> {
    if requested_usages
        .iter()
        .any(|usage| !operations.contains(usage))
    {
        return Err(DOMException::data_error(
            ctx,
            "JWK 'key_ops' does not contain all requested usages",
        ));
    }
    Ok(())
}
