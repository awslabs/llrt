use llrt_utils::{option::Undefined, primordials::Primordial};
use rquickjs::{
    class::Trace,
    prelude::{Opt, This},
    Class, Ctx, Error, Exception, JsLifetime, Object, Promise, Result, Value,
};

use crate::{
    queuing_strategy::QueuingStrategy,
    readable::stream::ReadableStreamState,
    readable::stream::{
        algorithms::{CancelAlgorithm, PullAlgorithm, StartAlgorithm},
        ReadableStream,
    },
    utils::promise::{PromisePrimordials, ResolveablePromise},
    writable::WritableStream,
};

use super::{
    controller::{
        self, CancelAlgorithm as TsCancelAlgorithm, FlushAlgorithm, TransformAlgorithm,
        TransformStreamDefaultController, TransformStreamDefaultControllerClass,
    },
    transformer::Transformer,
};

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub(crate) struct TransformStream<'js> {
    pub(super) readable: Option<Class<'js, ReadableStream<'js>>>,
    pub(super) writable: Option<Class<'js, WritableStream<'js>>>,
    pub(super) controller: Option<TransformStreamDefaultControllerClass<'js>>,
    pub(super) backpressure: bool,
    pub(super) backpressure_change_promise: Option<ResolveablePromise<'js>>,
    pub(super) cancel_promise: Option<Promise<'js>>,
    pub(super) cancel_error: Option<Value<'js>>,
    pub(super) cancel_in_progress: bool,
    pub(super) cancel_callback_in_progress: bool,
    pub(super) cancel_started: bool,
    pub(super) cancel_override_error: Option<Value<'js>>,
    pub(super) flush_promise: Option<Promise<'js>>,
    pub(super) flush_started: bool,
}

