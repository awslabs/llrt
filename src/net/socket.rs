// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    net::SocketAddr,
    result::Result as StdResult,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use rquickjs::{
    class::{Trace, Tracer},
    module::{Declarations, Exports},
    prelude::{Func, Opt, Rest, This},
    Class, Ctx, Error, Exception, Function, IntoJs, Object, Result, Undefined, Value,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream, UnixStream},
    select,
    sync::{
        broadcast::{self, Sender},
        oneshot::Receiver,
    },
};
use tracing::trace;

use crate::{
    events::{EmitError, Emitter, EventEmitter, EventKey, EventList},
    module::export_default,
    security::ensure_net_access,
    stream::{
        readable::{ReadableStream, ReadableStreamInner},
        writable::{WritableStream, WritableStreamInner},
        SteamEvents,
    },
    utils::{object::ObjectExt, result::ResultExt},
    vm::CtxExtension,
};

#[cfg(unix)]
use tokio::net::UnixListener;

const LOCALHOST: &str = "localhost";

impl_stream_events!(Socket, Server);

#[allow(dead_code)]
enum ReadyState {
    Opening,
    Open,
    Closed,
    ReadOnly,
    WriteOnly,
}

enum NetStream {
    Tcp((TcpStream, SocketAddr)),
    #[cfg(unix)]
    Unix((UnixStream, tokio::net::unix::SocketAddr)),
}
impl NetStream {
    async fn process<'js>(
        self,
        socket: &Class<'js, Socket<'js>>,
        ctx: &Ctx<'js>,
        allow_half_open: bool,
    ) -> Result<bool> {
        let (readable_done, writable_done) = match self {
            NetStream::Tcp((stream, _)) => {
                Socket::process_tcp_stream(socket, ctx, stream, allow_half_open)
            }
            #[cfg(unix)]
            NetStream::Unix((stream, _)) => {
                Socket::process_unix_stream(socket, ctx, stream, allow_half_open)
            }
        }?;
        let had_error = rw_join(ctx, readable_done, writable_done).await?;
        Ok(had_error)
    }
}

enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(UnixListener),
}

impl Listener {
    async fn accept<'js>(&self, ctx: &Ctx<'js>) -> Result<NetStream> {
        match self {
            Listener::Tcp(tcp) => tcp
                .accept()
                .await
                .map(|(stream, addr)| NetStream::Tcp((stream, addr)))
                .or_throw(ctx),
            #[cfg(unix)]
            Listener::Unix(unix) => unix
                .accept()
                .await
                .map(|(stream, addr)| NetStream::Unix((stream, addr)))
                .or_throw(ctx),
        }
    }
}

impl ReadyState {
    pub fn to_string(&self) -> String {
        String::from(match self {
            ReadyState::Opening => "opening",
            ReadyState::Open => "open",
            ReadyState::Closed => "closed",
            ReadyState::ReadOnly => "readOnly",
            ReadyState::WriteOnly => "writeOnly",
        })
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Server<'js> {
    emitter: EventEmitter<'js>,
    address: Value<'js>,
    #[qjs(skip_trace)]
    close_tx: Sender<()>,
    #[qjs(skip_trace)]
    allow_half_open: bool,
}

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

impl<'js> Emitter<'js> for Server<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
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

    pub fn destroy(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        error: Opt<Value<'js>>,
    ) -> Class<'js, Self> {
        ReadableStream::destroy(This(this.clone()), ctx.clone(), Opt(None));
        WritableStream::destroy(This(this.clone()), ctx.clone(), error);

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

