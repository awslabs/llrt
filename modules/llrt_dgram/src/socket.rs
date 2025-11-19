// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventKey, EventList};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, FromJs, Function, IntoJs, JsLifetime, Object, Result, Value,
};
use tokio::{net::UdpSocket, sync::broadcast};
use tracing::trace;

#[rquickjs::class]
pub struct Socket<'js> {
    emitter: EventEmitter<'js>,
    socket: Option<Arc<UdpSocket>>,
    is_bound: Arc<AtomicBool>,
    is_closed: Arc<AtomicBool>,
    local_address: Option<String>,
    local_port: Option<u16>,
    local_family: Option<String>,
    receiver_running: Arc<AtomicBool>,
    close_tx: broadcast::Sender<()>,
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

    fn on_event_changed(&mut self, _event: EventKey<'js>, _added: bool) -> Result<()> {
        Ok(())
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Socket<'js> {
    #[qjs(constructor)]
    pub fn ctor(ctx: Ctx<'js>, type_or_options: Value<'js>) -> Result<Class<'js, Self>> {
        let socket_type = if let Some(obj) = type_or_options.as_object() {
            obj.get_optional::<_, String>("type")?
                .unwrap_or_else(|| "udp4".to_string())
        } else if let Some(type_str) = type_or_options.as_string() {
            type_str.to_string()?
        } else {
            "udp4".to_string()
        };

        // Validate socket type
        if socket_type != "udp4" && socket_type != "udp6" {
            return Err(Exception::throw_type(
                &ctx,
                &format!("Invalid socket type: {}", socket_type),
            ));
        }

        let emitter = EventEmitter::new();
        let (close_tx, _) = broadcast::channel(1);

        let instance = Self {
            emitter,
            socket: None,
            is_bound: Arc::new(AtomicBool::new(false)),
            is_closed: Arc::new(AtomicBool::new(false)),
            local_address: None,
            local_port: None,
            local_family: Some(if socket_type == "udp4" {
                "IPv4".to_string()
            } else {
                "IPv6".to_string()
            }),
            receiver_running: Arc::new(AtomicBool::new(false)),
            close_tx,
        };

        Class::instance(ctx, instance)
    }

    pub fn bind(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        let mut port = 0u16;
        let mut address = "0.0.0.0".to_string();
        let mut callback: Option<Function> = None;

        // Parse arguments: can be (port, address, callback), (port, callback), (callback), or (options, callback)
        let mut args_iter = args.0.into_iter();

        if let Some(first_arg) = args_iter.next() {
            if let Some(func) = first_arg.as_function() {
                // bind(callback)
                callback = Some(func.clone());
            } else if let Some(num) = first_arg.as_int() {
                // bind(port, ...)
                port = num as u16;
                if let Some(second_arg) = args_iter.next() {
                    if let Some(func) = second_arg.as_function() {
                        // bind(port, callback)
                        callback = Some(func.clone());
                    } else if let Some(addr_str) = second_arg.as_string() {
                        // bind(port, address, ...)
                        address = addr_str.to_string()?;
                        if let Some(third_arg) = args_iter.next() {
                            if let Some(func) = third_arg.as_function() {
                                // bind(port, address, callback)
                                callback = Some(func.clone());
                            }
                        }
                    }
                }
            } else if let Some(obj) = first_arg.as_object() {
                // bind(options, callback)
                if let Some(p) = obj.get::<_, Option<u16>>("port")? {
                    port = p;
                }
                if let Some(addr) = obj.get::<_, Option<String>>("address")? {
                    address = addr;
                }
                if let Some(second_arg) = args_iter.next() {
                    if let Some(func) = second_arg.as_function() {
                        callback = Some(func.clone());
                    }
                }
            }
        }

        if let Some(cb) = callback {
            Self::add_event_listener_str(
                This(this.clone()),
                &ctx,
                "listening",
                cb,
                true,
                true,
            )?;
        }

        let bind_addr = [&address, ":", &port.to_string()].concat();
        let socket_class = this.clone();
        let is_bound = {
            let borrow = this.borrow();
            borrow.is_bound.clone()
        };
        let is_closed = {
            let borrow = this.borrow();
            borrow.is_closed.clone()
        };
        let receiver_running = {
            let borrow = this.borrow();
            borrow.receiver_running.clone()
        };

        ctx.clone().spawn_exit(async move {
            let socket = UdpSocket::bind(&bind_addr).await.or_throw(&ctx)?;
            let local_addr = socket.local_addr().or_throw(&ctx)?;

            let socket_arc = Arc::new(socket);

            {
                let mut borrow = socket_class.borrow_mut();
                borrow.socket = Some(socket_arc.clone());
                borrow.local_address = Some(local_addr.ip().to_string());
                borrow.local_port = Some(local_addr.port());
            }

            is_bound.store(true, Ordering::SeqCst);

            trace!("UDP socket bound to {}", local_addr);

            // Emit 'listening' event
            Self::emit_str(This(socket_class.clone()), &ctx, "listening", vec![], false)?;

            // Start receiving messages
            if receiver_running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let recv_socket = socket_arc.clone();
                let recv_class = socket_class.clone();
                let recv_closed = is_closed.clone();
                let recv_running = receiver_running.clone();
                let mut close_rx = {
                    let borrow = socket_class.borrow();
                    borrow.close_tx.subscribe()
                };

                ctx.clone().spawn_exit(async move {
                    let mut buf = vec![0u8; 65536];

                    let result: Result<()> = async {
                        loop {
                            tokio::select! {
                                _ = close_rx.recv() => {
                                    // Close signal received, exit loop
                                    break;
                                }
                                result = recv_socket.recv_from(&mut buf) => {
                                    match result {
                                        Ok((size, peer_addr)) => {
                                            let data = Buffer(buf[..size].to_vec()).into_js(&ctx)?;
                                            let info = Object::new(ctx.clone())?;
                                            info.set("address", peer_addr.ip().to_string())?;
                                            info.set("port", peer_addr.port())?;
                                            info.set(
                                                "family",
                                                if peer_addr.is_ipv4() { "IPv4" } else { "IPv6" },
                                            )?;

                                            let info_val: Value = info.into();
                                            Self::emit_str(
                                                This(recv_class.clone()),
                                                &ctx,
                                                "message",
                                                vec![data, info_val],
                                                false,
                                            )?;
                                        },
                                        Err(e) => {
                                            if recv_closed.load(Ordering::SeqCst) {
                                                break;
                                            }
                                            let error_msg = format!("UDP receive error: {}", e);

                                            if Err::<(), _>(Exception::throw_message(&ctx, &error_msg))
                                                .emit_error("message", &ctx, recv_class.clone())?
                                            {
                                                // Error was handled by error listener, continue receiving
                                            } else {
                                                // No error listener, propagate error to spawn_exit
                                                return Err(Exception::throw_message(&ctx, &error_msg));
                                            }
                                        },
                                    }
                                }
                            }
                        }
                        Ok(())
                    }.await;

                    recv_running.store(false, Ordering::SeqCst);
                    result
                })?;
            }

            Result::<()>::Ok(())
        })?;

        Ok(())
    }

    pub fn send(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        msg: Value<'js>,
        port: u16,
        address: Opt<String>,
        callback: Opt<Function<'js>>,
    ) -> Result<()> {
        let address = address.0.unwrap_or_else(|| "localhost".to_string());

        let bytes: Vec<u8> = if let Some(str_val) = msg.as_string() {
            str_val.to_string()?.into_bytes()
        } else {
            ObjectBytes::from_js(&ctx, msg)?.try_into()
                .map_err(|e: std::rc::Rc<str>| Exception::throw_type(&ctx, &e))?
        };

        let socket_arc = {
            let borrow = this.borrow();
            if borrow.is_closed.load(Ordering::SeqCst) {
                return Err(Exception::throw_message(&ctx, "Socket is closed"));
            }
            borrow.socket.clone()
        };

        let socket_class = this.clone();

        ctx.clone().spawn_exit(async move {
            let socket = if let Some(s) = socket_arc {
                s
            } else {
                // Auto-bind if not bound yet
                let bind_addr = "0.0.0.0:0";
                let new_socket = UdpSocket::bind(bind_addr).await.or_throw(&ctx)?;
                let socket_arc = Arc::new(new_socket);

                {
                    let mut borrow = socket_class.borrow_mut();
                    borrow.socket = Some(socket_arc.clone());
                    let local_addr = socket_arc.local_addr().or_throw(&ctx)?;
                    borrow.local_address = Some(local_addr.ip().to_string());
                    borrow.local_port = Some(local_addr.port());
                    borrow.is_bound.store(true, Ordering::SeqCst);
                }

                socket_arc
            };

            let dest_addr = [&address, ":", &port.to_string()].concat();
            let result = socket.send_to(&bytes, &dest_addr).await;

            if let Some(cb) = callback.0 {
                match result {
                    Ok(sent) => {
                        let null_val = Value::new_null(ctx.clone());
                        cb.call::<_, ()>((null_val, sent))?;
                    },
                    Err(e) => {
                        let error_msg = format!("UDP send error: {}", e);
                        let error_val = Exception::from_message(ctx.clone(), &error_msg);
                        cb.call::<_, ()>((error_val,))?;
                    },
                }
            } else {
                result.or_throw(&ctx)?;
            }

            Result::<()>::Ok(())
        })?;

        Ok(())
    }

    pub fn close(this: This<Class<'js, Self>>, ctx: Ctx<'js>, callback: Opt<Function<'js>>) -> Result<()> {
        {
            let borrow = this.borrow();
            if borrow.is_closed.load(Ordering::SeqCst) {
                return Ok(());
            }
            borrow.is_closed.store(true, Ordering::SeqCst);

            // Send close signal to interrupt receive loop immediately
            let _ = borrow.close_tx.send(());
        }

        if let Some(cb) = callback.0 {
            Self::add_event_listener_str(This(this.clone()), &ctx, "close", cb, true, true)?;
        }

        let socket_class = this.clone();

        ctx.clone().spawn_exit(async move {
            // Drop the socket
            {
                let mut borrow = socket_class.borrow_mut();
                borrow.socket = None;
            }

            // Emit 'close' event
            Self::emit_str(This(socket_class), &ctx, "close", vec![], false)?;

            Result::<()>::Ok(())
        })?;

        Ok(())
    }

    pub fn address(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let borrow = this.borrow();

        let obj = Object::new(ctx)?;

        if let Some(addr) = &borrow.local_address {
            obj.set("address", addr.clone())?;
        }
        if let Some(port) = borrow.local_port {
            obj.set("port", port)?;
        }
        if let Some(family) = &borrow.local_family {
            obj.set("family", family.clone())?;
        }

        Ok(obj)
    }

    pub fn unref(this: This<Class<'js, Self>>) -> Result<Class<'js, Self>> {
        // In Node.js, unref() allows the process to exit if this is the only active handle
        // In LLRT's context, this is a no-op but we keep it for API compatibility
        trace!("Socket.unref() called - no-op for API compatibility");
        Ok(this.0)
    }

    #[qjs(rename = "ref")]
    pub fn r#ref(this: This<Class<'js, Self>>) -> Result<Class<'js, Self>> {
        // Counterpart to unref(), also a no-op in LLRT
        trace!("Socket.ref() called - no-op for API compatibility");
        Ok(this.0)
    }
}
