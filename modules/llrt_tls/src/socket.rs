// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Uses shared types from llrt_net to reduce code duplication.
// The main difference from net::Socket is that TLSSocket wraps connections with TLS
// and adds TLS-specific properties (encrypted, authorized, cipher info, etc.)

use std::sync::{Arc, RwLock};

use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventKey, EventList};
// Re-use shared socket types from llrt_net
use llrt_net::{get_hostname, rw_join, ReadyState, LOCALHOST};
use llrt_stream::{
    impl_stream_events,
    readable::{ReadableStream, ReadableStreamInner},
    writable::{WritableStream, WritableStreamInner},
};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Opt, This},
    Class, Ctx, Error, Exception, Function, IntoJs, JsLifetime, Object, Result, Value,
};
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot::Receiver;
use tokio_rustls::{client::TlsStream, server::TlsStream as ServerTlsStream, TlsConnector};
use tracing::trace;

use crate::keylog::{ChannelKeyLog, KeyLogLine};
use crate::secure_context::SecureContext;
use crate::{build_client_config, BuildClientConfigOptions};

impl_stream_events!(TLSSocket);

/// Extract address parts from a SocketAddr (used for TLS connections where we don't
/// need the Result wrapper that llrt_net::get_address_parts provides)
fn get_address_parts(addr: std::net::SocketAddr) -> (String, u16, String) {
    (
        addr.ip().to_string(),
        addr.port(),
        String::from(if addr.is_ipv4() { "IPv4" } else { "IPv6" }),
    )
}

#[rquickjs::class]
pub struct TLSSocket<'js> {
    emitter: EventEmitter<'js>,
    readable_stream_inner: ReadableStreamInner<'js>,
    writable_stream_inner: WritableStreamInner<'js>,
    pub(crate) connecting: bool,
    pub(crate) destroyed: bool,
    pub(crate) pending: bool,
    pub(crate) encrypted: bool,
    pub(crate) authorized: bool,
    pub(crate) authorization_error: Option<String>,
    pub(crate) local_address: Option<String>,
    pub(crate) local_family: Option<String>,
    pub(crate) local_port: Option<u16>,
    pub(crate) remote_address: Option<String>,
    pub(crate) remote_family: Option<String>,
    pub(crate) remote_port: Option<u16>,
    pub(crate) ready_state: ReadyState,
    pub(crate) allow_half_open: bool,
    pub(crate) servername: Option<String>,
    pub(crate) alpn_protocol: Option<String>,
    // Cipher and protocol info captured after TLS handshake
    pub(crate) cipher_name: Option<String>,
    pub(crate) cipher_standard_name: Option<String>,
    pub(crate) cipher_version: Option<String>,
    pub(crate) protocol_version: Option<String>,
}

unsafe impl<'js> JsLifetime<'js> for TLSSocket<'js> {
    type Changed<'to> = TLSSocket<'to>;
}

impl<'js> Trace<'js> for TLSSocket<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
    }
}

impl<'js> Emitter<'js> for TLSSocket<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }

    fn on_event_changed(&mut self, event: EventKey<'js>, added: bool) -> Result<()> {
        self.readable_stream_inner.on_event_changed(event, added)
    }
}

impl<'js> ReadableStream<'js> for TLSSocket<'js> {
    fn inner_mut(&mut self) -> &mut ReadableStreamInner<'js> {
        &mut self.readable_stream_inner
    }

    fn inner(&self) -> &ReadableStreamInner<'js> {
        &self.readable_stream_inner
    }
}

impl<'js> WritableStream<'js> for TLSSocket<'js> {
    fn inner_mut(&mut self) -> &mut WritableStreamInner<'js> {
        &mut self.writable_stream_inner
    }

    fn inner(&self) -> &WritableStreamInner<'js> {
        &self.writable_stream_inner
    }
}

