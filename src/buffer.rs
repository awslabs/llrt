use rquickjs::{
    atom::PredefinedAtom,
    cstr,
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, This},
    Array, ArrayBuffer, Ctx, Exception, IntoJs, Object, Result, TypedArray, Value,
};

use crate::{
    encoding::encoder::Encoder,
    module::export_default,
    utils::{
        object::{get_bytes, get_bytes_offset_length, obj_to_array_buffer},
        result::ResultExt,
    },
};

pub struct Buffer(pub Vec<u8>);

impl<'js> IntoJs<'js> for Buffer {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let array_buffer = ArrayBuffer::new(ctx.clone(), self.0)?;
        let value = array_buffer.into_js(ctx)?;
        let constructor: Constructor = ctx.globals().get(stringify!(Buffer))?;
        constructor.construct((value,))
    }
}

fn byte_length<'js>(ctx: Ctx<'js>, value: Value<'js>, encoding: Opt<String>) -> Result<usize> {
    //slow path
    if let Some(encoding) = encoding.0 {
        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
        let bytes = get_bytes(&ctx, value)?;
        return Ok(encoder.decode(bytes).or_throw(&ctx)?.len());
    }
    //fast path
    if let Some(val) = value.as_string() {
        return Ok(val.to_string()?.len());
    }

    if value.is_array() {
        let array = value.as_array().unwrap();

        for val in array.iter::<u8>() {
            val.or_throw_msg(&ctx, "array value is not u8")?;
        }

        return Ok(array.len());
    }

    if let Some(obj) = value.as_object() {
        if let Some(array_buffer) = obj_to_array_buffer(&ctx, obj)? {
            return Ok(array_buffer.len());
        }
    }

    Err(Exception::throw_message(
        &ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or string",
    ))
}

fn to_string(this: This<Object<'_>>, ctx: Ctx, encoding: Opt<String>) -> Result<String> {
    let typed_array = TypedArray::<u8>::from_object(this.0)?;
    let bytes: &[u8] = typed_array.as_ref();
    let encoding = encoding.0.unwrap_or_else(|| String::from("utf-8"));
    let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
    encoder.encode_to_string(bytes).or_throw(&ctx)
}

fn alloc<'js>(
    ctx: Ctx<'js>,
    length: usize,
    fill: Opt<Value<'js>>,
    encoding: Opt<String>,
) -> Result<Value<'js>> {
    if let Some(value) = fill.0 {
        if let Some(value) = value.as_string() {
            let string = value.to_string()?;

            if let Some(encoding) = encoding.0 {
                let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
                let bytes = encoder.decode_from_string(string).or_throw(&ctx)?;
                return alloc_byte_ref(&ctx, &bytes, length);
            }

            let byte_ref = string.as_bytes();

            return alloc_byte_ref(&ctx, byte_ref, length);
        }
        if let Some(value) = value.as_int() {
            let bytes = vec![value as u8; length];
            return Buffer(bytes).into_js(&ctx);
        }
        if let Some(obj) = value.as_object() {
            if let Some(array_buffer) = obj_to_array_buffer(&ctx, obj)? {
                return alloc_byte_ref(&ctx, array_buffer.as_ref(), length);
            }
        }
    }

    Buffer(vec![0; length]).into_js(&ctx)
}

fn alloc_byte_ref<'js>(ctx: &Ctx<'js>, byte_ref: &[u8], length: usize) -> Result<Value<'js>> {
    let mut bytes = vec![0; length];
    let byte_ref_length = byte_ref.len();
    for i in 0..length {
        bytes[i] = byte_ref[i % byte_ref_length];
    }
    return Buffer(bytes).into_js(ctx);
}

fn concat<'js>(ctx: Ctx<'js>, list: Array<'js>, max_length: Opt<usize>) -> Result<Value<'js>> {
    let mut bytes = Vec::new();
    let mut total_length = 0;
    let mut length;
    for value in list.iter::<Object>() {
        let typed_array = TypedArray::<u8>::from_object(value?)?;
        let bytes_ref: &[u8] = typed_array.as_ref();

        length = bytes_ref.len();

        if length == 0 {
            continue;
        }

        if let Some(max_length) = max_length.0 {
            total_length += length;
            if total_length > max_length {
                let diff = max_length - (total_length - length);
                bytes.extend_from_slice(&bytes_ref[0..diff]);
                break;
            }
        }
        bytes.extend_from_slice(bytes_ref);
    }

    Buffer(bytes).into_js(&ctx)
}

fn from<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    offset_or_encoding: Opt<Value<'js>>,
    length: Opt<usize>,
) -> Result<Value<'js>> {
    let mut encoding: Option<String> = None;
    let mut offset: Option<usize> = None;

    if let Some(offset_or_encoding) = offset_or_encoding.0 {
        if offset_or_encoding.is_string() {
            encoding = Some(offset_or_encoding.get()?);
        } else if offset_or_encoding.is_number() {
            offset = Some(offset_or_encoding.get()?);
        }
    }

    let mut bytes = get_bytes_offset_length(&ctx, value, offset, length.0)?;

    if let Some(encoding) = encoding {
        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
        bytes = encoder.decode(bytes).or_throw(&ctx)?;
    }

    Buffer(bytes).into_js(&ctx)
}

fn set_prototype<'js>(ctx: &Ctx<'js>, constructor: Object<'js>) -> Result<()> {
    let _ = &constructor.set(PredefinedAtom::From, Func::from(from))?;
    let _ = &constructor.set(stringify!(alloc), Func::from(alloc))?;
    let _ = &constructor.set(stringify!(concat), Func::from(concat))?;
    let _ = &constructor.set("byteLength", Func::from(byte_length))?;

    let prototype: &Object = &constructor.get(PredefinedAtom::Prototype)?;
    prototype.set(PredefinedAtom::ToString, Func::from(to_string))?;

    ctx.globals().set(stringify!(Buffer), constructor)?;

    Ok(())
}

pub fn init<'js>(ctx: &Ctx<'js>) -> Result<()> {
    let buffer = ctx.eval::<Object<'js>, &str>(&format!(
        "class {0} extends Uint8Array {{}}\n{0}",
        stringify!(Buffer)
    ))?;
    set_prototype(ctx, buffer)
}

pub struct BufferModule;

impl ModuleDef for BufferModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(Buffer))?;
        declare.declare_static(cstr!("default"))?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let buf: Constructor = globals.get(stringify!(Buffer))?;

        export_default(ctx, exports, |default| {
            default.set(stringify!(Buffer), buf)?;
            Ok(())
        })?;

        Ok(())
    }
}
