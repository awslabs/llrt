// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, RwLock};

use llrt_events::{Emitter, EventEmitter, EventList};
use llrt_exceptions::{DOMException, DOMExceptionName};
use llrt_utils::mc_oneshot;
use rquickjs::{
    class::{Trace, Tracer},
    function::OnceFn,
    prelude::{Opt, This},
    Array, Class, Ctx, Error, Exception, Function, JsLifetime, Result, Undefined, Value,
};

#[derive(Clone)]
#[rquickjs::class]
pub struct AbortSignal<'js> {
    emitter: EventEmitter<'js>,
    pub aborted: bool,
    reason: Option<Value<'js>>,
    pub sender: mc_oneshot::Sender<Value<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for AbortSignal<'js> {
    type Changed<'to> = AbortSignal<'to>;
}

impl<'js> Trace<'js> for AbortSignal<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(reason) = &self.reason {
            tracer.mark(reason);
        }
        self.emitter.trace(tracer);
        self.sender.trace(tracer);
    }
}

impl<'js> Emitter<'js> for AbortSignal<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> AbortSignal<'js> {
    #[qjs(constructor)]
    pub fn new() -> Self {
        let (sender, _) = mc_oneshot::channel::<Value<'js>>();
        Self {
            emitter: EventEmitter::new(),
            aborted: false,
            reason: None,
            sender,
        }
    }

    #[qjs(get, rename = "onabort")]
    pub fn get_on_abort(&self) -> Option<Function<'js>> {
        Self::get_listeners_str(self, "abort").first().cloned()
    }

    #[qjs(set, rename = "onabort")]
    pub fn set_on_abort(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        listener: Function<'js>,
    ) -> Result<()> {
        Self::add_event_listener_str(this, &ctx, "abort", listener, false, false)?;
        Ok(())
    }

    pub fn remove_on_abort(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        listener: Function<'js>,
    ) -> Result<()> {
        Self::remove_event_listener_str(this, &ctx, "abort", listener)?;
        Ok(())
    }

    pub fn throw_if_aborted(&self, ctx: Ctx<'js>) -> Result<()> {
        if self.aborted {
            return Err(ctx.throw(
                self.reason
                    .clone()
                    .unwrap_or_else(|| Undefined.into_value(ctx.clone())),
            ));
        }
        Ok(())
    }

    #[qjs(static)]
    pub fn any(ctx: Ctx<'js>, signals: Array<'js>) -> Result<Class<'js, Self>> {
        let mut new_signal = AbortSignal::new();

        let mut signal_instances = Vec::with_capacity(signals.len());

        for signal in signals.iter() {
            let signal: Value = signal?;
            let signal: Class<AbortSignal> = Class::from_value(&signal)
                .map_err(|_| Exception::throw_type(&ctx, "Value is not an AbortSignal instance"))?;
            let signal_borrow = signal.borrow();
            if signal_borrow.aborted {
                new_signal.aborted = true;
                new_signal.reason.clone_from(&signal_borrow.reason);
                let new_signal = Class::instance(ctx, new_signal)?;
                return Ok(new_signal);
            } else {
                drop(signal_borrow);
                signal_instances.push(signal);
            }
        }

        let new_signal_instance = Class::instance(ctx.clone(), new_signal)?;
        for signal in signal_instances {
            let signal_instance_2 = new_signal_instance.clone();
            Self::add_event_listener_str(
                This(signal),
                &ctx,
                "abort",
                Function::new(
                    ctx.clone(),
                    OnceFn::from(|ctx, signal| {
                        struct Args<'js>(Ctx<'js>, This<Class<'js, AbortSignal<'js>>>);
                        let Args(ctx, signal) = Args(ctx, signal);
                        let mut borrow = signal_instance_2.borrow_mut();
                        borrow.aborted = true;
                        borrow.reason.clone_from(&signal.borrow().reason);
                        drop(borrow);
                        Self::send_aborted(This(signal_instance_2), ctx)
                    }),
                )?,
                false,
                true,
            )?;
        }

        Ok(new_signal_instance)
    }

    #[qjs(get)]
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    #[qjs(get)]
    pub fn reason(&self) -> Option<Value<'js>> {
        self.reason.clone()
    }

    #[qjs(set, rename = "reason")]
    pub fn set_reason(&mut self, reason: Opt<Value<'js>>) {
        match reason.0 {
            Some(new_reason) if !new_reason.is_undefined() => self.reason.replace(new_reason),
            _ => self.reason.take(),
        };
    }

    #[qjs(skip)]
    pub fn send_aborted(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        let mut borrow = this.borrow_mut();
        borrow.aborted = true;
        let reason = get_reason_or_dom_exception(
            &ctx,
            borrow.reason.as_ref(),
            DOMExceptionName::AbortError,
        )?;
        borrow.reason = Some(reason.clone());
        borrow.sender.send(reason);
        drop(borrow);
        Self::emit_str(this, &ctx, "abort", vec![], false)?;
        Ok(())
    }

    #[qjs(static)]
    pub fn abort(ctx: Ctx<'js>, reason: Opt<Value<'js>>) -> Result<Class<'js, Self>> {
        let mut signal = Self::new();
        signal.set_reason(reason);
        let instance = Class::instance(ctx.clone(), signal)?;
        Self::send_aborted(This(instance.clone()), ctx)?;
        Ok(instance)
    }

    #[qjs(static)]
    pub fn timeout(ctx: Ctx<'js>, milliseconds: u64) -> Result<Class<'js, Self>> {
        let timeout_error =
            get_reason_or_dom_exception(&ctx, None, DOMExceptionName::TimeoutError)?;

        let signal = Self::new();
        let signal_instance = Class::instance(ctx.clone(), signal)?;
        let signal_instance2 = signal_instance.clone();

        let cb = Function::new(
            ctx.clone(),
            OnceFn::from(move |ctx| {
                let mut borrow = signal_instance.borrow_mut();
                borrow.set_reason(Opt(Some(timeout_error)));
                drop(borrow);
                Self::send_aborted(This(signal_instance), ctx)?;
                Ok::<_, Error>(())
            }),
        )?;

        #[cfg(feature = "sleep-timers")]
        {
            llrt_timers::set_timeout_interval(
                &ctx,
                cb,
                milliseconds,
                llrt_utils::provider::ProviderType::Timeout,
            )?;
        }
        #[cfg(all(not(feature = "sleep-timers"), feature = "sleep-tokio"))]
        {
            use llrt_utils::ctx::CtxExtension;
            ctx.clone().spawn_exit_simple(async move {
                tokio::time::sleep(std::time::Duration::from_millis(milliseconds)).await;
                cb.call::<_, ()>(())?;
                Ok(())
            });
        }
        #[cfg(all(not(feature = "sleep-tokio"), not(feature = "sleep-timers")))]
        {
            compile_error!("Either the `sleep-tokio` or `sleep-timers` feature must be enabled")
        }

        Ok(signal_instance2)
    }
}

fn get_reason_or_dom_exception<'js>(
    ctx: &Ctx<'js>,
    reason: Option<&Value<'js>>,
    name: DOMExceptionName,
) -> Result<Value<'js>> {
    let reason = if let Some(reason) = reason {
        reason.clone()
    } else {
        let ex = DOMException::new_with_name(ctx, name, String::new())?;
        Class::instance(ctx.clone(), ex)?.into_value()
    };
    Ok(reason)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use llrt_test::test_async_with;

    use super::*;

    #[cfg(feature = "sleep-timers")]
    #[tokio::test]
    async fn test_abort_signal() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            llrt_timers::init(&ctx).unwrap();
            Box::pin(async move {
                let signal = AbortSignal::timeout(ctx, 5).unwrap();

                assert!(!signal.borrow().aborted());

                tokio::time::sleep(Duration::from_millis(50)).await;

                assert!(signal.borrow().aborted());
                let reason = signal.borrow().reason().unwrap();
                let reason = Class::<DOMException>::from_value(&reason).unwrap();
                assert_eq!(reason.borrow().name(), "TimeoutError");
            })
        })
        .await;
    }
}
