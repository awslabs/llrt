use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    prelude::{Opt, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Promise, Result, Value,
};

use crate::{
    readable::{
        readable_stream_default_controller_close_stream,
        readable_stream_default_controller_enqueue_value,
        readable_stream_default_controller_error_stream, ReadableStreamDefaultControllerClass,
    },
    utils::promise::{promise_resolved_with, ResolveablePromise},
};

use llrt_utils::primordials::Primordial;

use super::stream::TransformStreamClass;

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub(crate) struct TransformStreamDefaultController<'js> {
    pub(super) stream: TransformStreamClass<'js>,
    pub(super) transform_algorithm: Option<TransformAlgorithm<'js>>,
    pub(super) flush_algorithm: Option<FlushAlgorithm<'js>>,
    pub(super) cancel_algorithm: Option<CancelAlgorithm<'js>>,
    pub(super) finish_promise: Option<ResolveablePromise<'js>>,
}

pub(crate) type TransformStreamDefaultControllerClass<'js> =
    Class<'js, TransformStreamDefaultController<'js>>;

fn get_readable_default_controller<'js>(
    stream_class: &TransformStreamClass<'js>,
) -> Option<ReadableStreamDefaultControllerClass<'js>> {
    let stream = stream_class.borrow();
    let readable = stream.readable.as_ref()?;
    let readable = readable.borrow();
    match &readable.controller {
        crate::readable::ReadableStreamControllerClass::ReadableStreamDefaultController(c) => {
            Some(c.clone())
        },
        _ => None,
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TransformStreamDefaultController<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    #[qjs(get)]
    fn desired_size(&self) -> Option<f64> {
        let stream = self.stream.borrow();
        let readable_class = stream.readable.as_ref()?;
        let readable = readable_class.borrow();
        match &readable.controller {
            crate::readable::ReadableStreamControllerClass::ReadableStreamDefaultController(c) => {
                let c = c.borrow();
                c.readable_stream_default_controller_get_desired_size(&readable)
                    .0
            },
            _ => None,
        }
    }

    fn enqueue(
        ctx: Ctx<'js>,
        this: This<OwnedBorrowMut<'js, Self>>,
        chunk: Opt<Value<'js>>,
    ) -> Result<()> {
        let chunk = chunk.0.unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        let stream_class = this.stream.clone();
        drop(this);
        transform_stream_default_controller_enqueue(ctx, &stream_class, chunk)
    }

    fn error(
        ctx: Ctx<'js>,
        this: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<()> {
        let reason = reason
            .0
            .unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        let stream_class = this.stream.clone();
        drop(this);
        transform_stream_error(ctx, &stream_class, reason)
    }

    fn terminate(ctx: Ctx<'js>, this: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        let stream_class = this.stream.clone();
        drop(this);
        transform_stream_default_controller_terminate(ctx, &stream_class)
    }
}

impl<'js> TransformStreamDefaultController<'js> {
    pub(super) fn clear_algorithms(&mut self) {
        self.transform_algorithm = None;
        self.flush_algorithm = None;
        self.cancel_algorithm = None;
    }
}

#[derive(Trace, JsLifetime, Clone)]
pub(super) enum TransformAlgorithm<'js> {
    Identity,
    Function {
        f: Function<'js>,
        transformer: Option<Object<'js>>,
    },
}

#[derive(Trace, JsLifetime, Clone)]
pub(super) enum FlushAlgorithm<'js> {
    Noop,
    Function {
        f: Function<'js>,
        transformer: Option<Object<'js>>,
    },
}

#[derive(Trace, JsLifetime, Clone)]
pub(super) enum CancelAlgorithm<'js> {
    Noop,
    Function {
        f: Function<'js>,
        transformer: Option<Object<'js>>,
    },
}

// --- Abstract operations ---

pub(super) fn transform_stream_default_controller_enqueue<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    chunk: Value<'js>,
) -> Result<()> {
    let controller_class = get_readable_default_controller(stream_class)
        .ok_or_else(|| Exception::throw_type(&ctx, "readable controller not available"))?;

    readable_stream_default_controller_enqueue_value(ctx.clone(), controller_class.clone(), chunk)?;

    // Update backpressure
    let has_backpressure = {
        let stream = stream_class.borrow();
        let readable_class = stream.readable.as_ref().unwrap();
        let readable = readable_class.borrow();
        let c = controller_class.borrow();
        let desired = c.readable_stream_default_controller_get_desired_size(&readable);
        match desired.0 {
            Some(size) => size <= 0.0,
            None => true,
        }
    };

    let current_bp = stream_class.borrow().backpressure;
    if has_backpressure != current_bp {
        transform_stream_set_backpressure(stream_class, true);
    }

    Ok(())
}

