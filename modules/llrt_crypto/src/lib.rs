// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Compile-time checks for conflicting crypto features
#[cfg(all(feature = "crypto-rust", feature = "crypto-openssl"))]
compile_error!("Features `crypto-rust` and `crypto-openssl` are mutually exclusive");

#[cfg(all(feature = "crypto-rust", feature = "crypto-ring"))]
compile_error!("Features `crypto-rust` and `crypto-ring` are mutually exclusive");

#[cfg(all(feature = "crypto-rust", feature = "crypto-graviola"))]
compile_error!("Features `crypto-rust` and `crypto-graviola` are mutually exclusive");

#[cfg(all(feature = "crypto-openssl", feature = "crypto-ring"))]
compile_error!("Features `crypto-openssl` and `crypto-ring` are mutually exclusive");

#[cfg(all(feature = "crypto-openssl", feature = "crypto-graviola"))]
compile_error!("Features `crypto-openssl` and `crypto-graviola` are mutually exclusive");

#[cfg(all(feature = "crypto-ring", feature = "crypto-graviola"))]
compile_error!("Features `crypto-ring` and `crypto-graviola` are mutually exclusive");

mod crc32;
mod hash;
mod subtle;

mod provider;

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
use rand::RngExt;
use rquickjs::prelude::Async;
use rquickjs::{
    atom::PredefinedAtom,
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, Value,
};
use subtle::{
    subtle_decrypt, subtle_derive_bits, subtle_derive_key, subtle_digest, subtle_encrypt,
    subtle_export_key, subtle_generate_key, subtle_import_key, subtle_sign, subtle_unwrap_key,
    subtle_verify, subtle_wrap_key, CryptoKey, SubtleCrypto,
};

use self::{
    crc32::{Crc32, Crc32c},
    hash::{Hash, Hmac},
};

static CRYPTO_PROVIDER: Lazy<provider::DefaultProvider> =
    Lazy::new(|| provider::DefaultProvider {});

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
    let mut vec = vec![0u8; length];
    rand::rng().fill(&mut vec[..]);
    vec
}

fn get_random_bytes(ctx: Ctx, length: usize) -> Result<Value> {
    let random_bytes = random_byte_array(length);
    Buffer(random_bytes).into_js(&ctx)
}

fn get_random_int(first: i64, second: Opt<i64>) -> Result<i64> {
    let mut rng = rand::rng();
    let random_number = match second.0 {
        Some(max) => rng.random_range(first..max),
        None => rng.random_range(0..first),
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

        rand::rng().fill(&mut bytes[start + source_offset..end - source_offset]);
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

        rand::rng().fill(bytes)
    }

    Ok(obj)
}

fn uuidv4() -> String {
    let uuid = rand::random::<u128>() & 0xFFFFFFFFFFFF4FFFBFFFFFFFFFFFFFFF | 0x40008000000000000000;

    static HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let bytes = uuid.to_be_bytes();

    let mut buf = [0u8; 36];

    // Precomputed positions for 32 hex digits (excluding hyphens)
    static HEX_POS: [usize; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35,
    ];

    // Map each byte to its hex representation
    let mut hex_idx = 0;
    for &byte in &bytes[..] {
        let high = HEX_CHARS[(byte >> 4) as usize];
        let low = HEX_CHARS[(byte & 0x0f) as usize];

        buf[HEX_POS[hex_idx]] = high;
        buf[HEX_POS[hex_idx + 1]] = low;
        hex_idx += 2;
    }

    // Insert hyphens at standard positions
    buf[8] = b'-';
    buf[13] = b'-';
    buf[18] = b'-';
    buf[23] = b'-';

    // SAFETY: The buffer only contains valid UTF-8 characters (hex digits and hyphens)
    // that were explicitly set from the HEX_CHARS array and hyphen literals
    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
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

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(Crypto)
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
        declare.declare("randomBytes")?;
        declare.declare("randomUUID")?;
        declare.declare("randomInt")?;
        declare.declare("randomFillSync")?;
        declare.declare("randomFill")?;
        declare.declare("getRandomValues")?;
        declare.declare("crypto")?;
        declare.declare("webcrypto")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let crypto: Object = ctx.globals().get("crypto")?;

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
            default.set("crypto", crypto.clone())?;
            default.set("webcrypto", crypto)?;
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
