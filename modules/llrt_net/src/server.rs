// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    future::poll_fn,
    io,
    sync::{Arc, RwLock},
    task::{Poll, Waker},
};

use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventList};
use llrt_utils::{object::ObjectExt, result::ResultExt, reuse_list::ReuseList};
#[cfg(unix)]
use rquickjs::IntoJs;
use rquickjs::{
    class::Trace,
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Result, Undefined, Value,
};
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tracing::trace;

use super::{address_parts, rethrow_in_task, socket::Socket, Listener, NetStream};

#[rquickjs::class]
pub struct Server<'js> {
    emitter: EventEmitter<'js>,
    address: Value<'js>,
    allow_half_open: bool,
    listening: bool,
    listening_id: u32,
    listener: Option<Listener>,
    accept_waker: Option<Waker>,
    sockets: ReuseList<Class<'js, Socket<'js>>>,
}

impl<'js> Trace<'js> for Server<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
        self.address.trace(tracer);
        for socket_ref in self.sockets.iter() {
            socket_ref.trace(tracer);
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for Server<'js> {
    type Changed<'to> = Server<'to>;
}

impl<'js> Emitter<'js> for Server<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
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

        let instance = Class::instance(
            ctx.clone(),
            Self {
                emitter: EventEmitter::new(),
                address: Undefined.into_value(ctx.clone()),
                allow_half_open,
                listening: false,
                listening_id: 0,
                listener: None,
                accept_waker: None,
                sockets: ReuseList::with_capacity(8),
            },
        )?;

        if let Some(connection_listener) = connection_listener {
            Self::add_event_listener_str(
                instance.clone(),
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

    pub fn get_connections(&self, cb: Opt<Function<'js>>) -> Result<()> {
        if let Some(cb) = cb.0 {
            cb.call::<_, ()>((Undefined, self.sockets.len()))?;
        }
        Ok(())
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

        if this.borrow().listening {
            return Err(Exception::throw_message(&ctx, "ERR_SERVER_ALREADY_LISTEN"));
        }

        if let Some(first) = args_iter.next() {
            if let Some(callback_arg) = first.as_function() {
                callback = Some(callback_arg.clone());
            } else {
                if let Some(port_arg) = first.as_int() {
                    if port_arg > 0xFFFF {
                        return Err(Exception::throw_range(
                            &ctx,
                            "port should be between 0 and 65535",
                        ));
                    }
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
            Self::add_event_listener_str(this.clone(), &ctx, "listening", callback, true, true)?;
        }

        let ctx2 = ctx.clone();

        if port.is_none() && path.is_none() {
            port = Some(0)
        }

        let listening_id = {
            let mut server = this.borrow_mut();
            server.listening = true;
            server.listening_id = server.listening_id.wrapping_add(1);
            server.listening_id
        };

        ctx.spawn_exit(async move {
            let listener = match Self::bind(this.clone(), ctx2.clone(), port, host, path).await {
                Ok(listener) => listener,
                Err(e) => {
                    let mut server = this.borrow_mut();
                    if server.listening_id == listening_id {
                        server.listening = false;
                    }
                    drop(server);
                    Err::<(), _>(e).emit_error("listen", &ctx2, this.clone())?;
                    return Ok(()); // Don't stop the VM if failed to bind
                },
            };

            {
                let mut server = this.borrow_mut();
                if server.listening_id != listening_id {
                    return Ok(());
                }
                server.listener = Some(listener);
            }

            Self::emit_str(this.clone(), &ctx2, "listening", vec![], false)?;

            while let Some(accepted) = poll_fn(|cx| {
                let mut server = this.borrow_mut();
                let server = &mut *server;
                let Some(listener) = &server.listener else {
                    return Poll::Ready(None);
                };
                let accepted = listener.poll_accept(cx).map(Some);
                if accepted.is_pending() {
                    server.accept_waker = Some(cx.waker().clone());
                }
                accepted
            })
            .await
            {
                Self::handle_connection(&this, &ctx2, accepted).emit_error(
                    "handle_socket_connection",
                    &ctx2,
                    this.clone(),
                )?;
            }

            Ok(())
        })?;

        Ok(())
    }

    fn close(this: This<Class<'js, Self>>, ctx: Ctx<'js>, cb: Opt<Function<'js>>) -> Result<()> {
        trace!("Closing server");
        if let Some(cb) = cb.0 {
            Self::add_event_listener_str(this.clone(), &ctx, "close", cb, false, true)?;
        }
        let mut server = this.borrow_mut();
        if !server.listening {
            return Ok(());
        }
        server.listening = false;
        server.listening_id = server.listening_id.wrapping_add(1);
        server.listener = None;
        if let Some(waker) = server.accept_waker.take() {
            waker.wake();
        }
        drop(server);
        Self::emit_close_if_drained(&this, &ctx)
    }
}

impl<'js> Server<'js> {
    async fn bind(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        port: Option<i32>,
        host: Option<String>,
        path: Option<String>,
    ) -> Result<Listener> {
        let listener = if let Some(port) = port {
            let host = host.unwrap_or_else(|| String::from("0.0.0.0"));
            let listener = TcpListener::bind((host.as_str(), port as u16))
                .await
                .or_throw(&ctx)?;

            let address_object = Object::new(ctx.clone())?;

            let (address, port, family) = address_parts(listener.local_addr().or_throw(&ctx)?);
            address_object.set("address", address)?;
            address_object.set("port", port)?;
            address_object.set("family", family)?;

            this.borrow_mut().address = address_object.into_value();

            Listener::Tcp(listener)
        } else if let Some(path) = path {
            #[cfg(unix)]
            {
                let listener: UnixListener = UnixListener::bind(&path).or_throw(&ctx)?;
                this.borrow_mut().address = path.into_js(&ctx)?;
                Listener::Unix(listener)
            }
            #[cfg(not(unix))]
            {
                _ = path;
                return Err(Exception::throw_type(
                    &ctx,
                    "Unix domain sockets are not supported on this platform",
                ));
            }
        } else {
            panic!("unreachable")
        };
        Ok(listener)
    }

    fn handle_connection(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        accepted: io::Result<NetStream>,
    ) -> Result<()> {
        let stream = accepted.or_throw(ctx)?;
        let options = Object::new(ctx.clone())?;
        options.set("allowHalfOpen", this.borrow().allow_half_open)?;
        let socket = Socket::create(ctx, Some(options.into_value()), Some(stream))?;
        let index = this.borrow_mut().sockets.append(socket.clone());
        socket.borrow_mut().set_server(this.clone(), index);
        Self::emit_str(
            this.clone(),
            ctx,
            "connection",
            vec![socket.into_value()],
            false,
        )
    }

    pub(crate) fn connection_closed(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        index: usize,
    ) -> Result<()> {
        this.borrow_mut().sockets.remove(index);
        Self::emit_close_if_drained(this, ctx)
    }

    fn emit_close_if_drained(this: &Class<'js, Self>, ctx: &Ctx<'js>) -> Result<()> {
        let server = this.borrow();
        if server.listening || !server.sockets.is_empty() {
            return Ok(());
        }
        drop(server);
        Function::new(ctx.clone(), Self::emit_close)?.defer((This(this.clone()),))
    }

    fn emit_close(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        let emitted = Self::emit_str(this.0, &ctx, "close", vec![], false);
        rethrow_in_task(&ctx, emitted)
    }
}