pub(super) fn transform_stream_default_controller_terminate<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
) -> Result<()> {
    let controller_class = get_readable_default_controller(stream_class)
        .ok_or_else(|| Exception::throw_type(&ctx, "readable controller not available"))?;

    readable_stream_default_controller_close_stream(ctx.clone(), controller_class)?;

    let error = ctx.eval::<Value, _>("new TypeError('TransformStream terminated')")?;
    transform_stream_error_writable_and_unblock_write(stream_class, error);
    Ok(())
}

pub(super) fn transform_stream_error<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    e: Value<'js>,
) -> Result<()> {
    let controller_class = get_readable_default_controller(stream_class)
        .ok_or_else(|| Exception::throw_type(&ctx, "readable controller not available"))?;

    readable_stream_default_controller_error_stream(controller_class, e.clone())?;
    transform_stream_error_writable_and_unblock_write(stream_class, e);
    Ok(())
}

pub(super) fn transform_stream_error_writable_and_unblock_write<'js>(
    stream_class: &TransformStreamClass<'js>,
    _e: Value<'js>,
) {
    let stream = stream_class.borrow_mut();
    if let Some(ref controller_class) = stream.controller {
        controller_class.borrow_mut().clear_algorithms();
    }
    if stream.backpressure {
        drop(stream);
        transform_stream_set_backpressure(stream_class, false);
    }
}

pub(super) fn transform_stream_set_backpressure<'js>(
    stream_class: &TransformStreamClass<'js>,
    backpressure: bool,
) {
    let mut stream = stream_class.borrow_mut();
    if let Some(ref bp_promise) = stream.backpressure_change_promise {
        bp_promise.resolve_undefined();
    }
    stream.backpressure = backpressure;
}

pub(super) fn transform_stream_default_controller_perform_transform<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    chunk: Value<'js>,
) -> Result<Promise<'js>> {
    let controller = controller_class.borrow();
    let algorithm = controller
        .transform_algorithm
        .clone()
        .expect("transform algorithm must exist");
    drop(controller);

    let promise_primordials = crate::utils::promise::PromisePrimordials::get(&ctx)?.clone();

    let transform_promise = match algorithm {
        TransformAlgorithm::Identity => {
            let result =
                transform_stream_default_controller_enqueue(ctx.clone(), stream_class, chunk);
            promise_resolved_with(
                &ctx,
                &promise_primordials,
                result.map(|_| Value::new_undefined(ctx.clone())),
            )?
        },
        TransformAlgorithm::Function { f, transformer } => {
            let result: Result<Value> =
                f.call((This(transformer), chunk, controller_class.clone()));
            promise_resolved_with(&ctx, &promise_primordials, result)?
        },
    };

    Ok(transform_promise)
}

pub(super) fn perform_flush<'js>(
    ctx: Ctx<'js>,
    _stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
) -> Result<Promise<'js>> {
    let controller = controller_class.borrow();
    let algorithm = controller
        .flush_algorithm
        .clone()
        .unwrap_or(FlushAlgorithm::Noop);
    drop(controller);

    let promise_primordials = crate::utils::promise::PromisePrimordials::get(&ctx)?.clone();

    match algorithm {
        FlushAlgorithm::Noop => Ok(promise_primordials.promise_resolved_with_undefined.clone()),
        FlushAlgorithm::Function { f, transformer } => {
            let result: Result<Value> = f.call((This(transformer), controller_class.clone()));
            promise_resolved_with(&ctx, &promise_primordials, result)
        },
    }
}

pub(super) fn perform_cancel<'js>(
    ctx: Ctx<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    reason: Value<'js>,
) -> Result<Promise<'js>> {
    let controller = controller_class.borrow();
    let algorithm = controller
        .cancel_algorithm
        .clone()
        .unwrap_or(CancelAlgorithm::Noop);
    drop(controller);

    let promise_primordials = crate::utils::promise::PromisePrimordials::get(&ctx)?.clone();

    match algorithm {
        CancelAlgorithm::Noop => Ok(promise_primordials.promise_resolved_with_undefined.clone()),
        CancelAlgorithm::Function { f, transformer } => {
            let result: Result<Value> = f.call((This(transformer), reason));
            promise_resolved_with(&ctx, &promise_primordials, result)
        },
    }
}
