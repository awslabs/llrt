// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, RwLock};

use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventKey, EventList};
use llrt_stream::{
    impl_stream_events,
    readable::{ReadableStream, ReadableStreamInner},
    writable::{WritableStream, WritableStreamInner},
    SteamEvents,
};
use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Opt, Rest, This},
    Class, Ctx, Error, Exception, Function, JsLifetime, Object, Result, Value,
};
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::oneshot::Receiver,
};
use tracing::trace;

use super::{ensure_access, get_address_parts, get_hostname, rw_join, ReadyState, LOCALHOST};

impl_stream_events!(Socket);

#[rquickjs::class]
#[allow(dead_code)]
pub struct Socket<'js> {
    emitter: EventEmitter<'js>,
    readable_stream_inner: ReadableStreamInner<'js>,
    writable_stream_inner: WritableStreamInner<'js>,
    connecting: bool,
    destroyed: bool,
    pending: bool,
    local_address: Option<String>,
    local_family: Option<String>,
    local_port: Option<u16>,
    remote_address: Option<String>,
    remote_family: Option<String>,
    remote_port: Option<u16>,
    ready_state: ReadyState,
    allow_half_open: bool,
}

unsafe impl<'js> JsLifetime<'js> for Socket<'js> {
    type Changed<'to> = Socket<'to>;
}

impl<'js> Trace<'js> for Socket<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
    }
}

impl<'js> Emitter<'js> for Socket<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }

    fn on_event_changed(&mut self, event: EventKey<'js>, added: bool) -> Result<()> {
        self.readable_stream_inner.on_event_changed(event, added)
    }
}

impl<'js> ReadableStream<'js> for Socket<'js> {
    fn inner_mut(&mut self) -> &mut ReadableStreamInner<'js> {
        &mut self.readable_stream_inner
    }

    fn inner(&self) -> &ReadableStreamInner<'js> {
        &self.readable_stream_inner
    }
}

