// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Uri};

use rquickjs::{
    function::Opt,
    prelude::{Async, Func},
    Ctx, Exception, Object, Result, Value,
};

use std::time::Instant;

use crate::{
    environment,
    http::headers::Headers,
    net::HTTP_CLIENT,
    security::{ensure_url_access, HTTP_DENY_LIST},
    utils::{
        object::{get_bytes, ObjectExt},
        result::ResultExt,
    },
};
use crate::{security::HTTP_ALLOW_LIST, VERSION};

use super::response::Response;

struct FetchArgs<'js>(Ctx<'js>, Value<'js>, Opt<Value<'js>>);

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
            let FetchArgs(ctx, resource, args) = FetchArgs(ctx, resource, args);
            //let client = client.clone();

            let mut method = Ok(hyper::Method::GET);
            let mut body = Ok(Full::<Bytes>::default());
            let mut headers: Option<Result<Headers>> = None;

            let (url, resource_options) = get_url_options(resource);
            let mut url = url;

            let mut options = None;
            if let Some(opts) = args.0 {
                if opts.is_object() {
                    let opts = opts.into_object().unwrap();
                    options = Some(opts);
                }
            }

            let options = resource_options.or(options);

            if let Some(opts) = options {
                let method_opts = opts.get_optional::<&str, String>("method");

                headers = opts
                    .get_optional("headers")
                    .transpose()
                    .map(|v| v.and_then(|headers_val| Headers::from_value(&ctx, headers_val)));

                let body_opt: Option<Value> = opts.get("body").unwrap_or_default();
                let url_opt: Option<String> = opts.get("url").unwrap_or_default();

                if let Some(url_val) = url_opt {
                    url = Some(Ok(url_val));
                }

                if let Some(body_value) = body_opt {
                    let bytes = get_bytes(&ctx, body_value);
                    body = bytes.map(Full::from);
                }

                method = method_opts.and_then(|m| {
                    let m = m.as_deref();
                    match m {
                        None | Some("GET") => Ok(hyper::Method::GET),
                        Some("POST") => Ok(hyper::Method::POST),
                        Some("PUT") => Ok(hyper::Method::PUT),
                        Some("CONNECT") => Ok(hyper::Method::CONNECT),
                        Some("HEAD") => Ok(hyper::Method::HEAD),
                        Some("PATCH") => Ok(hyper::Method::PATCH),
                        Some("DELETE") => Ok(hyper::Method::DELETE),
                        _ => Err(Exception::throw_type(
                            &ctx,
                            &format!("Invalid HTTP method: {}", m.unwrap_or("{empty}")),
                        )),
                    }
                });
            }

            async move {
                let url = url.unwrap_or_else(|| {
                    Err(Exception::throw_reference(&ctx, "Missing required url"))
                })?;

                let uri: Uri = url.parse().or_throw(&ctx)?;

                let method = method?;
                let method_string = method.to_string();

                ensure_url_access(&ctx, &uri)?;

                let mut req = Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("user-agent", format!("llrt {}", VERSION))
                    .header("accept", "*/*");

                if let Some(headers) = headers {
                    for (key, value) in headers?.iter() {
                        req = req.header(key, value)
                    }
                }

                let body = body?;

                let req = req.body(body).or_throw(&ctx)?;
                let res = client.request(req).await.or_throw(&ctx)?; //TODO return ErrorObject

                Response::from_incoming(ctx, res, method_string, url, start)
            }
        })),
    )?;
    Ok(())
}

fn get_url_options(resource: Value) -> (Option<Result<String>>, Option<Object>) {
    if resource.is_string() {
        let url_string = resource.get();
        return (Some(url_string), None);
    } else if resource.is_object() {
        let resource_obj = resource.into_object().unwrap();
        return (None, Some(resource_obj));
    }
    (None, None)
}
