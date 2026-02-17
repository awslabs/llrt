// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_buffer::Buffer;
use llrt_utils::{bytes::ObjectBytes, iterable_enum, result::ResultExt};
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

iterable_enum!(HashAlgorithm, Md5, Sha1, Sha256, Sha384, Sha512);

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
    pub fn class_name(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "Md5",
            HashAlgorithm::Sha1 => "Sha1",
            HashAlgorithm::Sha256 => "Sha256",
            HashAlgorithm::Sha384 => "Sha384",
            HashAlgorithm::Sha512 => "Sha512",
        }
    }

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
    digest: Option<ProviderDigest>,
    #[qjs(skip_trace)]
    hmac: Option<ProviderHmac>,
}

impl Hash {
    pub fn new(ctx: Ctx<'_>, algorithm: String) -> Result<Self> {
        let algorithm = HashAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        Ok(Self {
            digest: Some(CRYPTO_PROVIDER.digest(algorithm)),
            hmac: None,
        })
    }

    pub fn new_hmac<'js>(
        ctx: Ctx<'js>,
        algorithm: String,
        secret: ObjectBytes<'js>,
    ) -> Result<Self> {
        let algorithm = HashAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
        let key = secret.as_bytes(&ctx)?;
        Ok(Self {
            digest: None,
            hmac: Some(CRYPTO_PROVIDER.hmac(algorithm, key)),
        })
    }

    fn do_update(&mut self, data: &[u8]) {
        if let Some(ref mut d) = self.digest {
            d.update(data);
        } else if let Some(ref mut h) = self.hmac {
            h.update(data);
        }
    }

    fn do_finalize(&mut self) -> Option<Vec<u8>> {
        if let Some(d) = self.digest.take() {
            Some(d.finalize())
        } else {
            self.hmac.take().map(|h| h.finalize())
        }
    }
}

#[rquickjs::methods]
impl Hash {
    #[qjs(rename = "digest")]
    fn hash_digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let result = self
            .do_finalize()
            .ok_or_else(|| rquickjs::Exception::throw_message(&ctx, "Digest already called"))?;

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
        this.0.borrow_mut().do_update(bytes);
        Ok(this.0)
    }
}

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct Hmac {
    #[qjs(skip_trace)]
    hash: Hash,
}

impl Hmac {
    pub fn new<'js>(ctx: Ctx<'js>, algorithm: String, key_value: ObjectBytes<'js>) -> Result<Self> {
        Ok(Self {
            hash: Hash::new_hmac(ctx, algorithm, key_value)?,
        })
    }
}

#[rquickjs::methods]
impl Hmac {
    fn digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        self.hash.hash_digest(ctx, encoding)
    }

    fn update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = bytes.as_bytes(&ctx)?;
        this.0.borrow_mut().hash.do_update(bytes);
        Ok(this.0)
    }
}
