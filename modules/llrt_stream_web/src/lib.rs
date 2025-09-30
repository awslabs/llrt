use llrt_utils::{
    module::{export_default, ModuleInfo},
    primordials::Primordial,
};
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

use crate::{
    readable::{ArrayConstructorPrimordials, IteratorPrimordials},
    utils::promise::PromisePrimordials,
    writable::WritableStreamDefaultControllerPrimordials,
};

mod queuing_strategy;
mod readable;
mod readable_writable_pair;
mod utils;
mod writable;

/// Defines web streams, which are exposed through the "stream/web" Node import, but also at the global scope
/// Web streams consist of Readable, Writable, and Transform streams. Transform is currently unimplemented.
///
/// https://developer.mozilla.org/en-US/docs/Web/API/Streams_API
///
/// # ReadableStream
/// ReadableStream knows how to 'pull' objects or bytes from an underlying source, generally a user-defined function or an [async] iterator.
/// A source enqueues data to the stream via a controller, either ReadableStreamDefaultController or a ReadableByteStreamController optionally for byte data.
/// The controller is created at stream initialisation and cannot change.
///
/// Data is read from the stream using a reader, which is obtained using stream.getReader(). A reader 'locks' the stream for reading, preventing
/// other readers from being created. When a reader is released with `reader.releaseLock()`, the stream goes back to having no reader and a new one can be created.
/// In the case of ReadableByteStreamController, a special reader ReadableStreamBYOBReader may be used, which allows users to provide their own
/// buffer to fill bytes into when reading. Otherwise, ReadableStreamDefaultReader is used by default, and this may also be used with byte streams.
///
/// A ReadableStream can be 'tee'd', which splits it into two readable streams which both read the same underlying data, potentially at different
/// paces. This is an area of substantial complexity for the implementation, particularly in the case of byte streams as the alternative reader types
/// must be handled correctly.
///
/// # WritableStream
/// WritableStream knows how to 'push' objects into an underlying sink, generally a user-defined function. It has no special casing for bytes, and so
/// only has one type of controller, WritableStreamDefaultController, and only one type of writer WritableStreamDefaultWriter. The controller is only needed for
/// error handling because writes are signalled via a function call to a user-defined 'write' method which receives the chunk directly.
///
/// Data is written to the stream using a WritableStreamDefaultWriter, which is obtained using stream.getWriter(). A writer 'locks' the stream for writing,
/// preventing other writers from being created. When a writer is released with `writer.releaseLock()`, the stream goes back to having no writer and a new one can be created.
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

            ArrayConstructorPrimordials::init(ctx)?;
            WritableStreamDefaultControllerPrimordials::init(ctx)?;
            IteratorPrimordials::init(ctx)?;
            PromisePrimordials::init(ctx)?;

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
    Class::<ReadableStreamDefaultController>::define(globals)?;
    Class::<ReadableStreamDefaultReader>::define(globals)?;

    Class::<WritableStream>::define(globals)?;
    Class::<WritableStreamDefaultController>::define(globals)?;

    // This is exposed globally by Node even though its not in the min-common-api
    Class::<WritableStreamDefaultWriter>::define(globals)?;

    Ok(())
}
