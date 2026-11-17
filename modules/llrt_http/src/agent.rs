// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use llrt_utils::{any_of::AnyOf4, bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{prelude::Opt, Ctx, Error, Exception, FromJs, Object, Result, Value};

use crate::HyperClient;

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
        let AgentOptions {
            reject_unauthorized,
            ca,
            cert,
            key,
        } = options.0.unwrap_or_default();

        let config = llrt_tls::build_client_config(llrt_tls::BuildClientConfigOptions {
            reject_unauthorized: reject_unauthorized.unwrap_or(true),
            ca,
            cert,
            key,
        })
        .or_throw_msg(&ctx, "Failed to build TLS config")?;
        let client =
            crate::build_client(Some(config)).or_throw_msg(&ctx, "Failed to build HTTP client")?;

        Ok(Self { client })
    }
}

#[derive(Default)]
pub struct AgentOptions {
    reject_unauthorized: Option<bool>,
    ca: Option<Vec<Vec<u8>>>,
    cert: Option<Vec<u8>>,
    key: Option<Vec<u8>>,
}

impl<'js> FromJs<'js> for AgentOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        Ok(Self {
            reject_unauthorized: obj.get_optional::<_, bool>("rejectUnauthorized")?,
            ca: get_pem_list(ctx, obj, "ca")?,
            cert: get_single_pem(ctx, obj, "cert")?,
            key: get_single_pem(ctx, obj, "key")?,
        })
    }
}

/// Reads an option that accepts a PEM string or buffer, or an array of them.
fn get_pem_list<'js>(
    ctx: &Ctx<'js>,
    obj: &Object<'js>,
    name: &str,
) -> Result<Option<Vec<Vec<u8>>>> {
    obj.get_optional::<_, AnyOf4<String, Vec<String>, ObjectBytes, Vec<ObjectBytes>>>(name)?
        .map(|value| match value {
            AnyOf4::A(value) => Ok(vec![value.into_bytes()]),
            AnyOf4::B(values) => Ok(values.into_iter().map(String::into_bytes).collect()),
            AnyOf4::C(value) => Ok(vec![value.into_bytes(ctx)?]),
            AnyOf4::D(values) => values
                .into_iter()
                .map(|value| value.into_bytes(ctx))
                .collect::<Result<Vec<_>>>(),
        })
        .transpose()
}

/// Like [`get_pem_list`], but only one value is supported: a single client
/// certificate chain and private key pair.
fn get_single_pem<'js>(ctx: &Ctx<'js>, obj: &Object<'js>, name: &str) -> Result<Option<Vec<u8>>> {
    let Some(mut values) = get_pem_list(ctx, obj, name)? else {
        return Ok(None);
    };
    if values.len() > 1 {
        return Err(Exception::throw_type(
            ctx,
            &format!("Only one {name} is supported"),
        ));
    }
    Ok(values.pop())
}
