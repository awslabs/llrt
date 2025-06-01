// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod crc32;
mod md5_hash;
mod sha_hash;
#[cfg(feature = "subtle-rs")]
mod subtle;

use std::slice;

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_encoding::{bytes_to_b64_string, bytes_to_hex_string};
use llrt_utils::{
    bytes::{bytes_to_typed_array, get_start_end_indexes, ObjectBytes},
    error::ErrorExtensions,
    error_messages::{ERROR_MSG_ARRAY_BUFFER_DETACHED, ERROR_MSG_NOT_ARRAY_BUFFER},
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use once_cell::sync::Lazy;
use rand::prelude::ThreadRng;
use rand::Rng;
use ring::rand::{SecureRandom, SystemRandom};
#[cfg(feature = "subtle-rs")]
use rquickjs::prelude::Async;
use rquickjs::{
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, Value,
};
#[cfg(feature = "subtle-rs")]
use subtle::{
    subtle_decrypt, subtle_derive_bits, subtle_derive_key, subtle_digest, subtle_encrypt,
    subtle_export_key, subtle_generate_key, subtle_import_key, subtle_sign, subtle_unwrap_key,
    subtle_verify, subtle_wrap_key, CryptoKey, SubtleCrypto,
};
use uuid::Uuid;
use uuid_simd::UuidExt;

use self::{
    crc32::{Crc32, Crc32c},
    md5_hash::Md5,
    sha_hash::{Hash, Hmac, ShaAlgorithm, ShaHash},
};

pub static SYSTEM_RANDOM: Lazy<SystemRandom> = Lazy::new(SystemRandom::new);

fn encoded_bytes<'js>(ctx: Ctx<'js>, bytes: &[u8], encoding: &str) -> Result<Value<'js>> {
    match encoding {
        "hex" => {
            let hex = bytes_to_hex_string(bytes);
            let hex = rquickjs::String::from_str(ctx, &hex)?;
            Ok(Value::from_string(hex))
        },
        "base64" => {
            let b64 = bytes_to_b64_string(bytes);
            let b64 = rquickjs::String::from_str(ctx, &b64)?;
            Ok(Value::from_string(b64))
        },
        _ => bytes_to_typed_array(ctx, bytes),
    }
}

#[inline]
pub fn random_byte_array(length: usize) -> Vec<u8> {
    let mut vec = vec![0; length];
    SYSTEM_RANDOM.fill(&mut vec).unwrap();
    vec
}

fn get_random_bytes(ctx: Ctx, length: usize) -> Result<Value> {
    let random_bytes = random_byte_array(length);
    Buffer(random_bytes).into_js(&ctx)
}

fn get_random_int(first: i64, second: Opt<i64>) -> Result<i64> {
    let mut rng = ThreadRng::default();
    let random_number = match second.0 {
        Some(max) => rng.gen_range(first..max),
        None => rng.gen_range(0..first),
    };

    Ok(random_number)
}

fn random_fill<'js>(ctx: Ctx<'js>, obj: Object<'js>, args: Rest<Value<'js>>) -> Result<()> {
    let args_iter = args.0.into_iter();
    let mut args_iter = args_iter.rev();

    let callback: Function = args_iter
        .next()
        .and_then(|v| v.into_function())
        .or_throw_msg(&ctx, "Callback required")?;
    let size = args_iter
        .next()
        .and_then(|arg| arg.as_int())
        .map(|i| i as usize);
    let offset = args_iter
        .next()
        .and_then(|arg| arg.as_int())
        .map(|i| i as usize);

    ctx.clone().spawn_exit(async move {
        if let Err(err) = random_fill_sync(ctx.clone(), obj.clone(), Opt(offset), Opt(size)) {
            let err = err.into_value(&ctx)?;
            () = callback.call((err,))?;

            return Ok(());
        }
        () = callback.call((Null.into_js(&ctx), obj))?;
        Ok::<_, Error>(())
    })?;
    Ok(())
}

fn random_fill_sync<'js>(
    ctx: Ctx<'js>,
    obj: Object<'js>,
    offset: Opt<usize>,
    size: Opt<usize>,
) -> Result<Object<'js>> {
    let offset = offset.unwrap_or(0);

    if let Some(object_bytes) = ObjectBytes::from_array_buffer(&obj)? {
        let (array_buffer, source_length, source_offset) = object_bytes
            .get_array_buffer()?
            .expect(ERROR_MSG_NOT_ARRAY_BUFFER);
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(&ctx)?;

        let (start, end) = get_start_end_indexes(source_length, size.0, offset);

        let bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), source_length) };

        SYSTEM_RANDOM
            .fill(&mut bytes[start + source_offset..end - source_offset])
            .unwrap();
    }

    Ok(obj)
}

