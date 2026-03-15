use llrt_utils::option::Undefined;
use rquickjs::{
    class::Trace,
    prelude::{Opt, This},
    Class, Ctx, Exception, JsLifetime, Object, Promise, Result, Value,
};

use crate::{
    queuing_strategy::QueuingStrategy,
    readable::stream::{
        algorithms::{CancelAlgorithm, PullAlgorithm, StartAlgorithm},
        ReadableStream,
    },
    utils::promise::ResolveablePromise,
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
    pub(super) writable: Option<Class<'js, crate::writable::WritableStream<'js>>>,
    pub(super) controller: Option<TransformStreamDefaultControllerClass<'js>>,
    pub(super) backpressure: bool,
    pub(super) backpressure_change_promise: Option<ResolveablePromise<'js>>,
}

pub(crate) type TransformStreamClass<'js> = Class<'js, TransformStream<'js>>;

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TransformStream<'js> {
    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        transformer: Opt<Undefined<Object<'js>>>,
        writable_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
        readable_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
    ) -> Result<Class<'js, Self>> {
        let transformer_obj = transformer.0.and_then(|u| u.0);
        let transformer_dict = match transformer_obj {
            Some(ref obj) => Transformer::from_object(obj.clone())?,
            None => Transformer::default(),
        };

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
            },
        )?;

        // Initial backpressure change promise
        let bp_promise = ResolveablePromise::new(&ctx)?;
        stream_class.borrow_mut().backpressure_change_promise = Some(bp_promise);

        // Build controller algorithms
        let transform_algorithm = match transformer_dict.transform {
            Some(f) => TransformAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            },
            None => TransformAlgorithm::Identity,
        };
        let flush_algorithm = match transformer_dict.flush {
            Some(f) => FlushAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            },
            None => FlushAlgorithm::Noop,
        };
        let cancel_algorithm = match transformer_dict.cancel {
            Some(f) => TsCancelAlgorithm::Function {
                f,
                transformer: transformer_obj.clone(),
            },
            None => TsCancelAlgorithm::Noop,
        };

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
        let writable_class = crate::writable::WritableStream::create_for_transform(
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

        // Invoke start() if present
        if let Some(start_fn) = transformer_dict.start {
            match start_fn.call::<_, Value>((This(transformer_obj), controller_class)) {
                Ok(val) => {
                    start_promise.resolve(val);
                },
                Err(_) => {
                    let err = ctx.catch();
                    start_promise.reject(err);
                },
            }
        } else {
            start_promise.resolve_undefined();
        }

        Ok(stream_class)
    }

    #[qjs(get)]
    fn readable(&self) -> Option<Class<'js, ReadableStream<'js>>> {
        self.readable.clone()
    }

    #[qjs(get)]
    fn writable(&self) -> Option<Class<'js, crate::writable::WritableStream<'js>>> {
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
    if stream.backpressure {
        let bp_promise = stream
            .backpressure_change_promise
            .as_ref()
            .map(|p| p.promise.clone());
        drop(stream);

        if let Some(bp_promise) = bp_promise {
            let sc = stream_class.clone();
            let cc = controller_class.clone();
            return crate::utils::promise::upon_promise::<Value<'js>, _>(
                ctx.clone(),
                bp_promise,
                move |ctx, _| {
                    let p = controller::transform_stream_default_controller_perform_transform(
                        ctx.clone(),
                        &sc,
                        &cc,
                        chunk,
                    )?;
                    Ok(p.into_value())
                },
            );
        }
    } else {
        drop(stream);
    }

    controller::transform_stream_default_controller_perform_transform(
        ctx,
        stream_class,
        controller_class,
        chunk,
    )
}

pub(crate) fn sink_close_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
) -> Result<Promise<'js>> {
    let flush_promise = controller::perform_flush(ctx.clone(), stream_class, controller_class)?;

    let sc = stream_class.clone();
    let cc = controller_class.clone();
    crate::utils::promise::upon_promise::<Value<'js>, _>(
        ctx.clone(),
        flush_promise,
        move |ctx, result| {
            cc.borrow_mut().clear_algorithms();
            match result {
                Ok(_) => {
                    let mut stream = sc.borrow_mut();
                    // Resolve any pending backpressure promise to break the cycle
                    if let Some(ref bp) = stream.backpressure_change_promise {
                        let _ = bp.resolve_undefined();
                    }
                    stream.backpressure_change_promise = None;
                    if let Some(ref readable) = stream.readable {
                        if let Some(controller_class) = {
                            let r = readable.borrow();
                            match &r.controller {
                                crate::readable::ReadableStreamControllerClass::ReadableStreamDefaultController(c) => Some(c.clone()),
                                _ => None,
                            }
                        } {
                            let _ =
                                crate::readable::readable_stream_default_controller_close_stream(
                                    ctx.clone(),
                                    controller_class,
                                );
                        }
                    }
                    Ok(Value::new_undefined(ctx))
                },
                Err(r) => {
                    controller::transform_stream_error(ctx.clone(), &sc, r.clone())?;
                    Err(ctx.throw(r))
                },
            }
        },
    )
}

pub(crate) fn sink_abort_algorithm<'js>(
    ctx: Ctx<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    reason: Value<'js>,
) -> Result<Promise<'js>> {
    let cancel_promise = controller::perform_cancel(ctx.clone(), controller_class, reason)?;

    let cc = controller_class.clone();
    crate::utils::promise::upon_promise::<Value<'js>, _>(
        ctx.clone(),
        cancel_promise,
        move |ctx, result| {
            cc.borrow_mut().clear_algorithms();
            match result {
                Ok(_) => Ok(Value::new_undefined(ctx)),
                Err(r) => Err(ctx.throw(r)),
            }
        },
    )
}

// --- Source algorithms ---

pub(crate) fn source_pull_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
) -> Result<Promise<'js>> {
    let mut stream = stream_class.borrow_mut();

    if let Some(ref old_bp) = stream.backpressure_change_promise {
        old_bp.resolve_undefined();
    }

    let new_bp = ResolveablePromise::new(&ctx)?;
    let return_promise = new_bp.promise.clone();
    stream.backpressure_change_promise = Some(new_bp);
    stream.backpressure = false;

    Ok(return_promise)
}

pub(crate) fn source_cancel_algorithm<'js>(
    ctx: Ctx<'js>,
    stream_class: &TransformStreamClass<'js>,
    controller_class: &TransformStreamDefaultControllerClass<'js>,
    reason: Value<'js>,
) -> Result<Promise<'js>> {
    let cancel_promise = controller::perform_cancel(ctx.clone(), controller_class, reason.clone())?;

    let sc = stream_class.clone();
    let cc = controller_class.clone();
    crate::utils::promise::upon_promise::<Value<'js>, _>(
        ctx.clone(),
        cancel_promise,
        move |ctx, result| {
            cc.borrow_mut().clear_algorithms();
            controller::transform_stream_error_writable_and_unblock_write(&sc, reason);
            match result {
                Ok(_) => Ok(Value::new_undefined(ctx)),
                Err(r) => Err(ctx.throw(r)),
            }
        },
    )
}