pub(crate) type TransformStreamClass<'js> = Class<'js, TransformStream<'js>>;

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TransformStream<'js> {
    pub(crate) fn from_transformer(
        ctx: Ctx<'js>,
        transformer: Object<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::new(
            ctx,
            Opt(Some(Undefined(Some(transformer)))),
            Opt(None),
            Opt(None),
        )
    }

    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        transformer: Opt<Undefined<Object<'js>>>,
        writable_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
        readable_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
    ) -> Result<Class<'js, Self>> {
        let transformer_obj = transformer.0.and_then(|u| u.0);
        let transformer_dict = transformer_obj
            .as_ref()
            .map(|obj| Transformer::from_object(obj.clone()))
            .transpose()?
            .unwrap_or_default();

        if transformer_dict.readable_type {
            return Err(Exception::throw_range(
                &ctx,
                "readableType is not supported",
            ));
        }
        if transformer_dict.writable_type {
            return Err(Exception::throw_range(
                &ctx,
                "writableType is not supported",
            ));
        }

        let readable_strategy = readable_strategy.0.and_then(|qs| qs.0);
        let writable_strategy = writable_strategy.0.and_then(|qs| qs.0);

        let readable_size = QueuingStrategy::extract_size_algorithm(readable_strategy.as_ref());
        let writable_size = QueuingStrategy::extract_size_algorithm(writable_strategy.as_ref());
        let readable_hwm = QueuingStrategy::extract_high_water_mark(&ctx, readable_strategy, 0.0)?;
        let writable_hwm = QueuingStrategy::extract_high_water_mark(&ctx, writable_strategy, 1.0)?;

        // Create the TransformStream instance
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                readable: None,
                writable: None,
                controller: None,
                backpressure: true,
                backpressure_change_promise: None,
                cancel_promise: None,
                cancel_error: None,
                cancel_in_progress: false,
                cancel_callback_in_progress: false,
                cancel_started: false,
                cancel_override_error: None,
                flush_promise: None,
                flush_started: false,
            },
        )?;

        // Initial backpressure change promise
        let bp_promise = ResolveablePromise::new(&ctx)?;
        stream_class.borrow_mut().backpressure_change_promise = Some(bp_promise);

        // Build controller algorithms
        let transform_algorithm = transformer_dict
            .transform
            .map(|f| TransformAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            })
            .unwrap_or(TransformAlgorithm::Identity);

        let flush_algorithm = transformer_dict
            .flush
            .map(|f| FlushAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            })
            .unwrap_or(FlushAlgorithm::Noop);

        let cancel_algorithm = transformer_dict
            .cancel
            .map(|f| TsCancelAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            })
            .unwrap_or(TsCancelAlgorithm::Noop);

        // Create controller
        let controller_class = Class::instance(
            ctx.clone(),
            TransformStreamDefaultController {
                stream: stream_class.clone(),
                transform_algorithm: Some(transform_algorithm),
                flush_algorithm: Some(flush_algorithm),
                cancel_algorithm: Some(cancel_algorithm),
                finish_promise: None,
            },
        )?;
        stream_class.borrow_mut().controller = Some(controller_class.clone());

        // Start promise
        let start_promise = ResolveablePromise::new(&ctx)?;

        // --- Create writable side with properly traced algorithm variants ---
        let writable_class = WritableStream::create_for_transform(
            ctx.clone(),
            start_promise.promise.clone(),
            stream_class.clone(),
            controller_class.clone(),
            writable_hwm,
            writable_size,
        )?;

        // --- Create readable side ---
        let pull_algorithm = PullAlgorithm::Transform(stream_class.clone());

        let cancel_algo = CancelAlgorithm::Transform {
            stream: stream_class.clone(),
            controller: controller_class.clone(),
        };

        let readable_objects = ReadableStream::create_readable_stream(
            ctx.clone(),
            StartAlgorithm::ReturnUndefined,
            pull_algorithm,
            cancel_algo,
            Some(readable_hwm),
            Some(readable_size),
        )?;

        {
            let mut stream = stream_class.borrow_mut();
            stream.readable = Some(readable_objects.stream.clone());
            stream.writable = Some(writable_class);
        }

        let start_stream = stream_class.clone();
        crate::utils::promise::upon_promise(
            ctx.clone(),
            start_promise.promise.clone(),
            Box::new(move |ctx, result| {
                if let Err(reason) = result {
                    controller::transform_stream_error(ctx.clone(), &start_stream, reason)?;
                }
                Ok(Value::new_undefined(ctx))
            }),
        )?;

        // Invoke start() if present
        if let Some(start_fn) = transformer_dict.start {
            match start_fn.call::<_, Value>((This(transformer_obj), controller_class)) {
                Ok(val) => {
                    start_promise.resolve(val)?;
                },
                Err(_) => {
                    return Err(Error::Exception);
                },
            }
        } else {
            start_promise.resolve_undefined()?;
        }

        Ok(stream_class)
    }

    #[qjs(get)]
    fn readable(&self) -> Option<Class<'js, ReadableStream<'js>>> {
        self.readable.clone()
    }

    #[qjs(get)]
    fn writable(&self) -> Option<Class<'js, WritableStream<'js>>> {
        self.writable.clone()
    }
}

// --- Sink algorithms ---

