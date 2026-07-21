// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    module::{export_default, ModuleInfo},
    object::define_subclass,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    function::{Args, Constructor, Rest},
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Function, IntoJs, Object, Result, Value,
};

pub use self::array_buffer_view::*;
pub use self::blob::*;
pub use self::buffer::*;
pub use self::file::*;

mod array_buffer_view;
mod blob;
mod buffer;
mod file;

pub struct BufferModule;

impl ModuleDef for BufferModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Buffer))?;
        declare.declare("atob")?;
        declare.declare("btoa")?;
        declare.declare("constants")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let buf: Constructor = globals.get(stringify!(Buffer))?;

        let constants = Object::new(ctx.clone())?;
        constants.set("MAX_LENGTH", u32::MAX)?; // For QuickJS
        constants.set("MAX_STRING_LENGTH", (1 << 30) - 1)?; // For QuickJS

        let atob: Function = ctx.globals().get("atob")?;
        let btoa: Function = ctx.globals().get("btoa")?;

        export_default(ctx, exports, |default| {
            default.set(stringify!(Buffer), buf)?;
            default.set("atob", atob.into_js(ctx)?)?;
            default.set("btoa", btoa.into_js(ctx)?)?;
            default.set("constants", constants)?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<BufferModule> for ModuleInfo<BufferModule> {
    fn from(val: BufferModule) -> Self {
        ModuleInfo {
            name: "buffer",
            module: val,
        }
    }
}

pub fn init<'js>(ctx: &Ctx<'js>) -> Result<()> {
    let globals = ctx.globals();
    BasePrimordials::init(ctx)?;

    // Buffer extends the native Uint8Array: it forwards construction to the
    // Uint8Array constructor and inherits its static and prototype members.
    let uint8array = BasePrimordials::get(ctx)?.constructor_uint8array.clone();
    let buffer_ctor = define_subclass(
        ctx,
        stringify!(Buffer),
        &uint8array,
        |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
            let uint8array = &BasePrimordials::get(&ctx)?.constructor_uint8array;
            let mut ctor_args = Args::new(ctx.clone(), args.0.len());
            ctor_args.push_args(args.0)?;
            ctor_args.construct::<Value>(uint8array)
        },
    )?;
    let buffer: Object = buffer_ctor.into_value().into_object().unwrap();
    set_prototype(ctx, buffer)?;

    BufferPrimordials::init(ctx)?;

    // Blob
    Class::<Blob>::define(&globals)?;

    // File
    Class::<File>::define(&globals)?;

    //init primordials
    let _ = BufferPrimordials::get(ctx)?;

    Ok(())
}
