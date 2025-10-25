// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::convert::Infallible;

use bytes::Bytes;
use either::Either;
use http_body_util::combinators::BoxBody;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use llrt_dns_cache::CachedDnsResolver;
use llrt_utils::object::ObjectExt;
use llrt_utils::result::ResultExt;
use rquickjs::{prelude::Opt, Ctx, Error, FromJs, Result};

#[rquickjs::class]
#[derive(rquickjs::JsLifetime, rquickjs::class::Trace)]
pub struct Agent {
    #[qjs(skip_trace)]
    client: Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>>,
}

impl Agent {
    pub fn client(
        &self,
    ) -> Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>> {
        self.client.clone()
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Agent {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, options: Opt<AgentOptions>) -> Result<Self> {
        let mut reject_unauthorized = true;
        let mut ca = None;

        if let Some(options) = options.0 {
            if let Some(opt_reject_unauthorized) = options.reject_unauthorized {
                reject_unauthorized = opt_reject_unauthorized;
            }
            if let Some(opt_ca) = options.ca {
                ca = Some(opt_ca);
            }
        }

        let config = llrt_tls::build_client_config(llrt_tls::BuildClientConfigOptions {
            reject_unauthorized,
            ca,
        })
        .or_throw_msg(&ctx, "Failed to build TLS config")?;
        let client =
            crate::build_client(Some(config)).or_throw_msg(&ctx, "Failed to build HTTP client")?;

        Ok(Self { client })
    }
}

pub struct AgentOptions {
    reject_unauthorized: Option<bool>,
    ca: Option<Vec<Vec<u8>>>,
}

impl<'js> FromJs<'js> for AgentOptions {
    fn from_js(_: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let reject_unauthorized = obj.get_optional::<_, bool>("rejectUnauthorized")?;
        let ca = obj
            .get_optional::<_, Either<String, Vec<String>>>("ca")?
            .map(|ca| match ca {
                Either::Left(ca) => vec![ca.into_bytes()],
                Either::Right(ca) => ca.into_iter().map(|ca| ca.into_bytes()).collect(),
            });

        Ok(Self {
            reject_unauthorized,
            ca,
        })
    }
}