impl<'js> WritableStream<'js> for Socket<'js> {
    fn inner_mut(&mut self) -> &mut WritableStreamInner<'js> {
        &mut self.writable_stream_inner
    }

    fn inner(&self) -> &WritableStreamInner<'js> {
        &self.writable_stream_inner
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Socket<'js> {
    #[qjs(constructor)]
    pub fn ctor(ctx: Ctx<'js>, opts: Opt<Object<'js>>) -> Result<Class<'js, Self>> {
        let mut allow_half_open = false;
        if let Some(opts) = opts.0 {
            if let Some(opt_allow_half_open) = opts.get_optional("allowHalfOpen")? {
                allow_half_open = opt_allow_half_open;
            }
        }

        Self::new(ctx, allow_half_open)
    }

    #[qjs(get, enumerable)]
    pub fn connecting(&self) -> bool {
        self.connecting
    }

    #[qjs(get, enumerable)]
    pub fn pending(&self) -> bool {
        self.pending
    }

    #[qjs(get, enumerable)]
    pub fn remote_address(&self) -> Option<String> {
        self.remote_address.clone()
    }

    pub fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
        cb: Opt<Function<'js>>,
    ) -> Result<()> {
        WritableStream::write_flushed(this, ctx.clone(), value, cb)?;
        Ok(())
    }

    pub fn end(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        callback: Opt<Function<'js>>,
    ) -> Result<()> {
        if let Some(cb) = callback.0 {
            Self::add_event_listener_str(This(this.clone()), &ctx, "end", cb, true, true)?;
        }

        //ReadableStream::destroy(This(this.clone()), ctx.clone())?;
        WritableStream::end(this);

        Ok(())
    }

    pub fn destroy(this: This<Class<'js, Self>>, error: Opt<Value<'js>>) -> Class<'js, Self> {
        this.borrow_mut().destroyed = true;
        ReadableStream::destroy(This(this.clone()), Opt(None));
        WritableStream::destroy(This(this.clone()), error);
        this.0
    }

    pub fn read(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        size: Opt<usize>,
    ) -> Result<Value<'js>> {
        ReadableStream::read(this, ctx, size)
    }

    #[qjs(get, enumerable)]
    pub fn local_address(&self) -> Option<String> {
        self.local_address.clone()
    }

    #[qjs(get, enumerable)]
    pub fn remote_family(&self) -> Option<String> {
        self.remote_family.clone()
    }

    #[qjs(get, enumerable)]
    pub fn local_family(&self) -> Option<String> {
        self.local_family.clone()
    }

    #[qjs(get, enumerable)]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    #[qjs(get, enumerable)]
    pub fn local_port(&self) -> Option<u16> {
        self.local_port
    }

    #[qjs(get, enumerable)]
    pub fn ready_state(&self) -> String {
        self.ready_state.to_string()
    }

    pub fn connect(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
        let args = args.0;

        let borrow = this.borrow();
        let allow_half_open = borrow.allow_half_open;
        if borrow.destroyed {
            return Err(Exception::throw_message(&ctx, "Socket destroyed"));
        }
        drop(borrow);

        let mut port = None;
        let mut host = String::from(LOCALHOST);
        let mut path = None;
        let mut listener = None;
        let mut last = None;
        let mut addr = None;

        let mut args = args.into_iter();

        if let Some(first) = args.next() {
            if let Some(opts) = first.as_object() {
                port = opts.get_optional("port")?;
                path = opts.get_optional("path")?;
                if let Some(host_arg) = opts.get_optional("host")? {
                    host = host_arg
                }
            } else if let Some(path_arg) = first.as_string() {
                path = Some(path_arg.to_string()?);
            } else if let Some(port_arg) = first.as_int() {
                port = Some(port_arg as u16);
                if let Some(next) = args.next() {
                    if let Some(host_arg) = next.as_string() {
                        host = host_arg.to_string()?;
                    } else {
                        last = Some(next)
                    }
                }
            }
        }

        if let Some(last) = last.or_else(|| args.next()) {
            if let Some(cb) = last.as_function() {
                listener = Some(cb.to_owned());
            }
        }

        if path.is_none() && port.is_none() {
            return Err(Exception::throw_type(&ctx, "port or path are required"));
        }

        if let Some(path) = path.clone() {
            ensure_access(&ctx, &path)?;
        }
        if let Some(port) = port {
            let hostname = get_hostname(&host, port);
            ensure_access(&ctx, &hostname)?;
            addr = Some(hostname);
        }

        let this = this.0;

        let this2 = this.clone();

        if let Some(listener) = listener {
            Socket::add_event_listener_str(
                This(this.clone()),
                &ctx,
                "connect",
                listener,
                false,
                true,
            )?;
        }

        ctx.clone().spawn_exit(async move {
            let ctx2 = ctx.clone();
            let ctx3 = ctx.clone();
            let this3 = this2.clone();
            if this3.borrow().destroyed {
                Socket::emit_close(this3.clone(), &ctx3, false)?;
                return Ok(());
            }
            let connect = async move {
                let (readable_done, writable_done) = if let Some(path) = path {
                    #[cfg(unix)]
                    {
                        let stream = UnixStream::connect(path).await.or_throw(&ctx3)?;
                        Self::process_unix_stream(&this2, &ctx3, stream, allow_half_open)
                    }
                    #[cfg(not(unix))]
                    {
                        _ = path;
                        return Err(Exception::throw_type(
                            &ctx3,
                            "Unix domain sockets are not supported on this platform",
                        ));
                    }
                } else if let Some(addr) = addr {
                    let stream = TcpStream::connect(addr).await.or_throw(&ctx3)?;
                    Self::process_tcp_stream(&this2, &ctx3, stream, allow_half_open)
                } else {
                    unreachable!()
                }?;

                Socket::emit_str(This(this2.clone()), &ctx3, "connect", vec![], false)?;

                let had_error = rw_join(&ctx3, readable_done, writable_done).await?;

                Socket::emit_close(this2, &ctx3, had_error)?;

                Ok::<_, Error>(())
            }
            .await;

            connect.emit_error("connect", &ctx2, this3)?;
            Ok(())
        })?;

        Ok(this)
    }
}

