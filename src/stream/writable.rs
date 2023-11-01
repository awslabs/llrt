use std::sync::{Arc, RwLock};

use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Func, Opt, This},
    Class, Ctx, Error, Exception, Function, Result, Value,
};

use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    sync::{
        broadcast::{self, Sender},
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot::Receiver,
    },
};

use crate::{
    events::{EmitError, Emitter, EventEmitter, EventList},
    stream::set_destroyed_and_error,
    util::{get_bytes, ResultExt},
    vm::{CtxExtension, ErrorExtensions},
};

use super::SteamEvents;

pub struct WritableStreamInner<'js> {
    emitter: EventEmitter<'js>,
    command_tx: UnboundedSender<WriteCommand<'js>>,
    command_rx: Option<UnboundedReceiver<WriteCommand<'js>>>,
    is_finished: bool,
    errored: bool,
    emit_close: bool,
    is_destroyed: bool,
    destroy_tx: Sender<Option<Value<'js>>>,
}

impl<'js> WritableStreamInner<'js> {
    pub fn new(emitter: EventEmitter<'js>, emit_close: bool) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let (destroy_tx, _) = broadcast::channel::<Option<Value<'js>>>(1);

        Self {
            command_tx: tx,
            command_rx: Some(rx),
            emitter,
            is_finished: false,
            is_destroyed: false,
            destroy_tx,
            emit_close,
            errored: false,
        }
    }
}

#[derive(Debug)]
pub enum WriteCommand<'js> {
    End,
    Write(Vec<u8>, Option<Function<'js>>, bool),
    Flush,
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct DefaultWritableStream<'js> {
    inner: WritableStreamInner<'js>,
}

impl<'js> Trace<'js> for WritableStreamInner<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
    }
}

impl<'js> Emitter<'js> for DefaultWritableStream<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.inner.emitter.get_event_list()
    }
}

impl<'js> DefaultWritableStream<'js> {
    fn with_emitter(ctx: Ctx<'js>, emitter: EventEmitter<'js>) -> Result<Class<'js, Self>> {
        Class::instance(
            ctx,
            Self {
                inner: WritableStreamInner::new(emitter, true),
            },
        )
    }

    pub fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Self::with_emitter(ctx, EventEmitter::new())
    }
}

impl<'js> WritableStream<'js> for DefaultWritableStream<'js> {
    fn inner_mut(&mut self) -> &mut WritableStreamInner<'js> {
        &mut self.inner
    }

    fn inner(&self) -> &WritableStreamInner<'js> {
        &self.inner
    }
}

pub trait WritableStream<'js>
where
    Self: Emitter<'js> + SteamEvents<'js>,
{
    fn inner_mut(&mut self) -> &mut WritableStreamInner<'js>;

    fn inner(&self) -> &WritableStreamInner<'js>;

    fn add_writable_stream_prototype(ctx: &Ctx<'js>) -> Result<()> {
        let proto = Class::<Self>::prototype(ctx.clone())
            .or_throw_msg(ctx, &format!("Prototype for {} not found", Self::NAME))?;

        proto.set("write", Func::from(Self::write))?;

        proto.set("end", Func::from(Self::end))?;

        Ok(())
    }

    fn destroy(
        this: This<Class<'js, Self>>,
        _ctx: Ctx<'js>,
        error: Opt<Value<'js>>,
    ) -> Class<'js, Self> {
        if !this.borrow().inner().is_finished {
            let mut borrow = this.borrow_mut();
            let inner = borrow.inner_mut();
            inner.is_finished = true;
            inner.is_destroyed = true;
            let tx = inner.destroy_tx.clone();
            drop(borrow);
            //it doesn't matter if channel is closed because then writable is already closed
            let _ = tx.send(error.0);
        }
        this.0
    }

    fn end(this: This<Class<'js, Self>>) -> Class<'js, Self> {
        if !this.borrow().inner().is_finished {
            let mut borrow = this.borrow_mut();
            let inner = borrow.inner_mut();
            inner.is_finished = true;
            let tx = inner.command_tx.clone();
            drop(borrow);
            //it doesn't matter if channel is closed because then writable is already closed
            let _ = tx.send(WriteCommand::End);
        }
        this.0
    }

    fn flush(this: Class<'js, Self>, ctx: &Ctx<'js>) -> Result<()> {
        let _ = this
            .borrow()
            .inner()
            .command_tx
            .send(WriteCommand::Flush)
            .or_throw(ctx);
        Ok(())
    }

    fn write_flushed(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
        cb: Opt<Function<'js>>,
    ) -> Result<()> {
        Self::do_write(this, ctx, value, cb, true)
    }

    fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
        cb: Opt<Function<'js>>,
    ) -> Result<()> {
        Self::do_write(this, ctx, value, cb, false)
    }

    fn do_write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
        cb: Opt<Function<'js>>,
        flush: bool,
    ) -> Result<()> {
        let bytes = get_bytes(&ctx, value)?;

        let callback = cb.0;

        if this
            .borrow()
            .inner()
            .command_tx
            .send(WriteCommand::Write(bytes, callback.clone(), flush))
            .is_err()
        {
            if let Some(cb) = callback {
                let err =
                    Exception::throw_message(&ctx, "This socket has been ended by the other party")
                        .into_value(&ctx)?;

                cb.call((err,))?;
            }
        }

        Ok(())
    }

    fn process<T: AsyncWrite + 'js + Unpin>(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        writable: T,
    ) -> Result<Receiver<bool>> {
        let mut borrow = this.borrow_mut();
        let inner = borrow.inner_mut();
        let is_ended = inner.is_finished;
        let mut is_destroyed = inner.is_destroyed;
        let emit_close = inner.emit_close;
        let mut command_rx = inner
            .command_rx
            .take()
            .expect("rx from writable process already taken!");
        let mut destroy_rx = inner.destroy_tx.subscribe();
        let mut error_value = None;

        drop(borrow);
        let ctx2 = ctx.clone();

        ctx.spawn_exit(async move {
            let ctx3 = ctx2.clone();
            let this2 = this.clone();
            let write_function = async move {
                let mut writer = BufWriter::new(writable);

                if !is_ended && !is_destroyed {
                    loop {
                        tokio::select! {
                            command = command_rx.recv() => {
                                 match command {
                                    Some(WriteCommand::Write(data, cb, flush)) => {
                                        writer.write_all(&data).await.or_throw(&ctx3)?;
                                        if flush {
                                            writer.flush().await.or_throw(&ctx3)?;
                                        }

                                        if let Some(cb) = cb {
                                            cb.call(())?;
                                        }
                                    },
                                    Some(WriteCommand::End) => {
                                        writer.shutdown().await.or_throw(&ctx3)?;
                                        break;
                                    },
                                    Some(WriteCommand::Flush) => writer.flush().await.or_throw(&ctx3)?,
                                    None => break,
                                }
                            },
                            error = destroy_rx.recv() => {
                                set_destroyed_and_error(&mut is_destroyed,  &mut error_value, error);
                                break;
                            }
                        }
                    }
                }

                drop(writer);

                if !is_destroyed {
                    Self::emit_str(This(this2), &ctx3, "finish", vec![], false)?;
                }

                if let Some(error_value) = error_value{
                    return Err(ctx3.throw(error_value));
                }

                Ok::<_, Error>(())
            }
            .await;

            let had_error = write_function.emit_error(&ctx2, this.clone())?;

            if emit_close {
                Self::emit_close(this,&ctx2,had_error)?;
            }

            Ok::<_, Error>(had_error)
        })
    }
}