/// Options for TLS connection
pub struct TlsConnectOptions {
    pub host: String,
    pub port: u16,
    pub servername: Option<String>,
    pub reject_unauthorized: bool,
    pub ca: Option<Vec<Vec<u8>>>,
    /// Client certificate in PEM format for mTLS
    pub cert: Option<Vec<u8>>,
    /// Client private key in PEM format for mTLS
    pub key: Option<Vec<u8>>,
    pub allow_half_open: bool,
    pub alpn_protocols: Option<Vec<String>>,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    /// Pre-built client configuration from a SecureContext
    pub client_config: Option<Arc<ClientConfig>>,
}

impl Default for TlsConnectOptions {
    fn default() -> Self {
        Self {
            host: LOCALHOST.to_string(),
            port: 443,
            servername: None,
            reject_unauthorized: true,
            ca: None,
            cert: None,
            key: None,
            allow_half_open: false,
            alpn_protocols: None,
            min_version: None,
            max_version: None,
            client_config: None,
        }
    }
}

impl TlsConnectOptions {
    pub fn from_js_options<'js>(ctx: &Ctx<'js>, opts: &Object<'js>) -> Result<Self> {
        let mut options = Self::default();

        if let Some(host) = opts.get_optional::<_, String>("host")? {
            options.host = host;
        }

        if let Some(port) = opts.get_optional::<_, u16>("port")? {
            options.port = port;
        }

        // servername must be a string - it's the SNI (Server Name Indication) hostname
        // which is required to be a valid DNS name string
        if let Some(servername) = opts.get_optional::<_, String>("servername")? {
            options.servername = Some(servername);
        }

        if let Some(reject_unauthorized) = opts.get_optional::<_, bool>("rejectUnauthorized")? {
            options.reject_unauthorized = reject_unauthorized;
        }

        if let Some(allow_half_open) = opts.get_optional::<_, bool>("allowHalfOpen")? {
            options.allow_half_open = allow_half_open;
        }

        // Handle CA certificates
        if let Some(ca_value) = opts.get_optional::<_, Value>("ca")? {
            let mut ca_certs = Vec::new();
            if let Some(ca_array) = ca_value.as_array() {
                for item in ca_array.iter::<Value>() {
                    let item = item?;
                    if let Some(s) = item.as_string() {
                        ca_certs.push(s.to_string()?.into_bytes());
                    } else if let Some(bytes) = get_bytes_from_value(ctx, &item)? {
                        ca_certs.push(bytes);
                    }
                }
            } else if let Some(s) = ca_value.as_string() {
                ca_certs.push(s.to_string()?.into_bytes());
            } else if let Some(bytes) = get_bytes_from_value(ctx, &ca_value)? {
                ca_certs.push(bytes);
            }
            if !ca_certs.is_empty() {
                options.ca = Some(ca_certs);
            }
        }

        // Handle client certificate for mTLS
        if let Some(cert_value) = opts.get_optional::<_, Value>("cert")? {
            if let Some(s) = cert_value.as_string() {
                options.cert = Some(s.to_string()?.into_bytes());
            } else if let Some(bytes) = get_bytes_from_value(ctx, &cert_value)? {
                options.cert = Some(bytes);
            }
        }

        // Handle client private key for mTLS
        if let Some(key_value) = opts.get_optional::<_, Value>("key")? {
            if let Some(s) = key_value.as_string() {
                options.key = Some(s.to_string()?.into_bytes());
            } else if let Some(bytes) = get_bytes_from_value(ctx, &key_value)? {
                options.key = Some(bytes);
            }
        }

        // Handle ALPN protocols
        if let Some(alpn_value) = opts.get_optional::<_, Value>("ALPNProtocols")? {
            let mut protocols = Vec::new();
            if let Some(alpn_array) = alpn_value.as_array() {
                for item in alpn_array.iter::<String>() {
                    protocols.push(item?);
                }
            }
            if !protocols.is_empty() {
                options.alpn_protocols = Some(protocols);
            }
        }

        // Handle TLS version constraints
        if let Some(min_version) = opts.get_optional::<_, String>("minVersion")? {
            options.min_version = Some(min_version);
        }

