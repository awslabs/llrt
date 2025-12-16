// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

use llrt_events::{Emitter, EventEmitter, EventList};
use llrt_utils::primordials::{BasePrimordials, Primordial};
pub use llrt_utils::sysinfo;
use llrt_utils::{
    module::ModuleInfo,
    object::Proxy,
    result::ResultExt,
    signals,
    sysinfo::{ARCH, PLATFORM},
    time, VERSION,
};
use rquickjs::{
    class::{Trace, Tracer},
    convert::Coerced,
    module::{Declarations, Exports, ModuleDef},
    object::{Accessor, Property},
    prelude::{Func, Opt, Rest, This},
    Array, BigInt, Class, Ctx, Error, Exception, Function, IntoJs, Object, Result, Value,
};

#[cfg(unix)]
mod signal_handler;

pub static EXIT_CODE: AtomicU8 = AtomicU8::new(0);

fn cwd(ctx: Ctx<'_>) -> Result<String> {
    env::current_dir()
        .or_throw(&ctx)
        .map(|path| path.to_string_lossy().to_string())
}

fn hr_time_big_int(ctx: Ctx<'_>) -> Result<BigInt<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();

    let elapsed = now.checked_sub(started).unwrap_or_default();

    BigInt::from_u64(ctx, elapsed)
}

fn hr_time(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();
    let elapsed = now.checked_sub(started).unwrap_or_default();

    let seconds = elapsed / 1_000_000_000;
    let remaining_nanos = elapsed % 1_000_000_000;

    let array = Array::new(ctx)?;

    array.set(0, seconds)?;
    array.set(1, remaining_nanos)?;

    Ok(array)
}

fn to_exit_code(ctx: &Ctx<'_>, code: &Value<'_>) -> Result<Option<u8>> {
    if let Ok(code) = code.get::<Coerced<f64>>() {
        let code = code.0;
        let code: u8 = if code.fract() != 0.0 {
            return Err(Exception::throw_range(
                ctx,
                "The value of 'code' must be an integer",
            ));
        } else {
            (code as i32).rem_euclid(256) as u8
        };
        return Ok(Some(code));
    }
    Ok(None)
}

fn exit(ctx: Ctx<'_>, code: Value<'_>) -> Result<()> {
    let code = match to_exit_code(&ctx, &code)? {
        Some(code) => code,
        None => EXIT_CODE.load(Ordering::Relaxed),
    };
    std::process::exit(code.into())
}

fn env_proxy_setter<'js>(
    target: Object<'js>,
    prop: Value<'js>,
    value: Coerced<String>,
) -> Result<bool> {
    target.set(prop, value.to_string())?;
    Ok(true)
}

#[cfg(unix)]
fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

#[cfg(unix)]
fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

#[cfg(unix)]
fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(unix)]
fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

#[cfg(unix)]
fn setuid(id: u32) -> i32 {
    unsafe { libc::setuid(id) }
}

#[cfg(unix)]
fn setgid(id: u32) -> i32 {
    unsafe { libc::setgid(id) }
}

#[cfg(unix)]
fn seteuid(id: u32) -> i32 {
    unsafe { libc::seteuid(id) }
}

#[cfg(unix)]
fn setegid(id: u32) -> i32 {
    unsafe { libc::setegid(id) }
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct Process<'js> {
    emitter: EventEmitter<'js>,
    env: Object<'js>,
    argv: Vec<String>,
    argv0: String,
}

impl<'js> Trace<'js> for Process<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
        self.env.trace(tracer);
    }
}

