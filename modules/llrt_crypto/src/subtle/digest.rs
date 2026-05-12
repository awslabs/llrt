// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Result, Value};

use crate::{
    hash::HashAlgorithm,
    provider::{CryptoProvider, SimpleDigest},
    CRYPTO_PROVIDER,
};

use super::algorithm_not_supported_error;

pub fn subtle_digest<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> impl std::future::Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    // Snapshot the algorithm name + buffer bytes synchronously (before the
    // returned future is polled), so modifying the ArrayBufferView after
    // the call can't affect the result. Per WebCryptoAPI §24, `digest` takes
    // its input by copy at call time (WPT `digest.https.any.js` "altered
    // buffer after call").
    let prepared: Result<(HashAlgorithm, Vec<u8>)> = (|| {
        let algorithm = if let Some(s) = algorithm.as_string() {
            s.to_string().or_throw(&ctx)?
        } else if let Some(name) = algorithm.get_optional::<_, String>("name")? {
            name
        } else {
            return Err(rquickjs::Exception::throw_type(
                &ctx,
                "Algorithm 'name' property required",
            ));
        };
        let hash_algorithm = match HashAlgorithm::try_from(algorithm.as_str()) {
            Ok(h) => h,
            Err(_) => return algorithm_not_supported_error(&ctx),
        };
        // A detached BufferSource has no bytes — treat as empty input (WPT
        // `digest.https.any.js` "transferred buffer during call").
        let input: Vec<u8> = data.as_bytes_opt().map(<[u8]>::to_vec).unwrap_or_default();
        Ok((hash_algorithm, input))
    })();

    async move {
        let (hash_algorithm, input) = prepared?;
        let bytes = digest(&hash_algorithm, &input);
        ArrayBuffer::new(ctx, bytes)
    }
}

pub fn digest(hash_algorithm: &HashAlgorithm, data: &[u8]) -> Vec<u8> {
    let mut hasher = CRYPTO_PROVIDER.digest(*hash_algorithm);
    hasher.update(data);
    hasher.finalize()
}
