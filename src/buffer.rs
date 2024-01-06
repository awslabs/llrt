use rquickjs::{
    atom::PredefinedAtom,
    cstr,
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, This},
    Array, ArrayBuffer, Ctx, IntoJs, Object, Result, TypedArray, Value,
};

use crate::{
    encoding::encoder::Encoder,
    util::{export_default, get_bytes_offset_length, ResultExt},
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

fn to_string(this: This<Object<'_>>, ctx: Ctx, encoding: Opt<String>) -> Result<String> {
    let typed_array = TypedArray::<u8>::from_object(this.0)?;
    let bytes: &[u8] = typed_array.as_ref();
    let encoding = encoding.0.unwrap_or_else(|| String::from("utf-8"));
    let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
    encoder.encode_to_string(bytes).or_throw(&ctx)
}

fn alloc(ctx: Ctx<'_>, length: usize) -> Result<Value<'_>> {
    let zero_vec = vec![0; length];

    Buffer(zero_vec).into_js(&ctx)
}

fn concat<'js>(ctx: Ctx<'js>, list: Array<'js>, total_length: Opt<usize>) -> Result<Value<'js>> {
    let mut bytes = Vec::new();
    let mut current_length = 0;
    for value in list.iter::<Object>() {
        let typed_array = TypedArray::<u8>::from_object(value?)?;
        let bytes_ref: &[u8] = typed_array.as_ref();
        bytes.extend_from_slice(bytes_ref);
    }

    Buffer(bytes).into_js(&ctx)
    //TypedArray::<u8>::from_object(this.0)?;
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
    let _ = &constructor.set("alloc", Func::from(alloc))?;

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
