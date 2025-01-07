use llrt_utils::module::{export_default, ModuleInfo};
use queuing_strategy::{ByteLengthQueuingStrategy, CountQueuingStrategy};
use readable::{
    ReadableByteStreamController, ReadableStream, ReadableStreamBYOBReader,
    ReadableStreamBYOBRequest, ReadableStreamDefaultController, ReadableStreamDefaultReader,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Result,
};
use writable::{WritableStream, WritableStreamDefaultController, WritableStreamDefaultWriter};

mod queuing_strategy;
mod readable;
mod readable_writable_pair;
mod utils;
mod writable;

pub struct StreamWebModule;

// https://nodejs.org/api/webstreams.html
impl ModuleDef for StreamWebModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(ReadableStream))?;
        declare.declare(stringify!(ReadableStreamDefaultReader))?;
        declare.declare(stringify!(ReadableStreamBYOBReader))?;
        declare.declare(stringify!(ReadableStreamDefaultController))?;
        declare.declare(stringify!(ReadableByteStreamController))?;
        declare.declare(stringify!(ReadableStreamBYOBRequest))?;

        declare.declare(stringify!(WritableStream))?;
        declare.declare(stringify!(WritableStreamDefaultWriter))?;
        declare.declare(stringify!(WritableStreamDefaultController))?;

        declare.declare(stringify!(ByteLengthQueuingStrategy))?;
        declare.declare(stringify!(CountQueuingStrategy))?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<ReadableStream>::define(default)?;
            Class::<ReadableStreamDefaultReader>::define(default)?;
            Class::<ReadableStreamBYOBReader>::define(default)?;
            Class::<ReadableStreamDefaultController>::define(default)?;
            Class::<ReadableByteStreamController>::define(default)?;
            Class::<ReadableStreamBYOBRequest>::define(default)?;

            Class::<WritableStream>::define(default)?;
            Class::<WritableStreamDefaultWriter>::define(default)?;
            Class::<WritableStreamDefaultController>::define(default)?;

            Class::<ByteLengthQueuingStrategy>::define(default)?;
            Class::<CountQueuingStrategy>::define(default)?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<StreamWebModule> for ModuleInfo<StreamWebModule> {
    fn from(val: StreamWebModule) -> Self {
        ModuleInfo {
            name: "stream/web",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = &ctx.globals();

    // https://min-common-api.proposal.wintercg.org/#api-index
    Class::<ByteLengthQueuingStrategy>::define(globals)?;
    Class::<CountQueuingStrategy>::define(globals)?;

    Class::<ReadableByteStreamController>::define(globals)?;
    Class::<ReadableStream>::define(globals)?;
    Class::<ReadableStreamBYOBReader>::define(globals)?;
    Class::<ReadableStreamBYOBRequest>::define(globals)?;
    Class::<ReadableStreamDefaultReader>::define(globals)?;

    Class::<WritableStream>::define(globals)?;
    Class::<WritableStreamDefaultController>::define(globals)?;

    Ok(())
}
