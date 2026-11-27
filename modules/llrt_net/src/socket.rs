// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{io, pin::Pin};

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_encoding::Encoder;
use llrt_utils::{
    bytes::{get_lossy_string, ObjectBytes},
    error::ErrorExtensions,
    object::ObjectExt,
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    prelude::{Opt, Rest, This},
    Array, CatchResultExt, Class, Coerced, Ctx, Exception, Function, IntoJs, JsLifetime, Object,
    Result, Value,
};
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    select,
    sync::watch,
};

use super::{address_parts, rethrow_in_task, server::Server, NetStream, LOCALHOST};

const HIGH_WATER_MARK: u32 = 64 * 1024;
const READ_SIZE: usize = 64 * 1024;

type Reader = Pin<Box<dyn AsyncRead>>;
type Writer = Pin<Box<dyn AsyncWrite>>;

#[rquickjs::class]
pub struct Socket<'js> {
    reader: Option<Reader>,
    writer: Option<Writer>,
    has_handle: bool,
    connecting: bool,
    reading: bool,
    read_after_connect: bool,
    write_after_connect: Option<(Vec<u8>, Function<'js>)>,
    final_after_connect: Option<Function<'js>>,
    closed: watch::Sender<bool>,
    server: Option<(Class<'js, Server<'js>>, usize)>,
    local_address: Option<String>,
    local_family: Option<&'static str>,
    local_port: Option<u16>,
    remote_address: Option<String>,
    remote_family: Option<&'static str>,
    remote_port: Option<u16>,
}

unsafe impl<'js> JsLifetime<'js> for Socket<'js> {
    type Changed<'to> = Socket<'to>;
}