        let allow_half_open = this.borrow().allow_half_open;

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
            ensure_net_access(&ctx, &path)?;
        }
        if let Some(port) = port {
            let hostname = format!("{}:{}", host, port);
            ensure_net_access(&ctx, &hostname)?;
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
            let connect = async move {
                let (readable_done, writable_done) = if let Some(path) = path {
                    let stream = UnixStream::connect(path).await.or_throw(&ctx3)?;
                    Self::process_unix_stream(&this2, &ctx3, stream, allow_half_open)
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

            connect.emit_error(&ctx2, this3)?;
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

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Server<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Class<'js, Self>> {
        let mut args_iter = args.0.into_iter();

        let mut connection_listener = None;
        let mut allow_half_open = false;

        if let Some(first) = args_iter.next() {
            if let Some(connection_listener_arg) = first.as_function() {
                connection_listener = Some(connection_listener_arg.clone());
            }
            if let Some(opts_arg) = first.as_object() {
                allow_half_open = opts_arg.get_optional("allowHalfOpen")?.unwrap_or_default();
            }
        }
        if let Some(next) = args_iter.next() {
            connection_listener = next.into_function();
        }

        let emitter = EventEmitter::new();
        let (close_tx, _) = broadcast::channel::<()>(1);

        let instance = Class::instance(
            ctx.clone(),
            Self {
                emitter,
                address: Undefined.into_value(ctx.clone()),
                close_tx,
                allow_half_open,
            },
        )?;

        if let Some(connection_listener) = connection_listener {
            Self::add_event_listener_str(
                This(instance.clone()),
                &ctx,
                "connection",
                connection_listener,
                false,
                false,
            )?;
        }

        Ok(instance)
    }

    pub fn address(&self) -> Value<'js> {
        self.address.clone()
    }

    #[allow(unused_assignments)]
    ///TODO add backlog support
    pub fn listen(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        let mut args_iter = args.0.into_iter();
        let mut port = None;
        let mut path = None;
        let mut host = None;
        #[allow(unused_variables)] //TODO add backlog support
        let mut backlog = None;
        let mut callback = None;

        let borrow = this.borrow();
        let mut close_tx = borrow.close_tx.subscribe();
        let allow_half_open = borrow.allow_half_open;
        drop(borrow);

        if let Some(first) = args_iter.next() {
            if let Some(callback_arg) = first.as_function() {
                callback = Some(callback_arg.clone());
            } else {
                if let Some(port_arg) = first.as_int() {
                    port = Some(port_arg);
                }
                if let Some(path_arg) = first.as_string() {
                    path = Some(path_arg.to_string()?);
                }
                if let Some(opts_arg) = first.as_object() {
                    port = opts_arg.get_optional("port")?;
                    path = opts_arg.get_optional("path")?;
                    host = opts_arg.get_optional("host")?;
                    backlog = opts_arg.get_optional("backlog")?;
                }

                let path = first.into_string();

                if let Some(second) = args_iter.next() {
                    if let Some(callback_arg) = second.as_function() {
                        callback = Some(callback_arg.clone());
                    }
                    if let Some(host_arg) = second.as_string() {
                        host = Some(host_arg.to_string()?);
                    }
                    if path.is_some() {
                        if let Some(backlog_arg) = second.as_int() {
                            backlog = Some(backlog_arg);
                        }
                    }
                    if let Some(third) = args_iter.next() {
                        if let Some(callback_arg) = third.as_function() {
                            callback = Some(callback_arg.clone());
                        }
                        if port.is_some() {
                            if let Some(backlog_arg) = third.as_int() {
                                backlog = Some(backlog_arg);

                                callback = args_iter.next().and_then(|v| v.into_function());
                            }
                        }
                    }
                }
            }
        }

        if let Some(callback) = callback {
            Self::add_event_listener_str(
                This(this.clone()),
                &ctx,
                "listening",
                callback,
                true,
                true,
            )?;
        }

        let ctx2 = ctx.clone();

        let active_connections = Arc::new(AtomicUsize::new(0));

        if port.is_none() && path.is_none() {
            port = Some(0)
        }

        ctx.spawn_exit(async move {
            let listener = if let Some(port) = port {
                let listener = TcpListener::bind(format!(
                    "{}:{}",
                    host.unwrap_or_else(|| String::from("0.0.0.0")),
                    port
                ))
                .await
                .or_throw(&ctx2)?;

                let address_object = Object::new(ctx2.clone())?;

                let (address, port, family) = get_address_parts(&ctx2, listener.local_addr())?;
                address_object.set("address", address)?;
                address_object.set("port", port)?;
                address_object.set("family", family)?;

                this.borrow_mut().address = address_object.into_value();

                Listener::Tcp(listener)
            } else if let Some(path) = path {
                if !cfg!(unix) {
                    return Err(Exception::throw_type(
                        &ctx2,
                        "Unix domain sockets are not supported on this platform",
                    ));
                }

                let listener: UnixListener = UnixListener::bind(path).or_throw(&ctx2)?;

                Listener::Unix(listener)
            } else {
                panic!("unreachable")
            };

            Self::emit_str(This(this.clone()), &ctx2, "listening", vec![], false)?;

            loop {
                let ctx3 = ctx2.clone();
                let this2 = this.clone();

                select! {
                    socket = listener.accept(&ctx3) => {
                        Self::handle_socket_connection(
                            this2.clone(),
                            ctx3.clone(),
                            socket,active_connections.clone(),
                            allow_half_open
                        ).emit_error(&ctx3, this2)?;
                    },
                    _ = close_tx.recv() => {
                        break;
                    }
                }
            }

            Self::emit_str(this, &ctx2, "close", vec![], false)?;

            Ok(())
        })?;

        Ok(())
    }

    fn close(this: This<Class<'js, Self>>, ctx: Ctx<'js>, cb: Opt<Function<'js>>) -> Result<()> {
        if let Some(cb) = cb.0 {
            Self::add_event_listener_str(This(this.clone()), &ctx, "close", cb, true, true)?;
        }
        let _ = this.borrow().close_tx.send(());
        Ok(())
    }
}

