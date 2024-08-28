// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::bytes::{bytes_to_typed_array, ObjectBytes};
use ring::{
    digest::{self, Context as DigestContext},
    hmac::{self, Context as HmacContext},
};
use rquickjs::{function::Opt, prelude::This, Class, Ctx, Exception, Result, Value};

use super::encoded_bytes;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Hmac {
    #[qjs(skip_trace)]
    context: HmacContext,
}

#[rquickjs::methods]
impl Hmac {
    #[qjs(skip)]
    pub fn new<'js>(ctx: Ctx<'js>, algorithm: String, key_value: ObjectBytes<'js>) -> Result<Self> {
        let algorithm = match algorithm.to_lowercase().as_str() {
            "sha1" => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            "sha256" => hmac::HMAC_SHA256,
            "sha384" => hmac::HMAC_SHA384,
            "sha512" => hmac::HMAC_SHA512,
            _ => {
                return Err(Exception::throw_message(
                    &ctx,
                    &["Algorithm \"", &algorithm, "\" not supported"].concat(),
                ))
            },
        };

        Ok(Self {
            context: HmacContext::with_key(&hmac::Key::new(algorithm, key_value.as_bytes())),
        })
    }

    fn digest<'js>(&self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let signature = self.context.clone().sign();
        let bytes: &[u8] = signature.as_ref();

        match encoding.into_inner() {
            Some(encoding) => encoded_bytes(ctx, bytes, &encoding),
            None => bytes_to_typed_array(ctx, bytes),
        }
    }

    fn update<'js>(
        this: This<Class<'js, Self>>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = bytes.as_bytes();
        this.0.borrow_mut().context.update(bytes);

        Ok(this.0)
    }
}

impl Clone for Hmac {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Hash {
    #[qjs(skip_trace)]
    context: DigestContext,
}

#[rquickjs::methods]
impl Hash {
    #[qjs(skip)]
    pub fn new(ctx: Ctx<'_>, algorithm: String) -> Result<Self> {
        let algorithm = match algorithm.to_lowercase().as_str() {
            "sha1" => &digest::SHA1_FOR_LEGACY_USE_ONLY,
            "sha256" => &digest::SHA256,
            "sha384" => &digest::SHA384,
            "sha512" => &digest::SHA512,
            _ => {
                return Err(Exception::throw_message(
                    &ctx,
                    &["Algorithm \"", &algorithm, "\" not supported"].concat(),
                ))
            },
        };

        Ok(Self {
            context: DigestContext::new(algorithm),
        })
    }

    #[qjs(rename = "digest")]
    fn hash_digest<'js>(&self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let digest = self.context.clone().finish();
        let bytes: &[u8] = digest.as_ref();

        match encoding.0 {
            Some(encoding) => encoded_bytes(ctx, bytes, &encoding),
            None => bytes_to_typed_array(ctx, bytes),
        }
    }

    #[qjs(rename = "update")]
    fn hash_update<'js>(
        this: This<Class<'js, Self>>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = bytes.as_bytes();
        this.0.borrow_mut().context.update(bytes);
        Ok(this.0)
    }
}

iterable_enum!(pub, ShaAlgorithm, SHA1, SHA256, SHA384, SHA512);

impl ShaAlgorithm {
    pub fn class_name(&self) -> &'static str {
        match self {
            ShaAlgorithm::SHA1 => "Sha1",
            ShaAlgorithm::SHA256 => "Sha256",
            ShaAlgorithm::SHA384 => "Sha384",
            ShaAlgorithm::SHA512 => "Sha512",
        }
    }
    pub fn hmac_algorithm(&self) -> &'static hmac::Algorithm {
        match self {
            ShaAlgorithm::SHA1 => &hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            ShaAlgorithm::SHA256 => &hmac::HMAC_SHA256,
            ShaAlgorithm::SHA384 => &hmac::HMAC_SHA384,
            ShaAlgorithm::SHA512 => &hmac::HMAC_SHA512,
        }
    }

    fn digest_algorithm(&self) -> &'static digest::Algorithm {
        match self {
            ShaAlgorithm::SHA1 => &digest::SHA1_FOR_LEGACY_USE_ONLY,
            ShaAlgorithm::SHA256 => &digest::SHA256,
            ShaAlgorithm::SHA384 => &digest::SHA384,
            ShaAlgorithm::SHA512 => &digest::SHA512,
        }
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct ShaHash {
    #[qjs(skip_trace)]
    secret: Option<Vec<u8>>,
    #[qjs(skip_trace)]
    bytes: Vec<u8>,
    #[qjs(skip_trace)]
    algorithm: ShaAlgorithm,
}

#[rquickjs::methods]
impl ShaHash {
    #[qjs(skip)]
    pub fn new(algorithm: ShaAlgorithm, secret: Opt<ObjectBytes<'_>>) -> Result<Self> {
        let secret = secret.0.map(|bytes| bytes.into_bytes());

        Ok(ShaHash {
            secret,
            bytes: Vec::new(),
            algorithm,
        })
    }

    #[qjs(rename = "digest")]
    fn sha_digest<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(secret) = &self.secret {
            let key_value = secret;
            let key = hmac::Key::new(*self.algorithm.hmac_algorithm(), key_value);

            return bytes_to_typed_array(ctx, hmac::sign(&key, &self.bytes).as_ref());
        }

        bytes_to_typed_array(
            ctx,
            digest::digest(self.algorithm.digest_algorithm(), &self.bytes).as_ref(),
        )
    }

    #[qjs(rename = "update")]
    fn sha_update<'js>(
        this: This<Class<'js, Self>>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        this.0.borrow_mut().bytes = bytes.into();
        Ok(this.0)
    }
}
