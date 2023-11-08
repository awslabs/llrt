use std::{
    mem,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    qjs, Ctx, Exception, Function, Result,
};

use crate::{util::export_default, vm::CtxExtension};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Timeout<'js> {
    cb: Option<Function<'js>>,
    timeout: usize,
    id: usize,
    repeating: bool,
    delay: usize,
}

fn set_immediate(cb: Function) -> Result<()> {
    cb.defer::<()>(())?;
    Ok(())
}

fn get_current_time_millis() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_millis() as usize)
        .unwrap_or(0)
}

fn set_timeout_interval<'js>(
    timeouts: &Arc<Mutex<Vec<Timeout<'js>>>>,
    cb: Function<'js>,
    delay: usize,
    repeating: bool,
) -> usize {
    let timeout = get_current_time_millis() + delay;
    let id = TIMER_ID.fetch_add(1, Ordering::SeqCst);
    timeouts.lock().unwrap().push(Timeout {
        cb: Some(cb),
        timeout,
        id,
        repeating,
        delay,
    });
    id
}

fn clear_timeout_interval(timeouts: &Arc<Mutex<Vec<Timeout>>>, id: usize) {
    let mut timeouts = timeouts.lock().unwrap();
    if let Some(index) = timeouts.iter().position(|t| t.id == id) {
        timeouts.remove(index);
    }
}

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("clearInterval")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        export_default(ctx, exports, |default| {
            let functions = ["setTimeout", "clearTimeout", "setInterval", "clearInterval"];
            for func_name in functions {
                let function: Function = globals.get(func_name)?;
                default.set(func_name, function)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let timeouts = Arc::new(Mutex::new(Vec::<Timeout>::new()));
    let timeouts2 = timeouts.clone();
    let timeouts3 = timeouts.clone();
    let timeouts4 = timeouts.clone();
    let timeouts5 = timeouts.clone();

    globals.set(
        "setTimeout",
        Func::from(move |cb, delay| set_timeout_interval(&timeouts, cb, delay, false)),
    )?;

    globals.set(
        "setInterval",
        Func::from(move |cb, delay| set_timeout_interval(&timeouts2, cb, delay, true)),
    )?;

    globals.set(
        "clearTimeout",
        Func::from(move |id: usize| clear_timeout_interval(&timeouts3, id)),
    )?;

    globals.set(
        "clearInterval",
        Func::from(move |id: usize| clear_timeout_interval(&timeouts4, id)),
    )?;

    let ctx2 = ctx.clone();

    ctx.spawn_exit(async move {
        let raw_ctx = ctx2.as_raw();
        let rt: *mut qjs::JSRuntime = unsafe { qjs::JS_GetRuntime(raw_ctx.as_ptr()) };
        let mut ctx_ptr = mem::MaybeUninit::<*mut qjs::JSContext>::uninit();

        let mut interval = tokio::time::interval(Duration::from_millis(1));
        let mut to_call = Some(Vec::new());
        loop {
            interval.tick().await;
            let mut call_vec = to_call.take().unwrap(); //avoid creating a new vec
            let current_time = get_current_time_millis();
            let mut had_items = false;

            timeouts5.lock().unwrap().retain_mut(|timeout| {
                had_items = true;
                if current_time > timeout.timeout {
                    if !timeout.repeating {
                        //do not clone if not not repeating
                        call_vec.push(timeout.cb.take());
                        return false;
                    }
                    timeout.timeout = current_time + timeout.delay;
                    call_vec.push(timeout.cb.clone());
                }
                true
            });

            for cb in call_vec.iter_mut() {
                if let Some(cb) = cb.take() {
                    cb.call::<(), ()>(())?;
                };
            }

            call_vec.clear();
            to_call.replace(call_vec);

            let result = unsafe { qjs::JS_ExecutePendingJob(rt, ctx_ptr.as_mut_ptr()) };

            if result < 0 {
                let js_context = unsafe { ctx_ptr.assume_init() };
                let ctx_ptr = NonNull::new(js_context)
                    .expect("executing pending job returned a null context on error");

                let ctx = unsafe { Ctx::from_raw(ctx_ptr) };

                let err = ctx.catch();

                if let Some(x) = err.clone().into_object().and_then(Exception::from_object) {
                    Err(x.throw())?;
                } else {
                    Err(ctx.throw(err))?;
                }
            }

            if !had_items && result == 0 {
                break;
            }
        }
        timeouts5.lock().unwrap().clear();

        Ok(())
    })?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}
