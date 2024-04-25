// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::mutable_key_type, clippy::for_kv_map)]

use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use rquickjs::{
    class::{JsClass, OwnedBorrow, Trace, Tracer},
    function::OnceFn,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, Rest, This},
    Array, CatchResultExt, Class, Ctx, Exception, Function, Object, Result, String as JsString,
    Symbol, Undefined, Value,
};

use tracing::trace;

use crate::{
    exceptions::DOMException,
    utils::{mc_oneshot, result::ResultExt},
    vm::{CtxExtension, ErrorExtensions},
};

#[derive(Clone, Debug)]
pub enum EventKey<'js> {
    Symbol(Symbol<'js>),
    String(String),
}

impl<'js> EventKey<'js> {
    fn from_value(ctx: &Ctx, value: Value<'js>) -> Result<Self> {
        if value.is_string() {
            let key: String = value.get()?;
            Ok(EventKey::String(key))
        } else {
            let sym = value.into_symbol().ok_or("Not a symbol").or_throw(ctx)?;
            Ok(EventKey::Symbol(sym))
        }
    }
}

impl<'js> Eq for EventKey<'js> {}

impl<'js> PartialEq for EventKey<'js> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EventKey::Symbol(symbol1), EventKey::Symbol(symbol2)) => symbol1 == symbol2,
            (EventKey::String(str1), EventKey::String(str2)) => str1 == str2,
            _ => false,
        }
    }
}

pub struct EventItem<'js> {
    callback: Function<'js>,
    once: bool,
}

pub type EventList<'js> = Vec<(EventKey<'js>, Vec<EventItem<'js>>)>;
pub type Events<'js> = Arc<RwLock<EventList<'js>>>;

#[rquickjs::class]
#[derive(Clone)]
pub struct EventEmitter<'js> {
    pub events: Events<'js>,
}

impl<'js> Emitter<'js> for EventEmitter<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.events.clone()
    }
}

impl<'js> Trace<'js> for EventEmitter<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.trace_event_emitter(tracer);
    }
}

pub trait EmitError<'js> {
    fn emit_error<C>(self, ctx: &Ctx<'js>, this: Class<'js, C>) -> Result<bool>
    where
        C: Emitter<'js>;
}

impl<'js, T> EmitError<'js> for Result<T> {
    fn emit_error<C>(self, ctx: &Ctx<'js>, this: Class<'js, C>) -> Result<bool>
    where
        C: Emitter<'js>,
    {
        if let Err(err) = self.catch(ctx) {
            if this.borrow().has_listener_str("error") {
                let error_value = err.into_value(ctx)?;
                C::emit_str(This(this), ctx, "error", vec![error_value], false)?;
                return Ok(true);
            }
            return Err(err.throw(ctx));
        }
        Ok(false)
    }
}

