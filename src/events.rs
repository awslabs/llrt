// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::mutable_key_type, clippy::for_kv_map)]

use std::sync::{Arc, RwLock};

use rquickjs::{
    class::{JsClass, OwnedBorrow, Trace, Tracer},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest, This},
    CatchResultExt, Class, Ctx, Function, Result, String as JsString, Symbol, Value,
};

use tracing::trace;

use crate::{module::export_default, utils::result::ResultExt, vm::ErrorExtensions};

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

    fn add_event_emitter_prototype(ctx: &Ctx<'js>) -> Result<()> {
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

        Ok(())
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
            }
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
        self.get_event_list()
            .read()
            .unwrap()
            .iter()
            .any(|(k, _)| k == &key)
    }

    fn has_listener(&self, ctx: Ctx<'js>, event: Value<'js>) -> Result<bool> {
        let key = EventKey::from_value(&ctx, event)?;
        Ok(self
            .get_event_list()
            .read()
            .unwrap()
            .iter()
            .any(|(k, _)| k == &key))
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

//TODO implement, abort(reason)
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
struct AbortController {}

#[rquickjs::methods]
impl AbortController {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {}
    }
}

//TODO implement static methods abort() and timeout(miliseconds)
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
struct AbortSignal {}

#[rquickjs::methods]
impl AbortSignal {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {}
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
        export_default(ctx, exports, |default| {
            Class::<EventEmitter>::define(default)?;
            Ok(())
        })?;

        EventEmitter::add_event_emitter_prototype(ctx)?;

        Ok(())
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<AbortController>::define(&globals)?;
    Class::<AbortSignal>::define(&globals)?;

    Ok(())
}