        if let Some(max_version) = opts.get_optional::<_, String>("maxVersion")? {
            options.max_version = Some(max_version);
        }

        // Handle secureContext option - use pre-built client config
        if let Some(secure_context) =
            opts.get_optional::<_, Class<SecureContext>>("secureContext")?
        {
            let ctx_borrow = secure_context.borrow();
            if let Some(client_config) = &ctx_borrow.client_config {
                options.client_config = Some(client_config.clone());
            }
        }

        Ok(options)
    }
}

fn get_bytes_from_value<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<Option<Vec<u8>>> {
    // Try to convert the value to bytes using ObjectBytes
    if let Ok(bytes) = ObjectBytes::from(ctx, value) {
        if let Ok(vec) = TryInto::<Vec<u8>>::try_into(bytes) {
            return Ok(Some(vec));
        }
    }
    Ok(None)
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TLSSocket<'js> {
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
    pub fn encrypted(&self) -> bool {
        self.encrypted
    }

    #[qjs(get, enumerable)]
    pub fn authorized(&self) -> bool {
        self.authorized
    }

    #[qjs(get, enumerable)]
    pub fn authorization_error(&self) -> Option<String> {
        self.authorization_error.clone()
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

    #[qjs(get, enumerable)]
    pub fn servername(&self) -> Option<String> {
        self.servername.clone()
    }

    #[qjs(get, enumerable)]
    pub fn alpn_protocol(&self) -> Option<String> {
        self.alpn_protocol.clone()
    }

    /// Returns the negotiated TLS protocol version, or null if not connected.
    pub fn get_protocol(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let borrow = this.borrow();
        match &borrow.protocol_version {
            Some(version) => version.clone().into_js(&ctx),
            None => Ok(Value::new_null(ctx)),
        }
    }

    /// Returns an object representing the peer's certificate.
    /// If the peer does not provide a certificate, an empty object will be returned.
    pub fn get_peer_certificate(ctx: Ctx<'js>, _detailed: Opt<bool>) -> Result<Object<'js>> {
        // For now, return a basic certificate object
        // In a full implementation, we would extract certificate details from the TLS connection
        let cert = Object::new(ctx.clone())?;
        // Certificate details would be populated here if we had access to the peer cert
        // For now we return an empty object indicating no detailed cert info available
        Ok(cert)
    }

    /// Returns an object representing the cipher name and SSL/TLS protocol version.
    /// Returns null if the socket is not connected.
    pub fn get_cipher(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let borrow = this.borrow();
        // Return null if not connected (no cipher info available)
        if borrow.cipher_name.is_none() {
            return Ok(Value::new_null(ctx));
        }

        let cipher = Object::new(ctx.clone())?;
        if let Some(name) = &borrow.cipher_name {
            cipher.set("name", name.clone())?;
        }
        if let Some(standard_name) = &borrow.cipher_standard_name {
            cipher.set("standardName", standard_name.clone())?;
        }
        if let Some(version) = &borrow.cipher_version {
            cipher.set("version", version.clone())?;
        }
        Ok(cipher.into_value())
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

    pub fn connect(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        options: Object<'js>,
        callback: Opt<Function<'js>>,
    ) -> Result<Class<'js, Self>> {
        let borrow = this.borrow();
        let allow_half_open = borrow.allow_half_open;
        if borrow.destroyed {
            return Err(Exception::throw_message(&ctx, "Socket destroyed"));
        }
        drop(borrow);

        let mut opts = TlsConnectOptions::from_js_options(&ctx, &options)?;

        let this = this.0;
        let this2 = this.clone();

        if let Some(listener) = callback.0 {
            TLSSocket::add_event_listener_str(
                This(this.clone()),
                &ctx,
                "secureConnect",
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
                TLSSocket::emit_close(this3.clone(), &ctx3, false)?;
                return Ok(());
            }

            let connect = async move {
                let hostname = get_hostname(&opts.host, opts.port);

                // Connect TCP first
                let tcp_stream = TcpStream::connect(&hostname).await.or_throw(&ctx3)?;

                // Set TCP addresses
                if let Ok(peer_addr) = tcp_stream.peer_addr() {
                    let (remote_address, remote_port, remote_family) = get_address_parts(peer_addr);
                    let mut borrow = this2.borrow_mut();
                    borrow.remote_address = Some(remote_address);
                    borrow.remote_port = Some(remote_port);
                    borrow.remote_family = Some(remote_family);
                }

                if let Ok(local_addr) = tcp_stream.local_addr() {
                    let (local_address, local_port, local_family) = get_address_parts(local_addr);
                    let mut borrow = this2.borrow_mut();
                    borrow.local_address = Some(local_address);
                    borrow.local_port = Some(local_port);
                    borrow.local_family = Some(local_family);
                }

                // Check if there are keylog listeners
                let has_keylog_listeners = {
                    let borrow = this2.borrow();
                    borrow.has_listener_str("keylog")
                };

                // Create keylog channel if there are listeners
                let keylog_receiver: Option<mpsc::UnboundedReceiver<KeyLogLine>> =
                    if has_keylog_listeners {
                        let (keylog, receiver) = ChannelKeyLog::new();

                        // Build TLS config with keylog
                        let tls_config = if let Some(client_config) = opts.client_config {
                            // If using a pre-built config, we can't add keylog to it
                            // The user should set up keylog in their SecureContext
                            client_config
                        } else {
                            Arc::new(
                                build_client_config(BuildClientConfigOptions {
                                    reject_unauthorized: opts.reject_unauthorized,
                                    ca: opts.ca.clone(),
                                    cert: opts.cert.clone(),
                                    key: opts.key.clone(),
                                    key_log: Some(keylog),
                                    ..Default::default()
                                })
                                .map_err(|e| Exception::throw_message(&ctx3, &e.to_string()))?,
                            )
                        };

                        // Store the config for the connector
                        opts.client_config = Some(tls_config);
                        Some(receiver)
                    } else {
                        None
                    };

                // Build TLS config - use pre-built secureContext if available
                let tls_config = if let Some(client_config) = opts.client_config {
                    client_config
                } else {
                    Arc::new(
                        build_client_config(BuildClientConfigOptions {
                            reject_unauthorized: opts.reject_unauthorized,
                            ca: opts.ca,
                            cert: opts.cert,
                            key: opts.key,
                            key_log: None,
                            ..Default::default()
                        })
                        .map_err(|e| Exception::throw_message(&ctx3, &e.to_string()))?,
                    )
                };

                let connector = TlsConnector::from(tls_config);

                // If we have a keylog receiver, spawn a task to emit events
                if let Some(mut receiver) = keylog_receiver {
                    let this_keylog = this2.clone();
                    let ctx_keylog = ctx3.clone();
                    ctx_keylog.clone().spawn_exit(async move {
                        while let Some(line) = receiver.recv().await {
                            // Create a Buffer from the keylog line
                            let line_bytes = line.as_bytes().to_vec();
                            if let Ok(buffer) =
                                rquickjs::ArrayBuffer::new(ctx_keylog.clone(), line_bytes)
                            {
                                let _ = TLSSocket::emit_str(
                                    This(this_keylog.clone()),
                                    &ctx_keylog,
                                    "keylog",
                                    vec![buffer.into_value()],
                                    false,
                                );
                            }
                        }
                        Ok(())
                    })?;
                }

                // Determine server name for SNI
                let server_name_str = opts.servername.as_ref().unwrap_or(&opts.host);
                let server_name: ServerName = server_name_str
                    .to_string()
                    .try_into()
                    .map_err(|_| Exception::throw_message(&ctx3, "Invalid server name"))?;

                // Store servername
                {
                    let mut borrow = this2.borrow_mut();
                    borrow.servername = Some(server_name_str.clone());
                }

                // Perform TLS handshake
                let tls_stream = connector
                    .connect(server_name, tcp_stream)
                    .await
                    .map_err(|e| {
                        let mut borrow = this2.borrow_mut();
                        borrow.authorized = false;
                        borrow.authorization_error = Some(e.to_string());
                        Exception::throw_message(&ctx3, &format!("TLS handshake failed: {}", e))
                    })?;

                // Capture cipher and protocol info from the TLS connection
                let (cipher_name, cipher_standard_name, protocol_version) = {
                    let conn = tls_stream.get_ref().1;
                    let cipher_suite = conn.negotiated_cipher_suite();
                    let protocol = conn.protocol_version();

                    let (name, standard_name) = if let Some(suite) = cipher_suite {
                        let suite_id = suite.suite();
                        let standard = format!("{:?}", suite_id);
                        let openssl_name = crate::cipher_suite_to_openssl_name(suite_id);
                        (Some(openssl_name.to_string()), Some(standard))
                    } else {
                        (None, None)
                    };

                    let version = protocol.map(|p| format!("{:?}", p));
                    (name, standard_name, version)
                };

                // Mark as encrypted and authorized
                {
                    let mut borrow = this2.borrow_mut();
                    borrow.encrypted = true;
                    borrow.authorized = opts.reject_unauthorized; // If we got here with reject_unauthorized, we're authorized
                    borrow.connecting = false;
                    borrow.pending = false;
                    borrow.ready_state = ReadyState::Open;
                    // Store cipher and protocol info
                    borrow.cipher_name = cipher_name;
                    borrow.cipher_standard_name = cipher_standard_name;
                    borrow.cipher_version = protocol_version.clone();
                    borrow.protocol_version = protocol_version;
                }

                trace!("TLS connection established to {}", hostname);

                // Process the TLS stream
                let (readable_done, writable_done) =
                    Self::process_tls_stream(&this2, &ctx3, tls_stream, allow_half_open)?;

                // Emit secureConnect event
                TLSSocket::emit_str(This(this2.clone()), &ctx3, "secureConnect", vec![], false)?;

                let had_error = rw_join(&ctx3, readable_done, writable_done).await?;

                TLSSocket::emit_close(this2, &ctx3, had_error)?;

                Ok::<_, Error>(())
            }
            .await;

            connect.emit_error("connect", &ctx2, this3)?;
            Ok(())
        })?;

        Ok(this)
    }
}

impl<'js> TLSSocket<'js> {
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
                encrypted: false,
                authorized: false,
                authorization_error: None,
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
                servername: None,
                alpn_protocol: None,
                cipher_name: None,
                cipher_standard_name: None,
                cipher_version: None,
                protocol_version: None,
            },
        )?;
        Ok(instance)
    }

    pub fn process_tls_stream(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        stream: TlsStream<TcpStream>,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)> {
        let (reader, writer) = tokio::io::split(stream);
        Self::process_stream(this, ctx, reader, writer, allow_half_open)
    }

    pub fn process_server_tls_stream(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        stream: ServerTlsStream<TcpStream>,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)> {
        let (reader, writer) = tokio::io::split(stream);
        Self::process_stream(this, ctx, reader, writer, allow_half_open)
    }

    fn process_stream<R, W>(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        reader: R,
        writer: W,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)>
    where
        R: tokio::io::AsyncRead + 'js + Unpin,
        W: tokio::io::AsyncWrite + 'js + Unpin,
    {
        let this2 = this.clone();
        let readable_done =
            ReadableStream::process_callback(this.clone(), ctx, reader, move || {
                if !allow_half_open {
                    WritableStream::end(This(this2));
                }
            })?;
        let writable_done = WritableStream::process(this.clone(), ctx, writer)?;

        trace!("Connected to TLS stream");
        let mut borrow = this.borrow_mut();
        borrow.connecting = false;
        borrow.pending = false;
        borrow.ready_state = ReadyState::Open;
        drop(borrow);

        Ok((readable_done, writable_done))
    }

    pub fn emit_close(this: Class<'js, Self>, ctx: &Ctx<'js>, had_error: bool) -> Result<()> {
        Self::emit_str(
            This(this),
            ctx,
            "close",
            vec![had_error.into_js(ctx)?],
            false,
        )
    }
}