impl<'js> Trace<'js> for Socket<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some((_, cb)) = &self.write_after_connect {
            cb.trace(tracer);
        }
        if let Some(cb) = &self.final_after_connect {
            cb.trace(tracer);
        }
        if let Some((server, _)) = &self.server {
            server.trace(tracer);
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Socket<'js> {
    #[qjs(constructor)]
    pub fn ctor(ctx: Ctx<'js>, options: Opt<Value<'js>>) -> Result<Class<'js, Self>> {
        Self::create(&ctx, options.0, None)
    }

    #[qjs(get, enumerable)]
    pub fn connecting(&self) -> bool {
        self.connecting
    }

    #[qjs(get, enumerable)]
    pub fn pending(&self) -> bool {
        !self.has_handle || self.connecting
    }

    #[qjs(get, enumerable)]
    pub fn ready_state(this: This<Class<'js, Self>>) -> Result<&'static str> {
        if this.borrow().connecting {
            return Ok("opening");
        }
        let readable = this.get::<_, Coerced<bool>>("readable")?.0;
        let writable = this.get::<_, Coerced<bool>>("writable")?.0;
        Ok(match (readable, writable) {
            (true, true) => "open",
            (true, false) => "readOnly",
            (false, true) => "writeOnly",
            (false, false) => "closed",
        })
    }

    #[qjs(get, enumerable)]
    pub fn remote_address(&self) -> Option<String> {
        self.remote_address.clone()
    }

    #[qjs(get, enumerable)]
    pub fn local_address(&self) -> Option<String> {
        self.local_address.clone()
    }

    #[qjs(get, enumerable)]
    pub fn remote_family(&self) -> Option<&'static str> {
        self.remote_family
    }

    #[qjs(get, enumerable)]
    pub fn local_family(&self) -> Option<&'static str> {
        self.local_family
    }

    #[qjs(get, enumerable)]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    #[qjs(get, enumerable)]
    pub fn local_port(&self) -> Option<u16> {
        self.local_port
    }

    pub fn connect(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
        let args = args.0;

        if this.get::<_, Coerced<bool>>("destroyed")?.0 {
            return Err(Exception::throw_message(&ctx, "Socket destroyed"));
        }

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
            llrt_net_policy::check_socket_path(&path)
                .map_err(|message| Exception::throw_message(&ctx, &message))?;
        }
        if let Some(port) = port {
            llrt_net_policy::check_endpoint(&host, port)
                .map_err(|message| Exception::throw_message(&ctx, &message))?;
            addr = Some((host, port));
        }

        let this = this.0;

        if let Some(listener) = listener {
            let once: Function = this.get("once")?;
            once.call::<_, ()>((This(this.clone()), "connect", listener))?;
        }

        let mut closed = {
            let mut socket = this.borrow_mut();
            socket.has_handle = true;
            socket.connecting = true;
            socket.closed.subscribe()
        };

        let this2 = this.clone();
        ctx.clone().spawn_exit(async move {
            let connect = async {
                if let Some(path) = path {
                    #[cfg(unix)]
                    {
                        let stream = UnixStream::connect(path).await;
                        return Ok(stream.map(NetStream::Unix));
                    }
                    #[cfg(not(unix))]
                    {
                        _ = path;
                        return Err(Exception::throw_type(
                            &ctx,
                            "Unix domain sockets are not supported on this platform",
                        ));
                    }
                }
                let (host, port) = addr.unwrap();
                let stream = TcpStream::connect((host.as_str(), port)).await;
                Ok(stream.map(NetStream::Tcp))
            };
            let connected = select! {
                biased;
                _ = closed.wait_for(|closed| *closed) => return Ok(()),
                connected = connect => connected,
            };
            let error = match connected.catch(&ctx) {
                Ok(Ok(stream)) => return Self::after_connect(this2, &ctx, stream),
                Ok(Err(error)) => io_error(&ctx, &error)?,
                Err(error) => error.into_value(&ctx)?,
            };
            destroy(&this2, error)
        })?;

        Ok(this)
    }

    #[qjs(rename = "_read")]
    pub fn read_impl(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        let mut socket = this.borrow_mut();
        if socket.connecting || !socket.has_handle {
            socket.read_after_connect = true;
            return Ok(());
        }
        if socket.reading {
            return Ok(());
        }
        let Some(reader) = socket.reader.take() else {
            return Ok(());
        };
        socket.reading = true;
        let closed = socket.closed.subscribe();
        drop(socket);
        Self::read_until_full(this.0, ctx, reader, closed);
        Ok(())
    }

    #[qjs(rename = "_write")]
    pub fn write_impl(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        chunk: Value<'js>,
        encoding: Value<'js>,
        cb: Function<'js>,
    ) -> Result<()> {
        let data = chunk_bytes(&ctx, chunk, encoding)?;
        Self::write_generic(this.0, ctx, data, cb)
    }

    #[qjs(rename = "_writev")]
    pub fn writev_impl(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        chunks: Array<'js>,
        cb: Function<'js>,
    ) -> Result<()> {
        let mut data = Vec::new();
        for entry in chunks.iter::<Object>() {
            let entry = entry?;
            data.extend(chunk_bytes(
                &ctx,
                entry.get("chunk")?,
                entry.get("encoding")?,
            )?);
        }
        Self::write_generic(this.0, ctx, data, cb)
    }

    #[qjs(rename = "_final")]
    pub fn final_impl(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        cb: Function<'js>,
    ) -> Result<()> {
        let mut socket = this.borrow_mut();
        if socket.connecting {
            socket.final_after_connect = Some(cb);
            return Ok(());
        }
        let Some(mut writer) = socket.writer.take() else {
            drop(socket);
            return cb.call(());
        };
        let mut closed = socket.closed.subscribe();
        drop(socket);
        ctx.spawn_exit_simple(async move {
            select! {
                biased;
                _ = closed.wait_for(|closed| *closed) => {},
                _ = writer.shutdown() => {},
            }
            cb.call(())
        });
        Ok(())
    }

    #[qjs(rename = "_destroy")]
    pub fn destroy_impl(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        error: Value<'js>,
        cb: Function<'js>,
    ) -> Result<()> {
        let mut socket = this.borrow_mut();
        let had_handle = socket.has_handle;
        socket.has_handle = false;
        socket.connecting = false;
        socket.reading = false;
        socket.read_after_connect = false;
        socket.reader = None;
        socket.writer = None;
        socket.final_after_connect = None;
        socket.closed.send_replace(true);
        let server = socket.server.take();
        drop(socket);

        let had_error = error.get::<Coerced<bool>>()?.0;
        cb.call::<_, ()>((error,))?;

        if let Some((server, index)) = server {
            Server::connection_closed(&server, &ctx, index)?;
        }
        Function::new(ctx.clone(), close_after_pending_callbacks)?
            .defer((This(this.0), had_handle.then_some(had_error)))
    }
}

