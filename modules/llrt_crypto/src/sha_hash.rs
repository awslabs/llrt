// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    bytes::{bytes_to_typed_array, ObjectBytes},
    iterable_enum,
    result::ResultExt,
};
use rquickjs::{function::Opt, prelude::This, Class, Ctx, Exception, Result, Value};

use super::{encoded_bytes, CRYPTO_PROVIDER};
use crate::provider::{CryptoProvider, DefaultProvider, HmacProvider, SimpleDigest};

type ProviderHmac = <DefaultProvider as CryptoProvider>::Hmac;
type ProviderDigest = <DefaultProvider as CryptoProvider>::Digest;

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct Hmac {
    #[qjs(skip_trace)]
    inner: Option<ProviderHmac>,
}

#[rquickjs::methods]
impl Hmac {
    #[qjs(skip)]
    pub fn new<'js>(ctx: Ctx<'js>, algorithm: String, key_value: ObjectBytes<'js>) -> Result<Self> {
        let algorithm = ShaAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        let key = key_value.as_bytes(&ctx)?;
        let hmac = CRYPTO_PROVIDER.hmac(algorithm, key);

        Ok(Self { inner: Some(hmac) })
    }

    fn digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let hmac = self
            .inner
            .take()
            .ok_or_else(|| Exception::throw_message(&ctx, "Digest already called"))?;
        let result = hmac.finalize();

        match encoding.into_inner() {
            Some(encoding) => encoded_bytes(ctx, &result, &encoding),
            None => bytes_to_typed_array(ctx, &result),
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

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct Hash {
    #[qjs(skip_trace)]
    inner: Option<ProviderDigest>,
}

#[rquickjs::methods]
impl Hash {
    #[qjs(skip)]
    pub fn new(ctx: Ctx<'_>, algorithm: String) -> Result<Self> {
        let algorithm = ShaAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        let digest = CRYPTO_PROVIDER.digest(algorithm);

        Ok(Self {
            inner: Some(digest),
        })
    }

    #[qjs(rename = "digest")]
    fn hash_digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let digest = self
            .inner
            .take()
            .ok_or_else(|| Exception::throw_message(&ctx, "Digest already called"))?;
        let result = digest.finalize();

        match encoding.0 {
            Some(encoding) => encoded_bytes(ctx, &result, &encoding),
            None => bytes_to_typed_array(ctx, &result),
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

#[derive(Debug, Clone, Copy)]
pub enum ShaAlgorithm {
    MD5,
    SHA1,
    SHA256,
    SHA384,
    SHA512,
}

iterable_enum!(ShaAlgorithm, MD5, SHA1, SHA256, SHA384, SHA512);

impl ShaAlgorithm {
    pub fn class_name(&self) -> &'static str {
        match self {
            ShaAlgorithm::MD5 => "Md5",
            ShaAlgorithm::SHA1 => "Sha1",
            ShaAlgorithm::SHA256 => "Sha256",
            ShaAlgorithm::SHA384 => "Sha384",
            ShaAlgorithm::SHA512 => "Sha512",
        }
    }

    /// Returns the block size in bytes for this hash algorithm
    pub fn block_len(&self) -> usize {
        match self {
            ShaAlgorithm::MD5 => 64,
            ShaAlgorithm::SHA1 => 64,
            ShaAlgorithm::SHA256 => 64,
            ShaAlgorithm::SHA384 => 128,
            ShaAlgorithm::SHA512 => 128,
        }
    }

    /// Returns the digest/output size in bytes for this hash algorithm
    pub fn digest_len(&self) -> usize {
        match self {
            ShaAlgorithm::MD5 => 16,
            ShaAlgorithm::SHA1 => 20,
            ShaAlgorithm::SHA256 => 32,
            ShaAlgorithm::SHA384 => 48,
            ShaAlgorithm::SHA512 => 64,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ShaAlgorithm::MD5 => "MD5",
            ShaAlgorithm::SHA1 => "SHA-1",
            ShaAlgorithm::SHA256 => "SHA-256",
            ShaAlgorithm::SHA384 => "SHA-384",
            ShaAlgorithm::SHA512 => "SHA-512",
        }
    }

    pub fn as_numeric_str(&self) -> &'static str {
        match self {
            ShaAlgorithm::MD5 => "md5",
            ShaAlgorithm::SHA1 => "1",
            ShaAlgorithm::SHA256 => "256",
            ShaAlgorithm::SHA384 => "384",
            ShaAlgorithm::SHA512 => "512",
        }
    }
}

impl TryFrom<&str> for ShaAlgorithm {
    type Error = String;
    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match s.to_ascii_uppercase().as_str() {
            "SHA1" => ShaAlgorithm::SHA1,
            "SHA-1" => ShaAlgorithm::SHA1,
            "SHA256" => ShaAlgorithm::SHA256,
            "SHA-256" => ShaAlgorithm::SHA256,
            "SHA384" => ShaAlgorithm::SHA384,
            "SHA-384" => ShaAlgorithm::SHA384,
            "SHA512" => ShaAlgorithm::SHA512,
            "SHA-512" => ShaAlgorithm::SHA512,
            _ => return Err(["'", s, "' not available"].concat()),
        })
    }
}

impl AsRef<str> for ShaAlgorithm {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
