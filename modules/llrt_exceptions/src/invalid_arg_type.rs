// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom, function::Constructor, Coerced, Ctx, Error, Object, Result,
    String as JsString, Type, Value,
};

/// Throws Node.js's `ERR_INVALID_ARG_TYPE` for a property; covers only non-string values, until a caller needs more.
pub fn throw_invalid_arg_type(
    ctx: &Ctx<'_>,
    name: &str,
    expected: &str,
    value: &Value<'_>,
) -> Error {
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

// QuickJS keeps lone surrogates as WTF-8, so the bytes may not be valid UTF-8.
fn push_js_string(message: &mut String, value: &Value<'_>) -> Result<()> {
    let string = value.get::<Coerced<JsString>>()?.0.to_cstring()?;
    message.push_str(&String::from_utf8_lossy(string.as_bytes()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Runtime};

    use super::*;

    #[test]
    fn matches_node() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            for (source, received) in [
                ("12345678", "type number (12345678)"),
                ("-1234.5", "type number (-1234.5)"),
                ("1e21", "type number (1e+21)"),
                ("NaN", "type number (NaN)"),
                ("-0", "type number (-0)"),
                ("true", "type boolean (true)"),
                ("5n", "type bigint (5n)"),
                ("Symbol('d')", "type symbol (Symbol(d))"),
                ("Symbol()", "type symbol (Symbol())"),
                ("[]", "an instance of Array"),
                ("new Map()", "an instance of Map"),
                ("({})", "an instance of Object"),
                ("Object.create(null)", "[Object: null prototype] {}"),
                (
                    "Object.assign(Object.create(null), { a: 1 })",
                    "[Object: null prototype]",
                ),
                ("({ constructor: undefined })", "[Object]"),
                ("(function f() {})", "function f"),
                ("(class Foo {})", "function Foo"),
                ("null", "null"),
                ("undefined", "undefined"),
            ] {
                let value: Value = ctx.eval(source).unwrap();
                assert!(matches!(
                    throw_invalid_arg_type(&ctx, "options.key", "string", &value),
                    Error::Exception
                ));
                let error = ctx.catch().into_object().unwrap();
                assert_eq!(
                    error.get::<_, String>("message").unwrap(),
                    [
                        "The \"options.key\" property must be of type string. Received ",
                        received,
                    ]
                    .concat(),
                    "{source}"
                );
                assert_eq!(
                    error.get::<_, String>("code").unwrap(),
                    "ERR_INVALID_ARG_TYPE"
                );
            }
        });
    }
}