impl<'js> Socket<'js> {
    pub(crate) fn create(
        ctx: &Ctx<'js>,
        options: Option<Value<'js>>,
        stream: Option<NetStream>,
    ) -> Result<Class<'js, Self>> {
        let duplex = duplex(ctx)?;

        let opts = Object::new(ctx.clone())?;
        if let Some(options) = options.filter(|options| options.is_object()) {
            let assign: Function = BasePrimordials::get(ctx)?
                .constructor_object
                .get("assign")?;
            assign.call::<_, ()>((opts.clone(), options))?;
        }
        let allow_half_open = opts.get::<_, Coerced<bool>>("allowHalfOpen")?.0;
        opts.set("allowHalfOpen", allow_half_open)?;
        opts.set("emitClose", false)?;
        opts.set("autoDestroy", true)?;
        opts.set("decodeStrings", false)?;
        if is_nullish(&opts.get::<_, Value>("highWaterMark")?) {
            for key in ["readableHighWaterMark", "writableHighWaterMark"] {
                if is_nullish(&opts.get::<_, Value>(key)?) {
                    opts.set(key, HIGH_WATER_MARK)?;
                }
            }
        }

        let (closed, _) = watch::channel(false);
        let this = Class::instance(
            ctx.clone(),
            Self {
                reader: None,
                writer: None,
                has_handle: false,
                connecting: false,
                reading: false,
                read_after_connect: false,
                write_after_connect: None,
                final_after_connect: None,
                closed,
                server: None,
                local_address: None,
                local_family: None,
                local_port: None,
                remote_address: None,
                remote_family: None,
                remote_port: None,
            },
        )?;
        duplex.call::<_, ()>((This(this.clone()), opts))?;
        let on: Function = this.get("on")?;
        let on_end = Function::new(ctx.clone(), on_readable_stream_end)?;
        on.call::<_, ()>((This(this.clone()), "end", on_end))?;