impl<'js> Server<'js> {
    fn handle_socket_connection(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        stream_result: Result<NetStream>,
        active_connections: Arc<AtomicUsize>,
        allow_half_open: bool,
    ) -> Result<()> {
        let net_stream = stream_result.or_throw(&ctx)?;

        active_connections.fetch_add(1, Ordering::Relaxed);

        ctx.clone().spawn_exit(async move {
            let socket_instance = Socket::new(ctx.clone(), allow_half_open)?;
            let socket_instance2 = socket_instance.clone().as_value().clone();
            Self::emit_str(
                This(this.clone()),
                &ctx,
                "connection",
                vec![socket_instance2],
                false,
            )?;

            let had_error = net_stream
                .process(&socket_instance, &ctx, allow_half_open)
                .await?;

            Socket::emit_close(socket_instance, &ctx, had_error)?;

            active_connections.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        })?;

        Ok(())
    }
}

fn get_address_parts(
    ctx: &Ctx,
    addr: StdResult<SocketAddr, std::io::Error>,
) -> Result<(String, u16, String)> {
    let addr = addr.or_throw(ctx)?;
    Ok((
        addr.ip().to_string(),
        addr.port(),
        String::from(if addr.is_ipv4() { "IPv4" } else { "IPv6" }),
    ))
}

async fn rw_join<'js>(
    ctx: &Ctx<'js>,
    readable_done: Receiver<bool>,
    writable_done: Receiver<bool>,
) -> Result<bool> {
    let (readable_res, writable_res) = tokio::join!(readable_done, writable_done);
    let had_error = readable_res.or_throw_msg(ctx, "Readable sender dropped")?
        || writable_res.or_throw_msg(ctx, "Writable sender dropped")?;
    Ok(had_error)
}

pub fn declare(declare: &mut Declarations) -> Result<()> {
    declare.declare("createConnection")?;
    declare.declare("connect")?;
    declare.declare("createServer")?;
    declare.declare(stringify!(Socket))?;
    declare.declare(stringify!(Server))?;
    Ok(())
}

pub fn init<'js>(ctx: Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
    export_default(&ctx, exports, |default| {
        Class::<Socket>::define(default)?;
        Class::<Server>::define(default)?;

        Socket::add_event_emitter_prototype(&ctx)?;
        Server::add_event_emitter_prototype(&ctx)?;

        let connect = Func::from(|ctx, args| {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let this = Socket::new(ctx.clone(), false)?;
            Socket::connect(This(this), ctx.clone(), args)
        })
        .into_js(&ctx)?;

        default.set("createConnection", connect.clone())?;
        default.set("connect", connect)?;
        default.set(
            "createServer",
            Func::from(|ctx, args| {
                struct Args<'js>(Ctx<'js>);
                let Args(ctx) = Args(ctx);
                Server::new(ctx.clone(), args)
            }),
        )?;

        Ok(())
    })
}
