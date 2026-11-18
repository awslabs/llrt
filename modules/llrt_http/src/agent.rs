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
        .or_else(|error| match error.downcast::<llrt_tls::TlsError>() {
            Ok(error) => Err(throw_tls_error(&ctx, *error)),
            Err(error) => Err(error).or_throw_msg(&ctx, "Failed to build TLS config"),
        })?;
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

/// Throws an error that Node.js reports with OpenSSL's details, as Node.js
/// does.
fn throw_tls_error(ctx: &Ctx<'_>, error: llrt_tls::TlsError) -> Error {
    let exception = || -> Result<Value<'_>> {
        let exception = Exception::from_message(ctx.clone(), &error.message)?;
        if !error.openssl_error_stack.is_empty() {
            exception.set("opensslErrorStack", error.openssl_error_stack)?;
        }
        for (name, value) in [
            ("library", error.library),
            ("reason", error.reason),
            ("code", error.code.as_deref()),
        ] {
            if let Some(value) = value {
                exception.set(name, value)?;
            }
        }
        Ok(exception.into_value())
    };
    match exception() {
        Ok(exception) => ctx.throw(exception),
        Err(e) => e,
    }
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
        let mut message = String::with_capacity(96);
        message.push_str("The \"");
        message.push_str(name);
        message.push_str("\" property must be of type ");
        message.push_str(expected);
        message.push_str(". Received ");
        push_received(&mut message, value)?;
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

/// Appends how Node.js's `ERR_INVALID_ARG_TYPE` describes a value that is
/// neither a string, `null` nor `undefined`.
fn push_received(message: &mut String, value: &Value<'_>) -> Result<()> {
    match value.type_of() {
        Type::Bool => message.push_str(match value.as_bool() {
            Some(true) => "type boolean (true)",
            _ => "type boolean (false)",
        }),
        Type::Int | Type::Float => {
            message.push_str("type number (");
            match value.as_number() {
                Some(n) if n == 0.0 && n.is_sign_negative() => message.push_str("-0"),
                _ => push_js_string(message, value)?,
            }
            message.push(')');
        },
        Type::BigInt => {
            message.push_str("type bigint (");
            push_js_string(message, value)?;
            message.push_str("n)");
        },
        Type::Symbol => {
            message.push_str("type symbol (Symbol(");
            if let Some(symbol) = value.as_symbol() {
                let description = symbol.description()?;
                if !description.is_undefined() {
                    push_js_string(message, &description)?;
                }
            }
            message.push_str("))");
        },
        _ => {
            let Some(object) = value.as_object() else {
                return push_js_string(message, value);
            };
            if value.is_function() {
                message.push_str("function ");
                return push_js_string(message, &object.get("name")?);
            }
            let constructor: Value = object.get("constructor")?;
            match constructor.as_object() {
                Some(constructor) if constructor.contains_key("name")? => {
                    message.push_str("an instance of ");
                    push_js_string(message, &constructor.get("name")?)?;
                },
                _ if object.get_prototype().is_none() => {
                    message.push_str("[Object: null prototype]");
                    if object.keys::<Value>().next().is_none() {
                        message.push_str(" {}");
                    }
                },
                _ => message.push_str("[Object]"),
            }
        },
    }
    Ok(())
}

/// Appends JavaScript's `String(value)`.
fn push_js_string(message: &mut String, value: &Value<'_>) -> Result<()> {
    let string = value.get::<Coerced<rquickjs::String>>()?.0;
    message.push_str(&string.to_cstring()?);
    Ok(())
}
