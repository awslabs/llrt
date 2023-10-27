use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rquickjs::{
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Function, Result,
};
use tokio::sync::Notify;

use crate::{util::export_default, vm::CtxExtension};

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
struct Timeout {
    #[qjs(skip_trace)]
    abort: Arc<Notify>,
}

fn clear_timeout(_ctx: Ctx<'_>, timeout: Class<Timeout>) -> Result<()> {
    timeout.borrow().abort.notify_one();
    Ok(())
}

async fn yield_sleep(duration: Duration) {
    if duration.as_millis() == 0 {
        tokio::task::yield_now().await;
        return;
    }
    let start_time = Instant::now();
    while Instant::now() - start_time < duration {
        tokio::task::yield_now().await;
    }
}

fn set_timeout_interval<'js>(
    ctx: Ctx<'js>,
    cb: Function<'js>,
    msec: Option<u64>,
    interval: bool,
) -> Result<Class<'js, Timeout>> {
    let msec = msec.unwrap_or(0);

    let abort = Arc::new(Notify::new());
    let abort_ref = abort.clone();

    ctx.spawn_exit(async move {
        loop {
            let abort = abort_ref.clone();

            let aborted;

            tokio::select! {
                _ = abort.notified() => {
                    aborted = true;
                },
                _ = yield_sleep(Duration::from_millis(msec)) => {
                    aborted = false;
                }
            }

            if !aborted {
                cb.call::<(), ()>(())?;
            }

            if !interval || aborted {
                break;
            }
        }
        drop(cb);
        drop(abort_ref);
        Ok(())
    })?;

    Class::instance(ctx, Timeout { abort })
}

fn set_timeout<'js>(
    ctx: Ctx<'js>,
    cb: Function<'js>,
    msec: Opt<u64>,
) -> Result<Class<'js, Timeout>> {
    set_timeout_interval(ctx, cb, msec.0, false)
}

fn set_interval<'js>(
    ctx: Ctx<'js>,
    cb: Function<'js>,
    msec: Opt<u64>,
) -> Result<Class<'js, Timeout>> {
    set_timeout_interval(ctx, cb, msec.0, true)
}

fn set_immediate(cb: Function) -> Result<()> {
    cb.defer::<()>(())?;
    Ok(())
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

    Class::<Timeout>::register(ctx)?;

    globals.set("setTimeout", Func::from(set_timeout))?;
    globals.set("clearTimeout", Func::from(clear_timeout))?;
    globals.set("setInterval", Func::from(set_interval))?;
    globals.set("clearInterval", Func::from(clear_timeout))?;
    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}
