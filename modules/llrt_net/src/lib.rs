// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    io,
    net::SocketAddr,
    task::{Context, Poll},
};

use llrt_context::CtxExtension;
use llrt_events::Emitter;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest, This},
    Class, Ctx, Error, IntoJs, Result, Value,
};
use tokio::net::{TcpListener, TcpStream};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

mod server;
mod socket;

use self::{server::Server, socket::Socket};

const LOCALHOST: &str = "localhost";

enum NetStream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}

enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(UnixListener),
}

impl Listener {
    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<NetStream>> {
        match self {
            Listener::Tcp(tcp) => tcp
                .poll_accept(cx)
                .map_ok(|(stream, _)| NetStream::Tcp(stream)),
            #[cfg(unix)]
            Listener::Unix(unix) => unix
                .poll_accept(cx)
                .map_ok(|(stream, _)| NetStream::Unix(stream)),
        }
    }
}

fn address_parts(addr: SocketAddr) -> (String, u16, &'static str) {
    (
        addr.ip().to_string(),
        addr.port(),
        if addr.is_ipv4() { "IPv4" } else { "IPv6" },
    )
}

fn connect<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Class<'js, Socket<'js>>> {
    let options = args.0.first().filter(|arg| arg.is_object()).cloned();
    let socket = Socket::create(&ctx, options, None)?;
    Socket::connect(This(socket), ctx, args)
}

// A listener error in a job would only be logged; rethrown in a task, it ends the process.
fn rethrow_in_task<'js>(ctx: &Ctx<'js>, result: Result<()>) -> Result<()> {
    if let Err(Error::Exception) = result {
        let error = ctx.catch();
        let ctx2 = ctx.clone();
        ctx.spawn_exit_simple(async move { Err(ctx2.throw(error)) });
        return Ok(());
    }
    result
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

            Server::add_event_emitter_prototype(ctx)?;

            let connect = Func::from(connect).into_js(ctx)?;

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