fn get_random_values<'js>(ctx: Ctx<'js>, obj: Object<'js>) -> Result<Object<'js>> {
    if let Some(object_bytes) = ObjectBytes::from_array_buffer(&obj)? {
        if matches!(
            object_bytes,
            ObjectBytes::F64Array(_) | ObjectBytes::F32Array(_)
        ) {
            return Err(Exception::throw_message(&ctx, "Unsupported TypedArray"));
        }

        let (array_buffer, source_length, source_offset) = object_bytes
            .get_array_buffer()?
            .expect(ERROR_MSG_NOT_ARRAY_BUFFER);
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(&ctx)?;

        if source_length > 0x10000 {
            return Err(Exception::throw_message(
                &ctx,
                "QuotaExceededError: The requested length exceeds 65,536 bytes",
            ));
        }

        let bytes = unsafe {
            std::slice::from_raw_parts_mut(raw.ptr.as_ptr().add(source_offset), source_length)
        };

        SYSTEM_RANDOM.fill(bytes).unwrap()
    }

    Ok(obj)
}

fn uuidv4() -> String {
    Uuid::new_v4().format_hyphenated().to_string()
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime, rquickjs::class::Trace)]
struct Crypto {}

#[rquickjs::methods]
impl Crypto {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>) -> Result<Self> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<Crypto>::define(&globals)?;
    let crypto = Class::instance(ctx.clone(), Crypto {})?;

    crypto.set("createHash", Func::from(Hash::new))?;
    crypto.set("createHmac", Func::from(Hmac::new))?;
    crypto.set("randomBytes", Func::from(get_random_bytes))?;
    crypto.set("randomInt", Func::from(get_random_int))?;
    crypto.set("randomUUID", Func::from(uuidv4))?;
    crypto.set("randomFillSync", Func::from(random_fill_sync))?;
    crypto.set("randomFill", Func::from(random_fill))?;
    crypto.set("getRandomValues", Func::from(get_random_values))?;

    #[cfg(feature = "subtle-rs")]
    {
        Class::<SubtleCrypto>::define(&globals)?;
        Class::<CryptoKey>::define(&globals)?;

        let subtle = Class::instance(ctx.clone(), SubtleCrypto {})?;

        subtle.set("decrypt", Func::from(Async(subtle_decrypt)))?;
        subtle.set("deriveKey", Func::from(Async(subtle_derive_key)))?;
        subtle.set("deriveBits", Func::from(Async(subtle_derive_bits)))?;
        subtle.set("digest", Func::from(Async(subtle_digest)))?;
        subtle.set("encrypt", Func::from(Async(subtle_encrypt)))?;
        subtle.set("exportKey", Func::from(Async(subtle_export_key)))?;
        subtle.set("generateKey", Func::from(Async(subtle_generate_key)))?;
        subtle.set("importKey", Func::from(Async(subtle_import_key)))?;
        subtle.set("sign", Func::from(Async(subtle_sign)))?;
        subtle.set("verify", Func::from(Async(subtle_verify)))?;
        subtle.set("wrapKey", Func::from(Async(subtle_wrap_key)))?;
        subtle.set("unwrapKey", Func::from(Async(subtle_unwrap_key)))?;
        crypto.set("subtle", subtle)?;
    }

    globals.set("crypto", crypto)?;

    Ok(())
}

pub struct CryptoModule;

impl ModuleDef for CryptoModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createHash")?;
        declare.declare("createHmac")?;
        declare.declare("Crc32")?;
        declare.declare("Crc32c")?;
        declare.declare("Md5")?;
        declare.declare("randomBytes")?;
        declare.declare("randomUUID")?;
        declare.declare("randomInt")?;
        declare.declare("randomFillSync")?;
        declare.declare("randomFill")?;
        declare.declare("getRandomValues")?;

        for sha_algorithm in ShaAlgorithm::iter() {
            let class_name = sha_algorithm.class_name();
            declare.declare(class_name)?;
        }
        declare.declare("crypto")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            for sha_algorithm in ShaAlgorithm::iter() {
                let class_name: &str = sha_algorithm.class_name();
                let algo = sha_algorithm;

                let ctor =
                    Constructor::new_class::<ShaHash, _, _>(ctx.clone(), move |ctx, secret| {
                        struct Args<'js>(Ctx<'js>, Opt<ObjectBytes<'js>>);
                        let Args(ctx, secret) = Args(ctx, secret);
                        ShaHash::new(ctx, algo.clone(), secret)
                    })?;

                default.set(class_name, ctor)?;
            }

            let crypto: Object = ctx.globals().get("crypto")?;

            Class::<Md5>::define(default)?;
            Class::<Crc32>::define(default)?;
            Class::<Crc32c>::define(default)?;

            default.set("createHash", Func::from(Hash::new))?;
            default.set("createHmac", Func::from(Hmac::new))?;
            default.set("randomBytes", Func::from(get_random_bytes))?;
            default.set("randomInt", Func::from(get_random_int))?;
            default.set("randomUUID", Func::from(uuidv4))?;
            default.set("randomFillSync", Func::from(random_fill_sync))?;
            default.set("randomFill", Func::from(random_fill))?;
            default.set("getRandomValues", Func::from(get_random_values))?;
            default.set("crypto", crypto)?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<CryptoModule> for ModuleInfo<CryptoModule> {
    fn from(val: CryptoModule) -> Self {
        ModuleInfo {
            name: "crypto",
            module: val,
        }
    }
}
