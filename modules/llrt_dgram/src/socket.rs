// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventKey, EventList};
use llrt_utils::{bytes::ObjectBytes, latch::Latch, object::ObjectExt, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, FromJs, Function, IntoJs, JsLifetime, Object, Result, Value,
};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc, oneshot},
};
use tracing::trace;

type SendResult = std::result::Result<usize, String>;
type SendMessage = (Vec<u8>, String, Option<oneshot::Sender<SendResult>>);

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
    send_tx: Option<mpsc::UnboundedSender<SendMessage>>,
    ready_latch: Arc<Latch>,
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
            send_tx: None,
            ready_latch: Arc::new(Latch::default()),
        };

        Class::instance(ctx, instance)
    }

    fn start_listening(
        socket_class: Class<'js, Self>,
        ctx: Ctx<'js>,
        bind_addr: String,
    ) -> Result<()> {
        // Check state and get Arc clones in single borrow
        let (is_closed, receiver_running, ready_latch) = {
            let borrow = socket_class.borrow();
            if borrow.is_bound.load(Ordering::SeqCst) {
                return Err(Exception::throw_message(&ctx, "ERR_SOCKET_ALREADY_BOUND"));
            }
            if borrow.is_closed.load(Ordering::SeqCst) {
                return Err(Exception::throw_message(
                    &ctx,
                    "ERR_SOCKET_DGRAM_NOT_RUNNING",
                ));
            }
            (
                borrow.is_closed.clone(),
                borrow.receiver_running.clone(),
                borrow.ready_latch.clone(),
            )
        };

        // Increment latch before spawning - will be decremented when send_tx is ready or on error
        ready_latch.increment();

        let socket_class2 = socket_class.clone();
        let socket_class3 = socket_class.clone();
        ctx.clone().spawn_exit(async move {
            let bind_result = async {
                let socket = match UdpSocket::bind(&bind_addr).await {
                    Ok(s) => s,
                    Err(e) => {
                        ready_latch.decrement();
                        return Err(e).or_throw(&ctx);
                    },
                };
                let local_addr = match socket.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        ready_latch.decrement();
                        return Err(e).or_throw(&ctx);
                    },
                };

                let socket_arc = Arc::new(socket);

                // Create MPSC channel for send operations BEFORE starting loops
                let (send_tx, mut send_rx) = mpsc::unbounded_channel::<SendMessage>();

                let (recv_close_rx, send_close_rx) = {
                    let mut borrow = socket_class.borrow_mut();
                    borrow.socket = Some(socket_arc.clone());
                    borrow.local_address = Some(local_addr.ip().to_string());
                    borrow.local_port = Some(local_addr.port());
                    borrow.is_bound.store(true, Ordering::SeqCst);
                    borrow.send_tx = Some(send_tx);
                    (borrow.close_tx.subscribe(), borrow.close_tx.subscribe())
                };

                // Signal that send channel is ready
                ready_latch.decrement();

                trace!("UDP socket bound to {}", local_addr);

                // Emit 'listening' event
                Self::emit_str(This(socket_class.clone()), &ctx, "listening", vec![], false)?;

                // Start receiver loop
                let recv_started = receiver_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok();

                if recv_started {
                    let recv_socket = socket_arc.clone();
                    let recv_class = socket_class.clone();
                    let recv_closed = is_closed.clone();
                    let recv_running = receiver_running.clone();
                    let recv_ctx = ctx.clone();
                    let mut close_rx = recv_close_rx;

                    ctx.clone().spawn_exit(async move {
                        let recv_result = async {
                            let mut buf = vec![0u8; 65536];

                            loop {
                                tokio::select! {
                                    _ = close_rx.recv() => {
                                        break;
                                    }
                                    result = recv_socket.recv_from(&mut buf) => {
                                        match result {
                                            Ok((size, peer_addr)) => {
                                                let data = Buffer(buf[..size].to_vec()).into_js(&recv_ctx)?;
                                                let info = Object::new(recv_ctx.clone())?;
                                                info.set("address", peer_addr.ip().to_string())?;
                                                info.set("port", peer_addr.port())?;
                                                info.set(
                                                    "family",
                                                    if peer_addr.is_ipv4() { "IPv4" } else { "IPv6" },
                                                )?;

                                                let info_val: Value = info.into();
                                                Self::emit_str(
                                                    This(recv_class.clone()),
                                                    &recv_ctx,
                                                    "message",
                                                    vec![data, info_val],
                                                    false,
                                                )?;
                                            },
                                            Err(e) => {
                                                if recv_closed.load(Ordering::SeqCst) {
                                                    break;
                                                }
                                                return Err(Exception::throw_message(
                                                    &recv_ctx,
                                                    &format!("UDP receive error: {}", e),
                                                ));
                                            },
                                        }
                                    }
                                }
                            }

                            recv_running.store(false, Ordering::SeqCst);
                            Ok(())
                        }
                        .await;

                        recv_result.emit_error("recv", &recv_ctx, recv_class)?;
                        Ok(())
                    })?;
                }
                

                // Start sender loop
                let send_socket = socket_arc.clone();
                let mut close_rx = send_close_rx;
                let send_ctx = ctx.clone();
                ctx.spawn_exit(async move {
                    let send_result = {
                        loop {
                            tokio::select! {
                                _ = close_rx.recv() => {
                                    break;
                                }
                                msg = send_rx.recv() => {
                                    let Some((bytes, dest_addr, result_tx)) = msg else {
                                        break;
                                    };

                                    let result = send_socket.send_to(&bytes, &dest_addr).await;
                                    if let Some(result_tx) = result_tx{
                                        result_tx.send(result.map_err(|e| e.to_string())).map_err(|_|Exception::throw_message(&send_ctx, "Failed to call callback in send, channel closed!"))?;
                                        continue;
                                    }
                                    result?;
                                }
                            }
                        };
                        Ok(())
                    };
                    send_result.emit_error("receive", &send_ctx, socket_class3)?;
                    Ok(())
                })?;

                Ok(())
            }
            .await;

            bind_result.emit_error("bind", &ctx, socket_class2)?;
            Ok(())
        })?;

        Ok(())
    }

    #[qjs(skip)]
    fn ensure_listening(this: Class<'js, Self>, ctx: Ctx<'js>) -> Result<Arc<Latch>> {
        // Check if already bound and get latch
        let (is_bound, ready_latch) = {
            let borrow = this.borrow();
            (
                borrow.is_bound.load(Ordering::SeqCst),
                borrow.ready_latch.clone(),
            )
        };

        if !is_bound {
            // Auto-bind to random port
            let bind_addr = "0.0.0.0:0".to_string();
            Self::start_listening(this.clone(), ctx.clone(), bind_addr)?;
        }

        Ok(ready_latch)
    }

    pub fn bind(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
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
            Self::add_event_listener_str(This(this.clone()), &ctx, "listening", cb, true, true)?;
        }

        let bind_addr = [&address, ":", &port.to_string()].concat();

        Self::start_listening(this.0.clone(), ctx, bind_addr)?;

        Ok(this.0)
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

        // Extract bytes from message
        let bytes: Vec<u8> = if let Some(str_val) = msg.as_string() {
            str_val.to_string()?.into_bytes()
        } else {
            ObjectBytes::from_js(&ctx, msg)?
                .try_into()
                .map_err(|e: std::rc::Rc<str>| Exception::throw_type(&ctx, &e))?
        };

        // Check if socket is closed
        {
            let borrow = this.borrow();
            if borrow.is_closed.load(Ordering::SeqCst) {
                return Err(Exception::throw_message(&ctx, "Socket is closed"));
            }
        }

        // Format destination address
        let dest_addr = [&address, ":", &port.to_string()].concat();

        // Ensure listener is started and get latch to wait on
        let ready_latch = Self::ensure_listening(this.0.clone(), ctx.clone())?;

        let socket_class = this.0.clone();
        let socket_class2 = this.0.clone();

        let cb = callback.0;

        ctx.clone().spawn_exit(async move {
            let send_result = async {
                // Wait for send channel to be ready
                ready_latch.wait().await;

                // Send to the channel
                let result_rx = {
                    let (result_tx, result_rx) = if cb.is_some() {
                        let (result_tx, result_rx) = oneshot::channel();
                        (Some(result_tx), Some(result_rx))
                    } else {
                        (None, None)
                    };

                    let borrow = socket_class.borrow();
                    let send_tx = borrow.send_tx.as_ref().ok_or_else(|| {
                        Exception::throw_message(&ctx, "Failed to initialize socket")
                    })?;

                    send_tx
                        .send((bytes, dest_addr, result_tx))
                        .map_err(|_| Exception::throw_message(&ctx, "Failed to send message"))?;
                    result_rx
                };

                //we dont have any callback
                let Some(result_rx) = result_rx else {
                    return Ok(());
                };

                // Wait for result
                let result = result_rx.await.unwrap_or(Err("Socket closed".to_string()));

                let Some(cb) = cb else {
                    result.or_throw(&ctx)?;
                    return Ok(());
                };

                // Callback handles both success and error
                match result {
                    Ok(sent) => {
                        cb.call::<_, ()>((Value::new_null(ctx.clone()), sent))?;
                    },
                    Err(e) => {
                        let err = Exception::from_message(ctx.clone(), &e)?;
                        cb.call::<_, ()>((err,))?;
                    },
                }

                Ok(())
            }
            .await;

            send_result.emit_error("send", &ctx, socket_class2)?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn close(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        callback: Opt<Function<'js>>,
    ) -> Result<Class<'js, Self>> {
        let already_closed = {
            let borrow = this.borrow();
            let was_closed = borrow
                .is_closed
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err();
            if !was_closed {
                // Send close signal to interrupt receive/send loops
                let _ = borrow.close_tx.send(());
            }
            was_closed
        };

        if already_closed {
            return Err(Exception::throw_message(
                &ctx,
                "ERR_SOCKET_DGRAM_NOT_RUNNING",
            ));
        }

        if let Some(cb) = callback.0 {
            Self::add_event_listener_str(This(this.clone()), &ctx, "close", cb, true, true)?;
        }

        // Drop the socket and clear send channel
        {
            let mut borrow = this.borrow_mut();
            borrow.socket = None;
            borrow.send_tx = None;
        }

        // Emit close event directly (don't rely on sender loop which may not have started)
        let this_clone = this.0.clone();
        ctx.clone().spawn_exit(async move {
            Self::emit_str(This(this_clone), &ctx, "close", vec![], false)?;
            Ok(())
        })?;

        Ok(this.0)
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
