use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Uri};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use rquickjs::{
    function::Opt,
    prelude::{Async, Func},
    Ctx, Error, Exception, Object, Result, Value,
};

use std::time::{Duration, Instant};

use crate::{
    http::headers::Headers,
    net::TLS_CONFIG,
    security::{ensure_url_access, HTTP_DENY_LIST},
    utils::{
        object::{get_bytes, ObjectExt},
        result::ResultExt,
    },
};
use crate::{security::HTTP_ALLOW_LIST, VERSION};

use super::response::{Response, ResponseData};

struct FetchArgs<'js>(Ctx<'js>, Value<'js>, Opt<Value<'js>>);

pub(crate) fn init(ctx: &Ctx<'_>, globals: &Object) -> Result<()> {
    if let Some(Err(err)) = &*HTTP_ALLOW_LIST {
        return Err(Exception::throw_reference(
            ctx,
            &format!(
                "\"LLRT_NET_ALLOW\" env contains an invalid URI: {}",
                &err.to_string()
            ),
        ));
    }

    if let Some(Err(err)) = &*HTTP_DENY_LIST {
        return Err(Exception::throw_reference(
            ctx,
            &format!(
                "\"LLRT_NET_DENY\" env contains an invalid URI: {}",
                &err.to_string()
            ),
        ));
    }

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(TLS_CONFIG.clone())
        .https_or_http()
        .enable_http1()
        .build();

    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(5 * 30)) //5 minutes
        .build(https);

    // let client = Client::builder()
    //     .pool_idle_timeout(None)
    //     .build::<_, hyper::Body>(https);

    globals.set(
        "fetch",
        Func::from(Async(move |ctx, resource, args| {
            let start = Instant::now();
            let FetchArgs(ctx, resource, args) = FetchArgs(ctx, resource, args);
            let client = client.clone();

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

                headers = opts.get_optional("headers").transpose().map(|v| {
                    v.and_then(|headers_val| Headers::from_value(ctx.clone(), headers_val))
                });

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

                Ok::<Response, Error>(Response {
                    data: ResponseData::new(ctx, res, method_string, url, start)?,
                })
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
