// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{atomic::AtomicUsize, Arc, RwLock};

use llrt_utils::{ctx::CtxExtension, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Func, Opt, This},
    Class, Ctx, Error, IntoJs, Null, Result, Value,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, BufReader},
    sync::{
        broadcast::{self, Sender},
        oneshot::Receiver,
    },
};

use crate::{
    modules::{
        buffer::Buffer,
        events::{EmitError, Emitter, EventEmitter, EventKey, EventList},
    },
    stream::set_destroyed_and_error,
    utils::bytearray_buffer::BytearrayBuffer,
};

use super::{impl_stream_events, SteamEvents, DEFAULT_BUFFER_SIZE};

#[derive(PartialEq, Clone, Debug)]
pub enum ReadableState {
    Init,
    Flowing,
    Paused,
}

#[allow(dead_code)]
pub struct ReadableStreamInner<'js> {
    emitter: EventEmitter<'js>,
    destroy_tx: Sender<Option<Value<'js>>>,
    is_ended: bool,
    is_destroyed: bool,
    errored: bool,
    buffer: BytearrayBuffer,
    emit_close: bool,
    state: ReadableState,
    high_water_mark: AtomicUsize,
    listener: Option<&'static str>,
    data_listener_attached_tx: Sender<()>,
}

impl<'js> Trace<'js> for ReadableStreamInner<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
    }
}

impl<'js> ReadableStreamInner<'js> {
    pub fn on_event_changed(&mut self, event: EventKey<'js>, added: bool) -> Result<()> {
        if let EventKey::String(event) = event {
            match event.as_ref() {
                "data" => {
                    if added {
                        if self.state == ReadableState::Paused {
                            let _ = self.data_listener_attached_tx.send(());
                        }
                        self.state = ReadableState::Flowing;
                        self.listener = Some("data");
                    } else {
                        self.listener = None;
                    }
                },
                "readable" => {
                    if added {
                        self.state = ReadableState::Paused;
                        self.listener = Some("readable");
                    } else {
                        self.listener = None;
                    }
                },
                _ => {},
            }
        }
        Ok(())
    }

    pub fn new(emitter: EventEmitter<'js>, emit_close: bool) -> Self {
        let (destroy_tx, _) = broadcast::channel::<Option<Value<'js>>>(1);
        let (listener_attached_tx, _) = broadcast::channel::<()>(1);
        Self {
            emitter,
            destroy_tx,
            is_ended: false,
            data_listener_attached_tx: listener_attached_tx,
            buffer: BytearrayBuffer::new(DEFAULT_BUFFER_SIZE),
            state: ReadableState::Init,
            high_water_mark: DEFAULT_BUFFER_SIZE.into(),
            listener: None,
            is_destroyed: false,
            emit_close,
            errored: false,
        }
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct DefaultReadableStream<'js> {
    inner: ReadableStreamInner<'js>,
}

impl<'js> DefaultReadableStream<'js> {
    fn with_emitter(ctx: Ctx<'js>, emitter: EventEmitter<'js>) -> Result<Class<'js, Self>> {
        Class::instance(
            ctx,
            Self {
                inner: ReadableStreamInner::new(emitter, true),
            },
        )
    }

    pub fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Self::with_emitter(ctx, EventEmitter::new())
    }
}

impl_stream_events!(DefaultReadableStream);
impl<'js> Emitter<'js> for DefaultReadableStream<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.inner.emitter.get_event_list()
    }

    fn on_event_changed(&mut self, event: EventKey<'js>, added: bool) -> Result<()> {
        self.inner.on_event_changed(event, added)
    }
}
impl<'js> ReadableStream<'js> for DefaultReadableStream<'js> {
    fn inner_mut(&mut self) -> &mut ReadableStreamInner<'js> {
        &mut self.inner
    }

    fn inner(&self) -> &ReadableStreamInner<'js> {
        &self.inner
    }
}

