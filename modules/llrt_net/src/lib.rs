// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{net::SocketAddr, result::Result as StdResult};

use llrt_events::Emitter;
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, This},
    Class, Ctx, IntoJs, Result,
};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot::Receiver,
};

use self::security::ensure_access;
pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};

mod security;
mod server;
mod socket;

use self::{server::Server, socket::Socket};

const LOCALHOST: &str = "localhost";

#[allow(dead_code)]
enum ReadyState {
    Opening,
    Open,
    Closed,
    ReadOnly,
    WriteOnly,
}

impl ReadyState {
    #[allow(clippy::inherent_to_string)]
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
            },
            #[cfg(unix)]
            NetStream::Unix((stream, _)) => {
                Socket::process_unix_stream(socket, ctx, stream, allow_half_open)
            },
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
    async fn accept(&self, ctx: &Ctx<'_>) -> Result<NetStream> {
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

fn get_hostname(host: &str, port: u16) -> String {
    [host, itoa::Buffer::new().format(port)].join(":")
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

async fn rw_join(
    ctx: &Ctx<'_>,
    readable_done: Receiver<bool>,
    writable_done: Receiver<bool>,
) -> Result<bool> {
    let (readable_res, writable_res) = tokio::join!(readable_done, writable_done);
    let had_error = readable_res.or_throw_msg(ctx, "Readable sender dropped")?
        || writable_res.or_throw_msg(ctx, "Writable sender dropped")?;
    Ok(had_error)
}

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createConnection")?;
        declare.declare("connect")?;
        declare.declare("createServer")?;
        declare.declare(stringify!(Socket))?;
        declare.declare(stringify!(Server))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Socket>::define(default)?;
            Class::<Server>::define(default)?;

            Socket::add_event_emitter_prototype(ctx)?;
            Server::add_event_emitter_prototype(ctx)?;

            let connect = Func::from(|ctx, args| {
                struct Args<'js>(Ctx<'js>);
                let Args(ctx) = Args(ctx);
                let this = Socket::new(ctx.clone(), false)?;
                Socket::connect(This(this), ctx.clone(), args)
            })
            .into_js(ctx)?;

            default.set("createConnection", connect.clone())?;
            default.set("connect", connect)?;
            default.set(
                "createServer",
                Func::from(|ctx, args| {
                    struct Args<'js>(Ctx<'js>);
                    let Args(ctx) = Args(ctx);
                    Server::new(ctx.clone(), args)
                }),
            )
        })?;
        Ok(())
    }
}

impl From<NetModule> for ModuleInfo<NetModule> {
    fn from(val: NetModule) -> Self {
        ModuleInfo {
            name: "net",
            module: val,
        }
    }
}