pub trait Emitter<'js>
where
    Self: JsClass<'js> + Sized + 'js,
{
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>>;

    fn on_event_changed(&mut self, _event: EventKey<'js>, _added: bool) -> Result<()> {
        Ok(())
    }

    fn add_event_emitter_prototype(ctx: &Ctx<'js>) -> Result<Object<'js>> {
        let proto = Class::<Self>::prototype(ctx.clone())
            .or_throw_msg(ctx, "Prototype for EventEmitter not found")?;

        let on = Function::new(ctx.clone(), Self::on)?;
        let off = Function::new(ctx.clone(), Self::remove_event_listener)?;

        proto.set("once", Func::from(Self::once))?;

        proto.set("on", on.clone())?;

        proto.set("emit", Func::from(Self::emit))?;

        proto.set("prependListener", Func::from(Self::prepend_listener))?;

        proto.set(
            "prependOnceListener",
            Func::from(Self::prepend_once_listener),
        )?;

        proto.set("off", off.clone())?;

        proto.set("eventNames", Func::from(Self::event_names))?;

        proto.set("addListener", on)?;

        proto.set("removeListener", off)?;

        Ok(proto)
    }

    fn trace_event_emitter<'a>(&self, tracer: Tracer<'a, 'js>) {
        let events = self.get_event_list();
        let events = events.read().unwrap();
        for (key, items) in events.iter() {
            if let EventKey::Symbol(sym) = &key {
                tracer.mark(sym);
            }

            for item in items {
                tracer.mark(&item.callback);
            }
        }
    }

    fn remove_event_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        let events = this.clone().borrow().get_event_list();
        let mut events = events.write().or_throw(&ctx)?;

        let key = EventKey::from_value(&ctx, event)?;
        if let Some(index) = events.iter_mut().position(|(k, _)| k == &key) {
            let items = &mut events[index].1;
            if let Some(pos) = items.iter().position(|item| item.callback == listener) {
                items.remove(pos);
                if items.is_empty() {
                    events.remove(index);
                }
            }
        };

        Ok(this.0)
    }

    fn add_event_listener_str(
        this: This<Class<'js, Self>>,
        ctx: &Ctx<'js>,
        event: &str,
        listener: Function<'js>,
        prepend: bool,
        once: bool,
    ) -> Result<Class<'js, Self>> {
        let event = to_event(ctx, event)?;
        Self::add_event_listener(this, ctx.clone(), event, listener, prepend, once)
    }

    fn once(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, false, true)
    }

    fn on(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, false, false)
    }

    fn prepend_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, true, false)
    }

    fn prepend_once_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, true, true)
    }

    fn add_event_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
        prepend: bool,
        once: bool,
    ) -> Result<Class<'js, Self>> {
        let this2 = this.clone();
        let events = &this2.borrow().get_event_list();
        let mut events = events.write().or_throw(&ctx)?;
        let key = EventKey::from_value(&ctx, event)?;
        let mut is_new = false;

        let items = match events.iter_mut().find(|(k, _)| k == &key) {
            Some((_, entry_items)) => entry_items,
            None => {
                let new_items = Vec::new();
                is_new = true;
                events.push((key.clone(), new_items));
                &mut events.last_mut().unwrap().1
            },
        };

        let item = EventItem {
            callback: listener,
            once,
        };
        if !prepend {
            items.push(item);
        } else {
            items.insert(0, item);
        }
        if is_new {
            this2.borrow_mut().on_event_changed(key, true)?
        }
        Ok(this.0)
    }

    fn has_listener_str(&self, event: &str) -> bool {
        let key = EventKey::String(String::from(event));
        has_key(self.get_event_list(), key)
    }

    fn has_listener(&self, ctx: Ctx<'js>, event: Value<'js>) -> Result<bool> {
        let key = EventKey::from_value(&ctx, event)?;
        Ok(has_key(self.get_event_list(), key))
    }

    fn get_listeners(&self, ctx: &Ctx<'js>, event: Value<'js>) -> Result<Vec<Function<'js>>> {
        let key = EventKey::from_value(ctx, event)?;
        Ok(find_all_listeners(self.get_event_list(), key))
    }

    fn get_listeners_str(&self, event: &str) -> Vec<Function<'js>> {
        let key = EventKey::String(String::from(event));
        find_all_listeners(self.get_event_list(), key)
    }

    fn do_emit(
        event: Value<'js>,
        this: This<Class<'js, Self>>,
        ctx: &Ctx<'js>,
        args: Rest<Value<'js>>,
        defer: bool,
    ) -> Result<()> {
        trace!("Emitting: {:?}", event);
        let this2 = this.clone();
        let events = &this2.borrow().get_event_list();
        let mut events = events.write().or_throw(ctx)?;
        let key = EventKey::from_value(ctx, event)?;

        if let Some(index) = events.iter_mut().position(|(k, _)| k == &key) {
            let items = &mut events[index].1;
            let mut callbacks = Vec::with_capacity(items.len());
            items.retain(|item: &EventItem<'_>| {
                callbacks.push(item.callback.clone());
                !item.once
            });
            if items.is_empty() {
                events.remove(index);
                this.borrow_mut().on_event_changed(key, false)?;
            }
            drop(events);
            for callback in callbacks {
                let args = args.iter().map(|arg| arg.to_owned()).collect();
                let args = Rest(args);
                let this = This(this.clone());
                if defer {
                    callback.defer((this, args))?;
                } else {
                    callback.call::<_, ()>((this, args))?;
                }
            }
        }

        Ok(())
    }

    fn emit_str(
        this: This<Class<'js, Self>>,
        ctx: &Ctx<'js>,
        event: &str,
        args: Vec<Value<'js>>,
        defer: bool,
    ) -> Result<()> {
        let event = to_event(ctx, event)?;
        Self::do_emit(event, this, ctx, args.into(), defer)
    }

    fn emit(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        Self::do_emit(event, this, &ctx, args, false)
    }

    fn event_names(this: This<OwnedBorrow<'js, Self>>, ctx: Ctx<'js>) -> Result<Vec<Value<'js>>> {
        let events = this.get_event_list();
        let events = events.read().or_throw(&ctx)?;

        let mut names = Vec::with_capacity(events.len());
        for (key, _entry) in events.iter() {
            let value = match key {
                EventKey::Symbol(symbol) => symbol.clone().into_value(),
                EventKey::String(str) => JsString::from_str(ctx.clone(), str)?.into(),
            };

            names.push(value)
        }

        Ok(names)
    }
}

fn find_all_listeners<'js>(
    events: Arc<RwLock<EventList<'js>>>,
    key: EventKey<'js>,
) -> Vec<Function<'js>> {
    let events = events.read().unwrap();
    let items = events.iter().find(|(k, _)| k == &key);
    if let Some((_, callbacks)) = items {
        callbacks.iter().map(|item| item.callback.clone()).collect()
    } else {
        vec![]
    }
}

