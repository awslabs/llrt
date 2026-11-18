// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use llrt_utils::{
    any_of::AnyOf4,
    bytes::{get_lossy_string, ObjectBytes},
    object::ObjectExt,
};
use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Coerced, Constructor, Ctx, Error, Exception, FromJs,
    Object, Result, Type, Value,
};

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
            passphrase,
        } = options.0.unwrap_or_default();

        let config = llrt_tls::build_client_config(llrt_tls::BuildClientConfigOptions {
            reject_unauthorized: reject_unauthorized.unwrap_or(true),
            ca,
            cert,
            key,
            passphrase,
        })
        .map_err(|e| throw_tls_error(&ctx, e))?;
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
    passphrase: Option<Vec<u8>>,
}

/// Throws an error that Node.js reports with OpenSSL's details as Node does,
/// and any other error with the "Failed to build TLS config. " prefix.
fn throw_tls_error(ctx: &Ctx<'_>, error: Box<dyn std::error::Error + Send + Sync>) -> Error {
    let result = match error.downcast::<llrt_tls::TlsError>() {
        Ok(error) => tls_exception(ctx, &error),
        Err(error) => {
            return Exception::throw_message(ctx, &format!("Failed to build TLS config. {error}"))
        },
    };
    match result {
        Ok(exception) => ctx.throw(exception),
        Err(e) => e,
    }
}

fn tls_exception<'js>(ctx: &Ctx<'js>, error: &llrt_tls::TlsError) -> Result<Value<'js>> {
    let exception = Exception::from_message(ctx.clone(), &error.message)?;
    if !error.openssl_error_stack.is_empty() {
        exception.set("opensslErrorStack", error.openssl_error_stack.clone())?;
    }
    for (name, value) in [
        ("library", &error.library),
        ("reason", &error.reason),
        ("code", &error.code),
    ] {
        if let Some(value) = value {
            exception.set(name, value.as_ref())?;
        }
    }
    Ok(exception.into_value())
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
            passphrase: get_passphrase(ctx, obj)?,
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

/// Reads `passphrase`, which Node.js requires to be a string when there is a
/// key to load. `null` and `undefined` mean none.
fn get_passphrase<'js>(ctx: &Ctx<'js>, obj: &Object<'js>) -> Result<Option<Vec<u8>>> {
    let key: Value = obj.get("key")?;
    let has_key = match key.as_array() {
        Some(keys) => !keys.is_empty(),
        None => key.get::<Coerced<bool>>()?.0,
    };
    let passphrase: Value = obj.get("passphrase")?;
    if !has_key || passphrase.is_undefined() || passphrase.is_null() {
        return Ok(None);
    }
    if !passphrase.is_string() {
        return Err(throw_invalid_arg_type(
            ctx,
            "options.passphrase",
            "string",
            &passphrase,
        ));
    }
    Ok(Some(get_lossy_string(passphrase)?.into_bytes()))
}

/// Throws Node.js's `ERR_INVALID_ARG_TYPE` for a property.
fn throw_invalid_arg_type(ctx: &Ctx<'_>, name: &str, expected: &str, value: &Value<'_>) -> Error {
    let error = || -> Result<Value<'_>> {
        let message = format!(
            "The \"{name}\" property must be of type {expected}. Received {}",
            describe_received(value)?
        );
        let constructor: Constructor = ctx.globals().get(PredefinedAtom::TypeError)?;
        let error: Object = constructor.construct((message,))?;
        error.set("code", "ERR_INVALID_ARG_TYPE")?;
        Ok(error.into_value())
    };
    match error() {
        Ok(error) => ctx.throw(error),
        Err(e) => e,
    }
}

/// How Node.js's `ERR_INVALID_ARG_TYPE` describes a value that is neither a
/// string, `null` nor `undefined`.
fn describe_received(value: &Value<'_>) -> Result<String> {
    let to_string = |value: &Value<'_>| value.get::<Coerced<String>>().map(|s| s.0);
    Ok(match value.type_of() {
        Type::Bool => format!("type boolean ({})", to_string(value)?),
        Type::Int | Type::Float => match value.as_number() {
            Some(n) if n == 0.0 && n.is_sign_negative() => "type number (-0)".into(),
            _ => format!("type number ({})", to_string(value)?),
        },
        Type::BigInt => format!("type bigint ({}n)", to_string(value)?),
        Type::Symbol => {
            let description = value.as_symbol().map(|s| s.description()).transpose()?;
            let description = match description {
                Some(d) if !d.is_undefined() => to_string(&d)?,
                _ => String::new(),
            };
            format!("type symbol (Symbol({description}))")
        },
        _ => {
            let Some(object) = value.as_object() else {
                return to_string(value);
            };
            if value.is_function() {
                let name: Value = object.get("name")?;
                return Ok(format!("function {}", to_string(&name)?));
            }
            let constructor: Value = object.get("constructor")?;
            match constructor.as_object() {
                Some(constructor) if constructor.contains_key("name")? => {
                    let name: Value = constructor.get("name")?;
                    format!("an instance of {}", to_string(&name)?)
                },
                _ if object.get_prototype().is_none() => {
                    if object.keys::<Value>().next().is_none() {
                        "[Object: null prototype] {}".into()
                    } else {
                        "[Object: null prototype]".into()
                    }
                },
                _ => "[Object]".into(),
            }
        },
    })
}