        if let Some(stream) = stream {
            this.borrow_mut().attach(stream);
            let read: Function = this.get("read")?;
            read.call::<_, ()>((This(this.clone()), 0))?;
        }
        Ok(this)
    }

    pub(crate) fn set_server(&mut self, server: Class<'js, Server<'js>>, index: usize) {
        self.server = Some((server, index));
    }

    fn attach(&mut self, stream: NetStream) {
        let (reader, writer): (Reader, Writer) = match stream {
            NetStream::Tcp(stream) => {
                if let Ok(addr) = stream.peer_addr() {
                    let (address, port, family) = address_parts(addr);
                    self.remote_address = Some(address);
                    self.remote_port = Some(port);
                    self.remote_family = Some(family);
                }
                if let Ok(addr) = stream.local_addr() {
                    let (address, port, family) = address_parts(addr);
                    self.local_address = Some(address);
                    self.local_port = Some(port);
                    self.local_family = Some(family);
                }
                let (reader, writer) = stream.into_split();
                (Box::pin(reader), Box::pin(writer))
            },
            #[cfg(unix)]
            NetStream::Unix(stream) => {
                let (reader, writer) = stream.into_split();
                (Box::pin(reader), Box::pin(writer))
            },
        };
        self.reader = Some(reader);
        self.writer = Some(writer);
        self.has_handle = true;
    }

    fn after_connect(this: Class<'js, Self>, ctx: &Ctx<'js>, stream: NetStream) -> Result<()> {
        let (write, final_cb, read) = {
            let mut socket = this.borrow_mut();
            socket.attach(stream);
            socket.connecting = false;
            (
                socket.write_after_connect.take(),
                socket.final_after_connect.take(),
                std::mem::take(&mut socket.read_after_connect),
            )
        };

        let emit: Function = this.get("emit")?;
        emit.call::<_, ()>((This(this.clone()), "connect"))?;

        if let Some((data, cb)) = write {
            Self::write_generic(this.clone(), ctx.clone(), data, cb)?;
        }
        if let Some(cb) = final_cb {
            Self::final_impl(This(this.clone()), ctx.clone(), cb)?;
        }
        if read {
            Self::read_impl(This(this.clone()), ctx.clone())?;
        }

        let is_paused: Function = this.get("isPaused")?;
        if !is_paused.call::<_, bool>((This(this.clone()),))? {
            let read: Function = this.get("read")?;
            read.call::<_, ()>((This(this), 0))?;
        }
        Ok(())
    }

    fn read_until_full(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        mut reader: Reader,
        mut closed: watch::Receiver<bool>,
    ) {
        ctx.clone().spawn_exit_simple(async move {
            let mut buffer = Vec::with_capacity(READ_SIZE);
            loop {
                let result = select! {
                    biased;
                    _ = closed.wait_for(|closed| *closed) => return Ok(()),
                    result = reader.read_buf(&mut buffer) => result,
                };
                let push: Function = this.get("push")?;
                match result {
                    Ok(0) => {
                        this.borrow_mut().reading = false;
                        push.call::<_, ()>((This(this.clone()), rquickjs::Null))?;
                        let read: Function = this.get("read")?;
                        return read.call((This(this), 0));
                    },
                    Ok(_) => {
                        let chunk = Buffer(buffer.clone()).into_js(&ctx)?;
                        buffer.clear();
                        let more = push.call::<_, bool>((This(this.clone()), chunk))?;
                        let mut socket = this.borrow_mut();
                        if !socket.has_handle {
                            return Ok(());
                        }
                        if !more {
                            socket.reading = false;
                            socket.reader = Some(reader);
                            return Ok(());
                        }
                    },
                    Err(error) => {
                        this.borrow_mut().reading = false;
                        let error = io_error(&ctx, &error)?;
                        return destroy(&this, error);
                    },
                }
            }
        });
    }

    fn write_generic(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        data: Vec<u8>,
        cb: Function<'js>,
    ) -> Result<()> {
        let mut socket = this.borrow_mut();
        if socket.connecting {
            socket.write_after_connect = Some((data, cb));
            return Ok(());
        }
        let Some(mut writer) = socket.writer.take() else {
            drop(socket);
            return cb.call((node_error(&ctx, "ERR_SOCKET_CLOSED", "Socket is closed")?,));
        };
        let mut closed = socket.closed.subscribe();
        drop(socket);
        ctx.clone().spawn_exit_simple(async move {
            let result = select! {
                biased;
                _ = closed.wait_for(|closed| *closed) => Ok(()),
                result = writer.write_all(&data) => result,
            };
            {
                let mut socket = this.borrow_mut();
                if socket.has_handle {
                    socket.writer = Some(writer);
                }
            }
            match result {
                Ok(()) => cb.call(()),
                Err(error) => cb.call((io_error(&ctx, &error)?,)),
            }
        });
        Ok(())
    }
}

// Loads `stream` on first use, so that importing `net` stays free.
fn duplex<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    let prototype = Class::<Socket>::prototype(ctx)?.or_throw(ctx)?;
    let parent = prototype.get_prototype().or_throw(ctx)?;
    if parent != BasePrimordials::get(ctx)?.prototype_object {
        return parent.get(PredefinedAtom::Constructor);
    }
    let require: Function = ctx.globals().get("require")?;
    let stream: Object = require.call(("stream",))?;
    let duplex: Function = stream.get("Duplex")?;
    let duplex_prototype: Object = duplex.get(PredefinedAtom::Prototype)?;
    prototype.set_prototype(Some(&duplex_prototype))?;
    let constructor: Object = prototype.get(PredefinedAtom::Constructor)?;
    constructor.set_prototype(Some::<&Object>(&duplex))?;
    Ok(duplex)
}