fn has_key<'js>(event_list: Arc<RwLock<EventList<'js>>>, key: EventKey<'js>) -> bool {
    event_list.read().unwrap().iter().any(|(k, _)| k == &key)
}

fn to_event<'js>(ctx: &Ctx<'js>, event: &str) -> Result<Value<'js>> {
    let event = JsString::from_str(ctx.clone(), event)?;
    Ok(event.into_value())
}

#[rquickjs::methods]
impl<'js> EventEmitter<'js> {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct AbortController<'js> {
    signal: Class<'js, AbortSignal<'js>>,
}

#[rquickjs::methods]
impl<'js> AbortController<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>) -> Result<Self> {
        let signal = AbortSignal::new();

        let abort_controller = Self {
            signal: Class::instance(ctx, signal)?,
        };
        Ok(abort_controller)
    }

    #[qjs(get)]
    pub fn signal(&self) -> Class<'js, AbortSignal<'js>> {
        self.signal.clone()
    }

    pub fn abort(
        ctx: Ctx<'js>,
        this: This<Class<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<()> {
        let instance = this.0.borrow();
        let signal = instance.signal.clone();
        let mut signal_borrow = signal.borrow_mut();
        if signal_borrow.aborted {
            //only once
            return Ok(());
        }
        signal_borrow.set_reason(reason);
        drop(signal_borrow);
        AbortSignal::send_aborted(This(signal), ctx)?;

        Ok(())
    }
}

fn get_reason_or_dom_exception<'js>(
    ctx: &Ctx<'js>,
    reason: Option<&Value<'js>>,
    name: &str,
) -> Result<Value<'js>> {
    let reason = if let Some(reason) = reason {
        reason.clone()
    } else {
        let ex = DOMException::new(ctx.clone(), Opt(None), Opt(Some(name.into())))?;
        Class::instance(ctx.clone(), ex)?.into_value()
    };
    Ok(reason)
}

#[derive(Clone)]
#[rquickjs::class]
pub struct AbortSignal<'js> {
    emitter: EventEmitter<'js>,
    aborted: bool,
    reason: Option<Value<'js>>,
    pub sender: mc_oneshot::Sender<Value<'js>>,
}

impl<'js> Trace<'js> for AbortSignal<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(reason) = &self.reason {
            tracer.mark(reason);
        }
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
        Self::get_listeners_str(&self, "abort")
            .iter()
            .next()
            .map(|e| e.clone())
    }

    #[qjs(set, rename = "onabort")]
    pub fn set_on_abort(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        listener: Function<'js>,
    ) -> Result<()> {
        Self::add_event_listener_str(this, &ctx, "abort", listener, false, true)?;
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
            let signal: Class<AbortSignal> = Class::from_value(signal)
                .map_err(|_| Exception::throw_type(&ctx, "Value is not an AbortSignal instance"))?;
            let signal_borrow = signal.borrow();
            if signal_borrow.aborted {
                new_signal.aborted = true;
                new_signal.reason = signal_borrow.reason.clone();
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
                        borrow.reason = signal.borrow().reason.clone();
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
        if let Some(new_reason) = reason.0 {
            self.reason.replace(new_reason);
        } else {
            self.reason.take();
        }
    }

    #[qjs(skip)]
    pub fn send_aborted(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        let mut borrow = this.borrow_mut();
        borrow.aborted = true;
        let reason = get_reason_or_dom_exception(&ctx, borrow.reason.as_ref(), "AbortError")?;
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
        let timeout_error = get_reason_or_dom_exception(&ctx, None, "TimeoutError")?;

        let signal = Self::new();
        let signal_instance = Class::instance(ctx.clone(), signal)?;
        let signal_instance2 = signal_instance.clone();

        ctx.clone().spawn_exit(async move {
            tokio::time::sleep(Duration::from_millis(milliseconds)).await;
            let mut borrow = signal_instance.borrow_mut();
            borrow.set_reason(Opt(Some(timeout_error)));
            drop(borrow);
            Self::send_aborted(This(signal_instance), ctx)?;
            Ok(())
        })?;

        Ok(signal_instance2)
    }
}

pub struct EventsModule;

impl ModuleDef for EventsModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(EventEmitter))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        let ctor = Class::<EventEmitter>::create_constructor(ctx)?
            .expect("Can't create EventEmitter constructor");
        ctor.set(stringify!(EventEmitter), ctor.clone())?;
        exports.export(stringify!(EventEmitter), ctor.clone())?;
        exports.export("default", ctor)?;

        EventEmitter::add_event_emitter_prototype(ctx)?;

        Ok(())
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<AbortController>::define(&globals)?;
    Class::<AbortSignal>::define(&globals)?;

    AbortSignal::add_event_emitter_prototype(ctx)?;

    Ok(())
}
