// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_buffer::Buffer;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{
    class::Trace, function::Opt, prelude::This, Class, Ctx, IntoJs, JsLifetime, Result, Value,
};

use super::encoded_bytes;
use crate::provider::{CryptoProvider, HmacProvider, SimpleDigest};
use crate::CRYPTO_PROVIDER;

#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl TryFrom<&str> for HashAlgorithm {
    type Error = String;
    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match s.to_ascii_uppercase().as_str() {
            "MD5" => HashAlgorithm::Md5,
            "MD-5" => HashAlgorithm::Md5,
            "SHA1" => HashAlgorithm::Sha1,
            "SHA-1" => HashAlgorithm::Sha1,
            "SHA256" => HashAlgorithm::Sha256,
            "SHA-256" => HashAlgorithm::Sha256,
            "SHA384" => HashAlgorithm::Sha384,
            "SHA-384" => HashAlgorithm::Sha384,
            "SHA512" => HashAlgorithm::Sha512,
            "SHA-512" => HashAlgorithm::Sha512,
            _ => return Err(["'", s, "' not available"].concat()),
        })
    }
}

impl HashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha384 => "SHA-384",
            HashAlgorithm::Sha512 => "SHA-512",
        }
    }

    pub fn as_numeric_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "md5",
            HashAlgorithm::Sha1 => "1",
            HashAlgorithm::Sha256 => "256",
            HashAlgorithm::Sha384 => "384",
            HashAlgorithm::Sha512 => "512",
        }
    }

    pub fn digest_len(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 16,
            HashAlgorithm::Sha1 => 20,
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha384 => 48,
            HashAlgorithm::Sha512 => 64,
        }
    }

    pub fn block_len(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 64,
            HashAlgorithm::Sha1 => 64,
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha384 => 128,
            HashAlgorithm::Sha512 => 128,
        }
    }
}

type ProviderDigest = <crate::provider::DefaultProvider as CryptoProvider>::Digest;
type ProviderHmac = <crate::provider::DefaultProvider as CryptoProvider>::Hmac;

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct Hash {
    #[qjs(skip_trace)]
    inner: Option<ProviderDigest>,
}

impl Hash {
    pub fn new(ctx: Ctx<'_>, algorithm: String) -> Result<Self> {
        let algorithm = HashAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        Ok(Self {
            inner: Some(CRYPTO_PROVIDER.digest(algorithm)),
        })
    }
}

#[rquickjs::methods]
impl Hash {
    #[qjs(rename = "digest")]
    fn hash_digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let digest = self
            .inner
            .take()
            .ok_or_else(|| rquickjs::Exception::throw_message(&ctx, "Digest already called"))?;
        let result = digest.finalize();

        let Some(encoding) = encoding.0 else {
            return Buffer(result).into_js(&ctx);
        };

        match encoded_bytes(&ctx, &result, &encoding)? {
            Some(encoded) => Ok(encoded),
            None => Buffer(result).into_js(&ctx),
        }
    }

    #[qjs(rename = "update")]
    fn hash_update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = bytes.as_bytes(&ctx)?;
        let mut borrowed = this.0.borrow_mut();
        if let Some(ref mut digest) = borrowed.inner {
            digest.update(bytes);
        }
        drop(borrowed);
        Ok(this.0)
    }
}

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct Hmac {
    #[qjs(skip_trace)]
    inner: Option<ProviderHmac>,
}

impl Hmac {
    pub fn new<'js>(ctx: Ctx<'js>, algorithm: String, key_value: ObjectBytes<'js>) -> Result<Self> {
        let algorithm = HashAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        let key = key_value.as_bytes(&ctx)?;
        let hmac = CRYPTO_PROVIDER.hmac(algorithm, key);
        Ok(Self { inner: Some(hmac) })
    }
}

#[rquickjs::methods]
impl Hmac {
    fn digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let hmac = self
            .inner
            .take()
            .ok_or_else(|| rquickjs::Exception::throw_message(&ctx, "Digest already called"))?;
        let result = hmac.finalize();

        let Some(encoding) = encoding.0 else {
            return Buffer(result).into_js(&ctx);
        };

        match encoded_bytes(&ctx, &result, &encoding)? {
            Some(encoded) => Ok(encoded),
            None => Buffer(result).into_js(&ctx),
        }
    }

    fn update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = bytes.as_bytes(&ctx)?;
        let mut borrowed = this.0.borrow_mut();
        if let Some(ref mut hmac) = borrowed.inner {
            hmac.update(bytes);
        }
        drop(borrowed);
        Ok(this.0)
    }
}