impl<'js> Emitter<'js> for Process<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Process<'js> {
    #[qjs(get)]
    fn env(&self) -> Object<'js> {
        self.env.clone()
    }

    #[qjs(get)]
    fn argv(&self) -> Vec<String> {
        self.argv.clone()
    }

    #[qjs(get)]
    fn argv0(&self) -> String {
        self.argv0.clone()
    }

    #[qjs(get)]
    fn platform(&self) -> &'static str {
        PLATFORM
    }

    #[qjs(get)]
    fn arch(&self) -> &'static str {
        ARCH
    }

    #[qjs(get)]
    fn version(&self) -> &'static str {
        VERSION
    }

    #[qjs(get)]
    fn id(&self) -> u32 {
        std::process::id()
    }

    fn cwd(ctx: Ctx<'js>) -> Result<String> {
        cwd(ctx)
    }

    fn exit(ctx: Ctx<'js>, code: Opt<Value<'js>>) -> Result<()> {
        let code = code.0.unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        exit(ctx, code)
    }

    fn kill(ctx: Ctx<'js>, pid: u32, signal: Opt<Value<'js>>) -> Result<bool> {
        signals::kill(&ctx, pid, signal)
    }

    // EventEmitter methods
    fn on(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        // Start signal handler if this is a signal event (Unix only)
        #[cfg(unix)]
        if event.is_string() {
            let event_name: String = event.get()?;
            if signal_handler::is_signal_event(&event_name) {
                signal_handler::maybe_start_signal_handler(&ctx, &this.0, &event_name)?;
            }
        }
        Self::add_event_listener(this, ctx, event, listener, false, false)
    }

    fn once(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        // Start signal handler if this is a signal event (Unix only)
        #[cfg(unix)]
        if event.is_string() {
            let event_name: String = event.get()?;
            if signal_handler::is_signal_event(&event_name) {
                signal_handler::maybe_start_signal_handler(&ctx, &this.0, &event_name)?;
            }
        }
        Self::add_event_listener(this, ctx, event, listener, false, true)
    }

    fn off(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        <Self as Emitter>::remove_event_listener(this, ctx, event, listener)
    }

    #[qjs(rename = "addListener")]
    fn add_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, false, false)
    }

    #[qjs(rename = "removeListener")]
    fn remove_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        <Self as Emitter>::remove_event_listener(this, ctx, event, listener)
    }

    #[qjs(rename = "prependListener")]
    fn prepend_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, true, false)
    }

    #[qjs(rename = "prependOnceListener")]
    fn prepend_once_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
    ) -> Result<Class<'js, Self>> {
        Self::add_event_listener(this, ctx, event, listener, true, true)
    }

    fn emit(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<bool> {
        let has_listeners = this.borrow().has_listener_str_from_value(&ctx, &event)?;
        <Self as Emitter>::emit(this, ctx, event, args)?;
        Ok(has_listeners)
    }

    #[qjs(rename = "eventNames")]
    fn event_names(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Vec<Value<'js>>> {
        let events = this.borrow().get_event_list();
        let events = events.read().or_throw(&ctx)?;

        let mut names = Vec::with_capacity(events.len());
        for (key, _entry) in events.iter() {
            let value = match key {
                llrt_events::EventKey::Symbol(symbol) => symbol.clone().into_value(),
                llrt_events::EventKey::String(str) => {
                    rquickjs::String::from_str(ctx.clone(), str)?.into()
                },
            };
            names.push(value)
        }

        Ok(names)
    }

    #[qjs(rename = "listenerCount")]
    fn listener_count(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
    ) -> Result<usize> {
        let key = if event.is_string() {
            let s: String = event.get()?;
            llrt_events::EventKey::String(s.into())
        } else {
            let sym = event.into_symbol().ok_or("Not a symbol").or_throw(&ctx)?;
            llrt_events::EventKey::Symbol(sym)
        };

        let events = this.borrow().get_event_list();
        let events = events.read().or_throw(&ctx)?;

        Ok(events
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, items)| items.len())
            .unwrap_or(0))
    }
}

impl<'js> Process<'js> {
    fn add_event_listener(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        event: Value<'js>,
        listener: Function<'js>,
        prepend: bool,
        once: bool,
    ) -> Result<Class<'js, Self>> {
        <Self as Emitter>::add_event_listener(this, ctx, event, listener, prepend, once)
    }

    fn has_listener_str_from_value(&self, ctx: &Ctx<'js>, event: &Value<'js>) -> Result<bool> {
        if event.is_string() {
            let s: String = event.get()?;
            Ok(self.has_listener_str(&s))
        } else {
            self.has_listener(ctx.clone(), event.clone())
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    BasePrimordials::init(ctx)?;

    // Create environment proxy
    let env_map: HashMap<String, String> = env::vars().collect();
    let mut args: Vec<String> = env::args().collect();

    if let Some(arg) = args.get(1) {
        if arg == "-e" || arg == "--eval" {
            args.remove(1);
            args.remove(1);
        }
    }

    let env_obj = env_map.into_js(ctx)?;
    let env_proxy = Proxy::with_target(ctx.clone(), env_obj)?;
    env_proxy.setter(Func::from(env_proxy_setter))?;
    let env_proxy_obj: Object = env_proxy.into_js(ctx)?.into_object().unwrap();

    let argv0 = args.first().cloned().unwrap_or_default();

    // Create process instance
    let process = Process {
        emitter: EventEmitter::new(),
        env: env_proxy_obj,
        argv: args,
        argv0,
    };

    let process_class = Class::instance(ctx.clone(), process)?;

    // Add additional properties that aren't simple getters
    let process_obj = process_class
        .as_object()
        .expect("Process class should be an object");

    // versions object
    let process_versions = Object::new(ctx.clone())?;
    process_versions.set("llrt", VERSION)?;
    process_versions.set("node", "0.0.0")?;
    process_obj.set("versions", process_versions)?;

    // hrtime function with bigint method
    let hr_time_fn = Function::new(ctx.clone(), hr_time)?;
    hr_time_fn.set("bigint", Func::from(hr_time_big_int))?;
    process_obj.set("hrtime", hr_time_fn)?;

    // release object
    let release = Object::new(ctx.clone())?;
    release.prop("name", Property::from("llrt").enumerable())?;
    process_obj.set("release", release)?;

    // exitCode accessor
    process_obj.prop(
        "exitCode",
        Accessor::new(
            |ctx| {
                struct Args<'js>(Ctx<'js>);
                let Args(ctx) = Args(ctx);
                ctx.globals().get::<_, Value>("__exitCode")
            },
            |ctx, code| {
                struct Args<'js>(Ctx<'js>, Value<'js>);
                let Args(ctx, code) = Args(ctx, code);
                if let Some(code) = to_exit_code(&ctx, &code)? {
                    EXIT_CODE.store(code, Ordering::Relaxed);
                }
                ctx.globals().set("__exitCode", code)?;
                Ok::<_, Error>(())
            },
        )
        .configurable()
        .enumerable(),
    )?;

    // Unix-specific methods - added directly to the object because #[cfg(unix)]
    // on individual methods inside #[rquickjs::methods] doesn't work correctly
    // with the proc macro on Windows
    #[cfg(unix)]
    {
        process_obj.set("getuid", Func::from(getuid))?;
        process_obj.set("getgid", Func::from(getgid))?;
        process_obj.set("geteuid", Func::from(geteuid))?;
        process_obj.set("getegid", Func::from(getegid))?;
        process_obj.set("setuid", Func::from(setuid))?;
        process_obj.set("setgid", Func::from(setgid))?;
        process_obj.set("seteuid", Func::from(seteuid))?;
        process_obj.set("setegid", Func::from(setegid))?;
    }

    globals.set("process", process_class)?;

    Ok(())
}

