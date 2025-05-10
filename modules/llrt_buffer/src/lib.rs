// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    module::{export_default, ModuleInfo},
    primordials::Primordial,
};
use rquickjs::{
    atom::PredefinedAtom,
    function::Constructor,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Object, Result,
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

        export_default(ctx, exports, |default| {
            default.set(stringify!(Buffer), buf)?;
            default.set("atob", Func::from(atob))?;
            default.set("btoa", Func::from(btoa))?;
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

    // Buffer
    let buffer = ctx.eval::<Object<'js>, &str>(concat!(
        "class ",
        stringify!(Buffer),
        " extends Uint8Array {}\n",
        stringify!(Buffer),
    ))?;
    set_prototype(ctx, buffer)?;

    // Blob
    if let Some(constructor) = Class::<Blob>::create_constructor(ctx)? {
        constructor.prop(
            PredefinedAtom::SymbolHasInstance,
            Func::from(Blob::has_instance),
        )?;
        globals.set(stringify!(Blob), constructor)?;
    }

    // File
    Class::<File>::define(&globals)?;

    //init primordials
    let _ = BufferPrimordials::get(ctx)?;

    // Conversion
    globals.set("atob", Func::from(atob))?;
    globals.set("btoa", Func::from(btoa))?;

    Ok(())
}