pub(crate) fn sink_write_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    chunk: Value<'js>,
) -> Result<Promise<'js>> {
    let stream = stream_class.borrow();
    let transform_promise = if stream.backpressure {
        let bp_promise = stream
            .backpressure_change_promise
            .as_ref()
            .map(|p| p.promise.clone());
        drop(stream);

        if let Some(bp_promise) = bp_promise {
            let sc = stream_class.clone();
            let cc = controller_class.clone();
            crate::utils::promise::upon_promise(
                ctx.clone(),
                bp_promise,
                Box::new(move |ctx, result| {
                    if let Err(reason) = result {
                        return Err(ctx.throw(reason));
                    }
                    if let Some(writable) = sc.borrow().writable.clone() {
                        let reason = match &writable.borrow().state {
                            crate::writable::WritableStreamState::Erroring(reason)
                            | crate::writable::WritableStreamState::Errored(reason) => {
                                Some(reason.clone())
                            },
                            _ => None,
                        };
                        if let Some(reason) = reason {
                            return Err(ctx.throw(reason));
                        }
                    }
                    let p = controller::transform_stream_default_controller_perform_transform(
                        ctx.clone(),
                        &sc,
                        &cc,
                        chunk,
                    )?;
                    Ok(p.into_value())
                }),
            )?
        } else {
            controller::transform_stream_default_controller_perform_transform(
                ctx.clone(),
                stream_class,
                controller_class,
                chunk,
            )?
        }
    } else {
        drop(stream);
        controller::transform_stream_default_controller_perform_transform(
            ctx.clone(),
            stream_class,
            controller_class,
            chunk,
        )?
    };

    let sc = stream_class.clone();
    crate::utils::promise::upon_promise(
        ctx.clone(),
        transform_promise,
        Box::new(move |ctx, result| match result {
            Ok(value) => Ok(value),
            Err(reason) => {
                controller::transform_stream_error(ctx.clone(), &sc, reason.clone())?;
                Err(ctx.throw(reason))
            },
        }),
    )
}

pub(crate) fn sink_close_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
) -> Result<Promise<'js>> {
    if let Some(cancel_promise) = stream_class.borrow().cancel_promise.clone() {
        return Ok(cancel_promise);
    }
    if stream_class.borrow().cancel_started {
        let primordials = PromisePrimordials::get(&ctx)?.clone();
        return Ok(primordials.promise_resolved_with_undefined.clone());
    }
    stream_class.borrow_mut().flush_started = true;
    let flush_promise = controller::perform_flush(ctx.clone(), stream_class, controller_class)?;

    let sc = stream_class.clone();
    sc.borrow_mut().flush_promise = Some(flush_promise.clone());
    let cc = controller_class.clone();
    crate::utils::promise::upon_promise(
        ctx.clone(),
        flush_promise,
        Box::new(move |ctx, result| {
            cc.borrow_mut().clear_algorithms();
            match result {
                Ok(_) => {
                    let stored_error = {
                        let stream = sc.borrow();
                        let readable_errored = stream.readable.as_ref().is_some_and(|readable| {
                            matches!(readable.borrow().state, ReadableStreamState::Errored(_))
                        });
                        if readable_errored {
                            stream
                                .writable
                                .as_ref()
                                .and_then(|writable| writable.borrow().stored_error())
                        } else {
                            None
                        }
                    };
                    if let Some(reason) = stored_error {
                        let writable = sc.borrow().writable.clone().unwrap();
                        WritableStream::finish_in_flight_close_with_error(
                            ctx.clone(),
                            writable,
                            reason,
                        )?;
                        return Ok(Value::new_undefined(ctx));
                    }

                    let mut stream = sc.borrow_mut();
                    // Resolve any pending backpressure promise to break the cycle
                    if let Some(ref bp) = stream.backpressure_change_promise {
                        bp.resolve_undefined()?;
                    }
                    stream.backpressure_change_promise = None;
                    let readable_controller = stream.readable.as_ref().and_then(|readable| {
                        let r = readable.borrow();
                        if let crate::readable::ReadableStreamControllerClass::ReadableStreamDefaultController(c) = &r.controller {
                            Some(c.clone())
                        } else {
                            None
                        }
                    });
                    drop(stream);
                    if let Some(c) = readable_controller {
                        crate::readable::readable_stream_default_controller_close_stream(
                            ctx.clone(),
                            c,
                        )?;
                    }
                    Ok(Value::new_undefined(ctx))
                },
                Err(r) => {
                    controller::transform_stream_error(ctx.clone(), &sc, r.clone())?;
                    Err(ctx.throw(r))
                },
            }
        }),
    )
}

