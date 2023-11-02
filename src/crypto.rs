use std::slice;

use once_cell::sync::Lazy;
use ring::{
    digest::{self, Context as DigestContext},
    hmac::{self, Context as HmacContext},
    rand::{SecureRandom, SystemRandom},
};
use rquickjs::{
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest, This},
    Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, TypedArray, Value,
};

use crate::{
    encoding::encoder::{bytes_to_b64_string, bytes_to_hex_string},
    util::{
        bytes_to_typed_array, export_default, get_bytes, get_checked_len, obj_to_array_buffer,
        ResultExt,
    },
    uuid::uuidv4,
    vm::{CtxExtension, ErrorExtensions},
};

pub static SYSTEM_RANDOM: Lazy<SystemRandom> = Lazy::new(SystemRandom::new);

fn encoded_bytes<'js>(ctx: Ctx<'js>, bytes: &[u8], encoding: &str) -> Result<Value<'js>> {
    match encoding {
        "hex" => {
            let hex = bytes_to_hex_string(bytes);
            let hex = rquickjs::String::from_str(ctx, &hex)?;
            Ok(Value::from_string(hex))
        }
        "base64" => {
            let b64 = bytes_to_b64_string(bytes);
            let b64 = rquickjs::String::from_str(ctx, &b64)?;
            Ok(Value::from_string(b64))
        }
        _ => bytes_to_typed_array(ctx, bytes),
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Hmac {
    #[qjs(skip_trace)]
    context: HmacContext,
}

#[rquickjs::methods]
impl Hmac {
    #[qjs(skip)]
    fn new<'js>(ctx: Ctx<'js>, algorithm: String, secret: Value<'js>) -> Result<Hmac> {
        let key_value = get_bytes(&ctx, secret)?;

        let algorithm = match algorithm.to_lowercase().as_str() {
            "sha1" => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            "sha256" => hmac::HMAC_SHA256,
            "sha384" => hmac::HMAC_SHA384,
            "sha512" => hmac::HMAC_SHA512,
            _ => {
                return Err(Exception::throw_message(
                    &ctx,
                    &format!("Algorithm \"{}\" not supported", &algorithm),
                ))
            }
        };

        Ok(Hmac {
            context: HmacContext::with_key(&hmac::Key::new(algorithm, &key_value)),
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
        ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = get_bytes(&ctx, value)?;
        this.0.borrow_mut().context.update(&bytes);

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
    fn new(ctx: Ctx<'_>, algorithm: String) -> Result<Hash> {
        let algorithm = match algorithm.to_lowercase().as_str() {
            "sha1" => &digest::SHA1_FOR_LEGACY_USE_ONLY,
            "sha256" => &digest::SHA256,
            "sha512" => &digest::SHA512,
            _ => {
                return Err(Exception::throw_message(
                    &ctx,
                    &format!("Algorithm \"{}\" not supported", &algorithm),
                ))
            }
        };

        Ok(Hash {
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
        ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = get_bytes(&ctx, value)?;
        this.0.borrow_mut().context.update(&bytes);
        Ok(this.0)
    }
}

iterable_enum!(pub, ShaAlgorithm, SHA1, SHA256, SHA384, SHA512);

impl ShaAlgorithm {
    fn class_name(&self) -> &'static str {
        match self {
            ShaAlgorithm::SHA1 => "Sha1",
            ShaAlgorithm::SHA256 => "Sha256",
            ShaAlgorithm::SHA384 => "Sha384",
            ShaAlgorithm::SHA512 => "Sha512",
        }
    }
    fn hmac_algorithm(&self) -> &'static hmac::Algorithm {
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
    fn new<'js>(ctx: Ctx<'js>, algorithm: ShaAlgorithm, secret: Opt<Value<'js>>) -> Result<Self> {
        let secret = secret.0;
        let secret = match secret {
            Some(secret) => {
                let bytes = get_bytes(&ctx, secret)?;
                Some(bytes)
            }
            None => None,
        };

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
        ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = get_bytes(&ctx, value)?;
        this.0.borrow_mut().bytes = bytes;
        Ok(this.0)
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

    let array_buffer = TypedArray::new(ctx.clone(), random_bytes)?;
    array_buffer.into_js(&ctx)
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
            callback.call((err,))?;

            return Ok(());
        }
        callback.call((Null.into_js(&ctx), obj))?;
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

    if let Some(array_buffer) = obj_to_array_buffer(obj.clone())? {
        let checked_len = get_checked_len(array_buffer.len(), size.0, offset);

        let raw = array_buffer
            .as_raw()
            .ok_or("ArrayBuffer is detached")
            .or_throw(&ctx)?;
        let bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

        SYSTEM_RANDOM
            .fill(&mut bytes[offset..offset + checked_len])
            .unwrap();
    }

    Ok(obj)
}

pub struct CryptoModule;

impl ModuleDef for CryptoModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("createHash")?;
        declare.declare("createHmac")?;
        declare.declare("Crc32")?;
        declare.declare("Crc32c")?;
        declare.declare("randomBytes")?;
        declare.declare("randomUUID")?;
        declare.declare("randomFillSync")?;
        declare.declare("randomFill")?;

        for sha_algorithm in ShaAlgorithm::iterate() {
            let class_name = sha_algorithm.class_name();
            declare.declare(class_name)?;
        }

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<Hash>::register(ctx)?;
        Class::<Hmac>::register(ctx)?;
        Class::<ShaHash>::register(ctx)?;

        export_default(ctx, exports, |default| {
            for sha_algorithm in ShaAlgorithm::iterate() {
                let class_name: &str = sha_algorithm.class_name();
                let algo = sha_algorithm;

                let ctor =
                    Constructor::new_class::<ShaHash, _, _>(ctx.clone(), move |ctx, secret| {
                        ShaHash::new(ctx, algo, secret)
                    })?;

                default.set(class_name, ctor)?;
            }

            default.set("createHash", Func::from(Hash::new))?;
            default.set("createHmac", Func::from(Hmac::new))?;
            default.set("randomBytes", Func::from(get_random_bytes))?;
            default.set("randomUUID", Func::from(uuidv4))?;
            default.set("randomFillSync", Func::from(random_fill_sync))?;
            default.set("randomFill", Func::from(random_fill))?;
            Ok(())
        })?;

        Ok(())
    }
}