pub trait ReadableStream<'js>
where
    Self: Emitter<'js> + SteamEvents<'js>,
{
    fn inner_mut(&mut self) -> &mut ReadableStreamInner<'js>;

    fn inner(&self) -> &ReadableStreamInner<'js>;

    fn add_readable_stream_prototype(ctx: &Ctx<'js>) -> Result<()> {
        let proto = Class::<Self>::prototype(ctx.clone())
            .or_throw_msg(ctx, &["Prototype for ", Self::NAME, " not found"].concat())?;

        proto.set("read", Func::from(Self::read))?;

        proto.set("destroy", Func::from(Self::destroy))?;

        Ok(())
    }

    fn destroy(
        this: This<Class<'js, Self>>,
        _ctx: Ctx<'js>,
        error: Opt<Value<'js>>,
    ) -> Class<'js, Self> {
        let mut borrow = this.borrow_mut();
        let inner = borrow.inner_mut();
        inner.is_destroyed = true;
        let _ = inner.destroy_tx.send(error.0);
        drop(borrow);
        this.0
    }

    fn read(this: This<Class<'js, Self>>, ctx: Ctx<'js>, size: Opt<usize>) -> Result<Value<'js>> {
        if let Some(data) = this.borrow().inner().buffer.read(size.0) {
            return Buffer(data).into_js(&ctx);
        }

        Ok(Null.into_value(ctx))
    }

    fn drain(this: Class<'js, Self>, ctx: &Ctx<'js>) -> Result<()> {
        let this2 = this.clone();
        let borrow = this2.borrow();
        let inner = borrow.inner();
        let listener = inner.listener;

        if let Some(listener) = listener {
            let ba_buffer = inner.buffer.clone();
            if ba_buffer.len() > 0 {
                drop(borrow);
                let args = match listener {
                    "data" => {
                        let buffer = ba_buffer.read(None).unwrap_or_default();
                        if buffer.is_empty() {
                            return Ok(());
                        }
                        vec![Buffer(buffer).into_js(ctx)?]
                    },
                    "readable" => {
                        vec![]
                    },
                    _ => {
                        vec![]
                    },
                };
                Self::emit_str(This(this), ctx, listener, args, false)?;
            }
        }
        Ok(())
    }

    fn process<T: AsyncRead + 'js + Unpin>(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        readable: T,
    ) -> Result<Receiver<bool>> {
        Self::do_process(this, ctx, readable, || {})
    }

    fn process_callback<T: AsyncRead + 'js + Unpin, C: FnOnce() + Sized + 'js>(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        readable: T,
        on_end: C,
    ) -> Result<Receiver<bool>> {
        Self::do_process(this, ctx, readable, on_end)
    }

    fn do_process<T: AsyncRead + 'js + Unpin, C: FnOnce() + Sized + 'js>(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        readable: T,
        on_end: C,
    ) -> Result<Receiver<bool>> {
        let ctx2 = ctx.clone();
        ctx.spawn_exit(async move {
            let this2 = this.clone();
            let ctx3 = ctx2.clone();

            let borrow = this2.borrow();
            let inner = borrow.inner();
            let mut destroy_rx = inner.destroy_tx.subscribe();
            let is_ended = inner.is_ended;
            let mut is_destroyed = inner.is_destroyed;
            let emit_close = inner.emit_close;

            let mut listener_attached_tx = inner.data_listener_attached_tx.subscribe();
            let ba_buffer = inner.buffer.clone();
            let mut has_data = false;
            drop(borrow);

            let read_function = async move {
                let mut reader: BufReader<T> = BufReader::new(readable);
                let mut buffer = Vec::<u8>::with_capacity(DEFAULT_BUFFER_SIZE);
                let mut last_state = ReadableState::Init;
                let mut error_value = None;

                if !is_ended && !is_destroyed {
                    loop {
                        tokio::select! {
                            result = reader.read_buf(&mut buffer) => {
                                let bytes_read = result.or_throw(&ctx3)?;

                                let mut state = this2.borrow().inner().state.clone();
                                if !has_data && state == ReadableState::Init {
                                    this2.borrow_mut().inner_mut().state = ReadableState::Paused;
                                    state =  ReadableState::Paused;
                                    has_data = true;
                                }

                                match state {
                                    ReadableState::Flowing => {
                                        if last_state == ReadableState::Paused {
                                            if let Some(empty_buffer) = ba_buffer.read(None) {
                                                buffer.extend(empty_buffer);
                                            }
                                        }

                                        if buffer.is_empty() {
                                            break;
                                        }

                                        Self::emit_str(
                                            This(this2.clone()),
                                            &ctx3,
                                            "data",
                                            vec![Buffer(buffer.clone()).into_js(&ctx3)?],
                                            false
                                        )?;
                                        buffer.clear();
                                    },
                                    ReadableState::Paused => {

                                        if bytes_read == 0 {
                                            break;
                                        }

                                        let write_buffer_future = ba_buffer.write(&mut buffer);
                                        Self::emit_str(
                                            This(this2.clone()),
                                            &ctx3,
                                            "readable",
                                            vec![],
                                            false
                                        )?;
                                        tokio::select!{
                                            capacity = write_buffer_future => {
                                                buffer.clear();
                                                //increase buffer capacity if bytearray buffer has more capacity to reduce read syscalls
                                                buffer.reserve(buffer.capacity()-capacity);
                                            }
                                            error = destroy_rx.recv()  => {
                                                set_destroyed_and_error(&mut is_destroyed,  &mut error_value, error);
                                                break;
                                            }
                                            _ = listener_attached_tx.recv() => {
                                                ba_buffer.clear().await
                                                //don't clear buffer
                                            }
                                        }
                                    },
                                    _ => {
                                        //should not happen
                                    }
                                }

                                last_state = state;


                            }
                            error = destroy_rx.recv()  => {
                                set_destroyed_and_error(&mut is_destroyed,  &mut error_value, error);
                                break;
                            },
                        }
                    }
                }

                let mut borrow = this2.borrow_mut();
                let inner = borrow.inner_mut();
                inner.buffer.close().await;
                if is_destroyed {
                    inner.is_destroyed = true;
                } else {
                    inner.is_ended = true;
                }

                drop(borrow);
                drop(reader);

                if !is_destroyed {
                    on_end();
                    Self::emit_str(This(this2), &ctx3, "end", vec![], false)?;
                }

                if let Some(error_value) = error_value{
                    return Err(ctx3.throw(error_value));
                }

                Ok::<_, Error>(())
            }
            .await;

            let had_error = read_function.emit_error(&ctx2, this.clone())?;

            if emit_close {
                Self::emit_close(this,&ctx2,had_error)?;
            }

            Ok::<_, Error>(had_error)
        })
    }
}