impl<'js> Socket<'js> {
    pub fn new(ctx: Ctx<'js>, allow_half_open: bool) -> Result<Class<'js, Self>> {
        let emitter = EventEmitter::new();

        let readable_stream_inner = ReadableStreamInner::new(emitter.clone(), false);
        let writable_stream_inner = WritableStreamInner::new(emitter.clone(), false);

        let instance = Class::instance(
            ctx,
            Self {
                emitter,
                connecting: false,
                destroyed: false,
                pending: true,
                ready_state: ReadyState::Closed,
                local_address: None,
                local_family: None,
                local_port: None,
                remote_address: None,
                remote_family: None,
                remote_port: None,
                readable_stream_inner,
                writable_stream_inner,
                allow_half_open,
            },
        )?;
        Ok(instance)
    }

    pub fn process_tcp_stream(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        stream: TcpStream,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)> {
        Self::set_addresses(this, ctx, &stream)?;

        let (reader, writer) = stream.into_split();
        Self::process_stream(this, ctx, reader, writer, allow_half_open)
    }

    #[cfg(unix)]
    pub fn process_unix_stream(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        stream: UnixStream,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)> {
        let (reader, writer) = stream.into_split();
        Self::process_stream(this, ctx, reader, writer, allow_half_open)
    }

    fn process_stream<R: AsyncRead + 'js + Unpin, W: AsyncWrite + 'js + Unpin>(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        reader: R,
        writer: W,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)> {
        let this2 = this.clone();
        let readable_done =
            ReadableStream::process_callback(this.clone(), ctx, reader, move || {
                if !allow_half_open {
                    WritableStream::end(This(this2));
                }
            })?;
        let writable_done = WritableStream::process(this.clone(), ctx, writer)?;

        trace!("Connected to stream");
        let mut borrow = this.borrow_mut();
        borrow.connecting = false;
        borrow.pending = false;
        borrow.ready_state = ReadyState::Open;
        drop(borrow);

        Ok((readable_done, writable_done))
    }

    pub fn set_addresses<'a>(
        this: &'a Class<'js, Self>,
        ctx: &Ctx<'js>,
        stream: &TcpStream,
    ) -> Result<()> {
        let mut borrow = this.borrow_mut();

        let (remote_address, remote_port, remote_family) =
            get_address_parts(ctx, stream.peer_addr())?;
        borrow.remote_address = Some(remote_address);
        borrow.remote_port = Some(remote_port);
        borrow.remote_family = Some(remote_family);

        let (local_address, local_port, local_family) =
            get_address_parts(ctx, stream.local_addr())?;
        borrow.local_address = Some(local_address);
        borrow.local_port = Some(local_port);
        borrow.local_family = Some(local_family);

        drop(borrow);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use llrt_buffer as buffer;
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};
    use rand::Rng;
    use rquickjs::{function::IntoArgs, module::Evaluated, Ctx, FromJs, Module};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use crate::NetModule;

    async fn server(port: u16) {
        let listerner = TcpListener::bind(("127.0.0.1", port)).await.unwrap();

        let (mut stream, _) = listerner.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();

        // Read
        let mut buf = vec![0; 1024];
        let n = stream.read(&mut buf).await.unwrap();

        // Write
        stream.write_all(&buf[..n]).await.unwrap();
        stream.flush().await.unwrap();
    }

    async fn call_test_delay<'js, T, A>(
        ctx: &Ctx<'js>,
        module: &Module<'js, Evaluated>,
        args: A,
    ) -> T
    where
        T: FromJs<'js>,
        A: IntoArgs<'js>,
    {
        tokio::time::sleep(Duration::from_millis(100)).await;
        call_test::<T, _>(ctx, module, args).await
    }

    #[tokio::test]
    async fn test_server_echo() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<NetModule>(ctx.clone(), "net")
                    .await
                    .unwrap();

                let mut rng = rand::rng();
                let port: u16 = rng.random_range(49152..=65535);

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { connect } from 'net';

                        export async function test(port) {
                            const socket = connect({ port });
                            const txData = "Hello World";
                            return new Promise((resolve, reject) => {
                                socket.on('connect', () => {
                                    socket.write(txData, (err) => {
                                        if (err) {
                                            reject(err);
                                        }
                                    });
                                });
                                socket.on('data', (rxData) => {
                                    resolve(rxData.toString() === txData);
                                });
                            });
                        }
                    "#,
                )
                .await
                .unwrap();

                let (ok, _) = tokio::join!(
                    call_test_delay::<bool, _>(&ctx, &module, (port,)),
                    server(port)
                );
                assert!(ok)
            })
        })
        .await;
    }
}
