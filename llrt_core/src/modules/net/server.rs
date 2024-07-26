// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

use llrt_modules::stream::impl_stream_events;
use rquickjs::{
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, Object, Result, Undefined, Value,
};
use tokio::{
    net::{TcpListener, UnixListener},
    select,
    sync::broadcast::{self, Sender},
};

use super::{get_address_parts, get_hostname, socket::Socket, Listener, NetStream};
use crate::{
    modules::events::{EmitError, Emitter, EventEmitter, EventList},
    stream::SteamEvents,
    utils::{object::ObjectExt, result::ResultExt},
    vm::CtxExtension,
};

impl_stream_events!(Server);

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
                let listener = TcpListener::bind(get_hostname(
                    &host.unwrap_or_else(|| String::from("0.0.0.0")),
                    port as u16,
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
