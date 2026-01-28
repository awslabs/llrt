// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use llrt_utils::{any_of::AnyOf4, bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{prelude::Opt, Ctx, Error, FromJs, Result, Value};

use crate::{
    proxy::{ProxyConfig, PROXY_CONFIG},
    HyperClient,
};

#[rquickjs::class]
#[derive(rquickjs::JsLifetime, rquickjs::class::Trace)]
pub struct Agent {
    #[qjs(skip_trace)]
    client: HyperClient,
}

impl Agent {
    pub fn client(&self) -> HyperClient {
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

        // Use global proxy config for the Agent's client
        let proxy_config: Option<&ProxyConfig> = if PROXY_CONFIG.is_enabled() {
            Some(&*PROXY_CONFIG)
        } else {
            None
        };
        let client = crate::build_client(Some(config), proxy_config)
            .or_throw_msg(&ctx, "Failed to build HTTP client")?;

        Ok(Self { client })
    }
}

pub struct AgentOptions {
    reject_unauthorized: Option<bool>,
    ca: Option<Vec<Vec<u8>>>,
}

impl<'js> FromJs<'js> for AgentOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let reject_unauthorized = obj.get_optional::<_, bool>("rejectUnauthorized")?;
        let ca = obj
            .get_optional::<_, AnyOf4<String, Vec<String>, ObjectBytes, Vec<ObjectBytes>>>("ca")?
            .map(|ca| {
                let ca = match ca {
                    AnyOf4::A(ca) => vec![ca.into_bytes()],
                    AnyOf4::B(ca) => ca.into_iter().map(|ca| ca.into_bytes()).collect(),
                    AnyOf4::C(ca) => vec![ca.into_bytes(ctx)?],
                    AnyOf4::D(ca) => ca
                        .into_iter()
                        .map(|ca| ca.into_bytes(ctx))
                        .collect::<Result<Vec<_>>>()?,
                };
                Ok::<_, Error>(ca)
            })
            .transpose()?;

        Ok(Self {
            reject_unauthorized,
            ca,
        })
    }
}