fn on_readable_stream_end<'js>(this: This<Object<'js>>, ctx: Ctx<'js>) -> Result<()> {
    if !this.get::<_, Coerced<bool>>("allowHalfOpen")?.0 {
        this.set("write", Function::new(ctx, write_after_fin)?)?;
    }
    Ok(())
}

fn write_after_fin<'js>(
    this: This<Object<'js>>,
    ctx: Ctx<'js>,
    chunk: Value<'js>,
    encoding: Opt<Value<'js>>,
    cb: Opt<Value<'js>>,
) -> Result<Value<'js>> {
    if !this.get::<_, Coerced<bool>>("writableEnded")?.0 {
        let write: Function = duplex(&ctx)?
            .get::<_, Object>(PredefinedAtom::Prototype)?
            .get("write")?;
        return write.call((this, chunk, encoding.0, cb.0));
    }
    let cb = match encoding.0 {
        Some(encoding) if encoding.is_function() => Some(encoding),
        _ => cb.0,
    };
    let error = node_error(
        &ctx,
        "EPIPE",
        "This socket has been ended by the other party",
    )?;
    if let Some(cb) = cb.and_then(Value::into_function) {
        cb.defer((error.clone(),))?;
    }
    destroy(&this, error)?;
    Ok(Value::new_bool(ctx, false))
}

fn close_after_pending_callbacks<'js>(
    this: This<Class<'js, Socket<'js>>>,
    ctx: Ctx<'js>,
    had_error: Option<bool>,
) -> Result<()> {
    let emitted = (|| {
        let write = this.borrow_mut().write_after_connect.take();
        if let Some((_, cb)) = write {
            let error = node_error(
                &ctx,
                "ERR_SOCKET_CLOSED_BEFORE_CONNECTION",
                "Socket closed before the connection was established",
            )?;
            cb.call::<_, ()>((error,))?;
        }
        let emit: Function = this.get("emit")?;
        match had_error {
            Some(had_error) => emit.call((This(this.0.clone()), "close", had_error)),
            None => emit.call((This(this.0.clone()), "close")),
        }
    })();
    rethrow_in_task(&ctx, emitted)
}

fn destroy<'js>(this: &Object<'js>, error: Value<'js>) -> Result<()> {
    let destroy: Function = this.get("destroy")?;
    destroy.call((This(this.clone()), error))
}

fn chunk_bytes<'js>(ctx: &Ctx<'js>, chunk: Value<'js>, encoding: Value<'js>) -> Result<Vec<u8>> {
    if chunk.is_string() {
        let encoder = Encoder::for_writable(ctx, Some(encoding))?;
        return encoder
            .decode_from_string(get_lossy_string(chunk)?)
            .or_throw(ctx);
    }
    Ok(ObjectBytes::from(ctx, &chunk)?.as_bytes(ctx)?.to_vec())
}

fn is_nullish(value: &Value<'_>) -> bool {
    value.is_null() || value.is_undefined()
}

fn node_error<'js>(ctx: &Ctx<'js>, code: &str, message: &str) -> Result<Value<'js>> {
    let exception = Exception::from_message(ctx.clone(), message)?;
    exception.set("code", code)?;
    Ok(exception.into_value())
}

fn io_error<'js>(ctx: &Ctx<'js>, error: &io::Error) -> Result<Value<'js>> {
    let exception = Exception::from_message(ctx.clone(), &error.to_string())?;
    if error.kind() == io::ErrorKind::ConnectionRefused {
        exception.set("code", "ECONNREFUSED")?;
    }
    Ok(exception.into_value())
}
