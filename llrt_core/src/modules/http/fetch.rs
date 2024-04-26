// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Uri};

use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, This},
    prelude::{Async, Func},
    Class, Coerced, Ctx, Exception, FromJs, Function, Object, Result, Value,
};
use tokio::select;

use std::time::Instant;

use crate::{
    environment,
    modules::events::AbortSignal,
    modules::http::headers::Headers,
    modules::net::HTTP_CLIENT,
    security::{ensure_url_access, HTTP_DENY_LIST},
    utils::{mc_oneshot, object::get_bytes, result::ResultExt},
};
use crate::{security::HTTP_ALLOW_LIST, VERSION};

use super::response::Response;

pub(crate) fn init(ctx: &Ctx<'_>, globals: &Object) -> Result<()> {
    if let Some(Err(err)) = &*HTTP_ALLOW_LIST {
        return Err(Exception::throw_reference(
            ctx,
            &format!(
                r#""{}" env contains an invalid URI: {}"#,
                environment::ENV_LLRT_NET_ALLOW,
                &err.to_string()
            ),
        ));
    }

    if let Some(Err(err)) = &*HTTP_DENY_LIST {
        return Err(Exception::throw_reference(
            ctx,
            &format!(
                r#""{}" env contains an invalid URI: {}"#,
                environment::ENV_LLRT_NET_ALLOW,
                &err.to_string()
            ),
        ));
    }

    //init eagerly
    let client = &*HTTP_CLIENT;

    globals.set(
        "fetch",
        Func::from(Async(move |ctx, resource, args| {
            let start = Instant::now();
            let options = get_fetch_options(&ctx, resource, args);

            async move {
                let options = options?;

                let uri: Uri = options.url.parse().or_throw(&ctx)?;
                let method_string = options.method.to_string();
                let method = options.method;
                let abort_receiver = options.abort_receiver;

                ensure_url_access(&ctx, &uri)?;

                let mut req = Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("user-agent", format!("llrt {}", VERSION))
                    .header("accept", "*/*");

                if let Some(headers) = options.headers {
                    for (key, value) in headers.iter() {
                        req = req.header(key, value)
                    }
                }

                let req = req.body(options.body).or_throw(&ctx)?;
                let res = if let Some(abort_receiver) = &abort_receiver {
                    select! {
                        res = client.request(req) => res.or_throw(&ctx)?,
                        reason = abort_receiver.recv() => return Err(ctx.throw(reason))
                    }
                } else {
                    client.request(req).await.or_throw(&ctx)?
                };

                Response::from_incoming(ctx, res, method_string, options.url, start, abort_receiver)
            }
        })),
    )?;
    Ok(())
}

struct FetchOptions<'js> {
    method: hyper::Method,
    url: String,
    headers: Option<Headers>,
    body: Full<Bytes>,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
}

fn get_fetch_options<'js>(
    ctx: &Ctx<'js>,
    resource: Value<'js>,
    opts: Opt<Value<'js>>,
) -> Result<FetchOptions<'js>> {
    let mut url = None;
    let mut resource_opts = None;
    let mut arg_opts = None;
    let mut headers = None;
    let mut method = None;
    let mut body = None;
    let mut abort_receiver = None;
    if let Some(obj) = resource.as_object() {
        let obj = obj.clone();
        if obj.instance_of::<crate::modules::http::Request>() {
            resource_opts = Some(obj);
        } else if let Some(to_string) = obj.get::<_, Option<Function>>(PredefinedAtom::ToString)? {
            url = Some(to_string.call::<_, String>((This(obj),))?);
        } else {
            resource_opts = Some(obj);
        }
    } else {
        url = Some(resource.get::<Coerced<String>>()?.to_string());
    }

    if let Some(options) = opts.0 {
        arg_opts = options.into_object();
    }

    if resource_opts.is_some() || arg_opts.is_some() {
        if let Some(method_opt) = get_option::<String>("method", &arg_opts, &resource_opts)? {
            method = Some(match method_opt.as_str() {
                "GET" => Ok(hyper::Method::GET),
                "POST" => Ok(hyper::Method::POST),
                "PUT" => Ok(hyper::Method::PUT),
                "CONNECT" => Ok(hyper::Method::CONNECT),
                "HEAD" => Ok(hyper::Method::HEAD),
                "PATCH" => Ok(hyper::Method::PATCH),
                "DELETE" => Ok(hyper::Method::DELETE),
                _ => Err(Exception::throw_type(
                    ctx,
                    &format!("Invalid HTTP method: {}", method_opt),
                )),
            }?);
        }

        if let Some(body_opt) = get_option::<Value>("body", &arg_opts, &resource_opts)? {
            let bytes = get_bytes(ctx, body_opt)?;
            body = Some(Full::from(bytes));
        }

        if let Some(url_opt) = get_option::<String>("url", &arg_opts, &resource_opts)? {
            url = Some(url_opt);
        }

        if let Some(headers_op) = get_option::<Value>("headers", &arg_opts, &resource_opts)? {
            headers = Some(Headers::from_value(ctx, headers_op)?);
        }

        if let Some(signal) = get_option::<Class<AbortSignal>>("signal", &arg_opts, &resource_opts)?
        {
            abort_receiver = Some(signal.borrow().sender.subscribe());
        }
    }

    let url = match url {
        Some(url) => url,
        None => return Err(Exception::throw_reference(ctx, "Missing required url")),
    };

    Ok(FetchOptions {
        method: method.unwrap_or_default(),
        url,
        headers,
        body: body.unwrap_or_default(),
        abort_receiver,
    })
}

fn get_option<'js, V: FromJs<'js> + Sized>(
    arg: &str,
    a: &Option<Object<'js>>,
    b: &Option<Object<'js>>,
) -> Result<Option<V>> {
    if let Some(opt) = a {
        if let Some(value) = opt.get::<_, Option<V>>(arg)? {
            return Ok(Some(value));
        }
    }

    if let Some(opt) = b {
        if let Some(value) = opt.get::<_, Option<V>>(arg)? {
            return Ok(Some(value));
        }
    }

    Ok(None)
}