pub struct ProcessModule;

impl ModuleDef for ProcessModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("env")?;
        declare.declare("cwd")?;
        declare.declare("argv0")?;
        declare.declare("id")?;
        declare.declare("argv")?;
        declare.declare("platform")?;
        declare.declare("arch")?;
        declare.declare("hrtime")?;
        declare.declare("release")?;
        declare.declare("version")?;
        declare.declare("versions")?;
        declare.declare("exitCode")?;
        declare.declare("exit")?;
        declare.declare("kill")?;
        declare.declare("on")?;
        declare.declare("once")?;
        declare.declare("off")?;
        declare.declare("addListener")?;
        declare.declare("removeListener")?;
        declare.declare("emit")?;
        declare.declare("eventNames")?;
        declare.declare("listenerCount")?;

        #[cfg(unix)]
        {
            declare.declare("getuid")?;
            declare.declare("getgid")?;
            declare.declare("geteuid")?;
            declare.declare("getegid")?;
            declare.declare("setuid")?;
            declare.declare("setgid")?;
            declare.declare("seteuid")?;
            declare.declare("setegid")?;
        }

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let process: Class<Process> = globals.get("process")?;

        // Export the process object directly as default
        // This allows prototype methods to be accessed correctly
        exports.export("default", process.clone())?;

        // Also export individual named exports for destructuring imports
        // We need to get these from the object including prototype chain
        let process_obj = process
            .as_object()
            .expect("Process class should be an object");

        // Export commonly used properties/methods
        let names = [
            "env",
            "cwd",
            "argv0",
            "argv",
            "platform",
            "arch",
            "hrtime",
            "release",
            "version",
            "versions",
            "exitCode",
            "exit",
            "kill",
            "on",
            "once",
            "off",
            "addListener",
            "removeListener",
            "emit",
            "eventNames",
            "listenerCount",
            "id",
            #[cfg(unix)]
            "getuid",
            #[cfg(unix)]
            "getgid",
            #[cfg(unix)]
            "geteuid",
            #[cfg(unix)]
            "getegid",
            #[cfg(unix)]
            "setuid",
            #[cfg(unix)]
            "setgid",
            #[cfg(unix)]
            "seteuid",
            #[cfg(unix)]
            "setegid",
        ];

        for name in names {
            if let Ok(value) = process_obj.get::<&str, Value>(name) {
                exports.export(name, value)?;
            }
        }

        Ok(())
    }
}

impl From<ProcessModule> for ModuleInfo<ProcessModule> {
    fn from(val: ProcessModule) -> Self {
        ModuleInfo {
            name: "process",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_hr_time() {
        time::init();
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<ProcessModule>(ctx.clone(), "process")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { hrtime } from 'process';

                        export async function test() {
                            // TODO: Delaying with setTimeout
                            for(let i=0; i < (1<<20); i++){}
                            return hrtime()

                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Vec<u32>, _>(&ctx, &module, ()).await;
                assert_eq!(result.len(), 2);
                assert_eq!(result[0], 0);
                assert!(result[1] > 0);
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hr_time_bigint() {
        time::init();
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<ProcessModule>(ctx.clone(), "process")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { hrtime } from 'process';

                        export async function test() {
                            // TODO: Delaying with setTimeout
                            for(let i=0; i < (1<<20); i++){}
                            return hrtime.bigint()

                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Coerced<i64>, _>(&ctx, &module, ()).await;
                assert!(result.0 > 0);
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_process_on() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        export async function test() {
                            let called = false;
                            process.on('test-event', () => {
                                called = true;
                            });
                            process.emit('test-event');
                            return called;
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<bool, _>(&ctx, &module, ()).await;
                assert!(result);
            })
        })
        .await;
    }
}