pub(crate) fn sink_abort_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    reason: Value<'js>,
) -> Result<Promise<'js>> {
    if let Some(cancel_promise) = stream_class.borrow().cancel_promise.clone() {
        return Ok(cancel_promise);
    }
    if stream_class.borrow().cancel_started {
        stream_class.borrow_mut().cancel_override_error = Some(reason.clone());
        let primordials = PromisePrimordials::get(&ctx)?.clone();
        return crate::utils::promise::promise_rejected_with(&primordials, reason);
    }

    stream_class.borrow_mut().cancel_started = true;
    stream_class.borrow_mut().cancel_in_progress = true;
    let cancel_promise = controller::perform_cancel(ctx.clone(), controller_class, reason.clone())?;

    let sc = stream_class.clone();
    let callback_stream = sc.clone();
    let cc = controller_class.clone();
    let result = crate::utils::promise::upon_promise(
        ctx.clone(),
        cancel_promise,
        Box::new(move |ctx, result| {
            finish_cancel_callback(&callback_stream, &cc);
            match result {
                Ok(_) => {
                    let cancel_error = callback_stream.borrow_mut().cancel_error.take();
                    if let Some(error) = cancel_error {
                        return Err(ctx.throw(error));
                    }
                    controller::transform_stream_error(
                        ctx.clone(),
                        &callback_stream,
                        reason.clone(),
                    )?;
                    Ok(Value::new_undefined(ctx))
                },
                Err(r) => {
                    controller::transform_stream_error(ctx.clone(), &callback_stream, r.clone())?;
                    Err(ctx.throw(r))
                },
            }
        }),
    )?;
    sc.borrow_mut().cancel_promise = Some(result.clone());
    Ok(result)
}

// --- Source algorithms ---

pub(crate) fn source_pull_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
) -> Result<Promise<'js>> {
    controller::transform_stream_set_backpressure(&ctx, stream_class, false)
}

pub(crate) fn source_cancel_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    reason: Value<'js>,
) -> Result<Promise<'js>> {
    if let Some(flush_promise) = stream_class.borrow().flush_promise.clone() {
        return Ok(flush_promise);
    }
    if stream_class.borrow().flush_started {
        let primordials = PromisePrimordials::get(&ctx)?.clone();
        return Ok(primordials.promise_resolved_with_undefined.clone());
    }
    if stream_class.borrow().cancel_started {
        let primordials = PromisePrimordials::get(&ctx)?.clone();
        return Ok(primordials.promise_resolved_with_undefined.clone());
    }

    stream_class.borrow_mut().cancel_started = true;
    stream_class.borrow_mut().cancel_in_progress = true;
    let cancel_promise = controller::perform_cancel(ctx.clone(), controller_class, reason.clone())?;

    let sc = stream_class.clone();
    let callback_stream = sc.clone();
    let cc = controller_class.clone();
    let result = crate::utils::promise::upon_promise(
        ctx.clone(),
        cancel_promise,
        Box::new(move |ctx, result| {
            finish_cancel_callback(&callback_stream, &cc);
            match result {
                Ok(_) => {
                    let cancel_error = callback_stream.borrow_mut().cancel_error.take();
                    if let Some(error) = cancel_error {
                        return Err(ctx.throw(error));
                    }
                    if let Some(error) = callback_stream.borrow_mut().cancel_override_error.take() {
                        return Err(ctx.throw(error));
                    }
                    controller::transform_stream_error_writable_and_unblock_write(
                        &callback_stream,
                        reason,
                    )?;
                    Ok(Value::new_undefined(ctx))
                },
                Err(r) => {
                    controller::transform_stream_error_writable_and_unblock_write(
                        &callback_stream,
                        r.clone(),
                    )?;
                    Err(ctx.throw(r))
                },
            }
        }),
    )?;
    sc.borrow_mut().cancel_promise = Some(result.clone());
    Ok(result)
}

fn finish_cancel_callback<'js>(
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
) {
    controller_class.borrow_mut().clear_algorithms();
    let mut stream = stream_class.borrow_mut();
    stream.cancel_in_progress = false;
    stream.cancel_callback_in_progress = false;
}
