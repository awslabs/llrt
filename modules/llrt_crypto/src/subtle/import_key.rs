// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Array, Class, Ctx, Exception, Result, Value};

use crate::subtle::CryptoKey;

use super::key_algorithm::{classify_and_check_usages, KeyAlgorithm, KeyAlgorithmMode};

pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key_data: ObjectBytes<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    if format != "raw" {
        return Err(Exception::throw_type(
            &ctx,
            &["Format '", &format, "' is not implemented"].concat(),
        ));
    };

    let data = key_data.into_bytes();

    if let Some(obj) = algorithm.as_object() {
        let name: String = obj.get_required("name", "algorithm")?;
        if name.starts_with("AES") || name == "HMAC" {
            obj.set("length", data.len())?;
        }
        if name.starts_with("RSA") {
            return Err(Exception::throw_type(
                &ctx,
                "RSA keys are not supported for import yet",
            ));
        }
    }

    let (key_algorithm, name) = KeyAlgorithm::from_js(&ctx, KeyAlgorithmMode::Import, algorithm)?;

    let (_, public_usages) = classify_and_check_usages(&ctx, &name, &key_usages)?;

    Class::instance(
        ctx,
        CryptoKey::new(
            "secret",
            name,
            extractable,
            key_algorithm,
            public_usages,
            data,
        ),
    )
}
