// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//! `process.stdout` and `process.stderr`.
//!
//! Writes are synchronous and go through the same `std::io` handles that
//! `console` uses, so output stays in call order across both APIs and is
//! never lost to `process.exit()`. On Linux, Node.js is likewise synchronous
//! for files, TTYs and pipes.

use std::io::{self, IsTerminal, Write};

use llrt_encoding::Encoder;
use llrt_utils::{
    bytes::{get_lossy_string, ObjectBytes},
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{prelude::Opt, Coerced, Ctx, Exception, Function, Object, Result, Value};

pub fn init<'js>(ctx: &Ctx<'js>, process: &Object<'js>) -> Result<()> {
    let write = Function::new(ctx.clone(), write_stdout)?;
    let stdout = create_stream(ctx, io::stdout().is_terminal(), write)?;
    process.set("stdout", stdout)?;

    let write = Function::new(ctx.clone(), write_stderr)?;
    let stderr = create_stream(ctx, io::stderr().is_terminal(), write)?;
    process.set("stderr", stderr)?;

    Ok(())
}

fn create_stream<'js>(ctx: &Ctx<'js>, is_tty: bool, write: Function<'js>) -> Result<Object<'js>> {
    let stream = Object::new(ctx.clone())?;
    stream.set("write", write.with_name("write")?)?;
    // Node.js only defines `isTTY` on TTY streams, so it is absent otherwise.
    if is_tty {
        stream.set("isTTY", true)?;
    }
    Ok(stream)
}

fn write_stdout<'js>(
    ctx: Ctx<'js>,
    chunk: Opt<Value<'js>>,
    encoding: Opt<Value<'js>>,
    callback: Opt<Value<'js>>,
) -> Result<bool> {
    write(&ctx, io::stdout(), chunk.0, encoding.0, callback.0)
}

fn write_stderr<'js>(
    ctx: Ctx<'js>,
    chunk: Opt<Value<'js>>,
    encoding: Opt<Value<'js>>,
    callback: Opt<Value<'js>>,
) -> Result<bool> {
    write(&ctx, io::stderr(), chunk.0, encoding.0, callback.0)
}

/// `writable.write(chunk[, encoding][, callback])`
fn write<'js>(
    ctx: &Ctx<'js>,
    mut output: impl Write,
    chunk: Option<Value<'js>>,
    encoding: Option<Value<'js>>,
    callback: Option<Value<'js>>,
) -> Result<bool> {
    // A missing chunk is rejected like `undefined`.
    let chunk = chunk.unwrap_or_else(|| Value::new_undefined(ctx.clone()));
    // write(chunk, callback)
    let (encoding, callback) = match encoding {
        Some(value) if value.is_function() => (None, Some(value)),
        encoding => (encoding, callback),
    };
    let callback = callback.and_then(Value::into_function);
    let encoder = to_encoder(ctx, encoding)?;

    let bytes = to_bytes(ctx, chunk, encoder)?;
    let result = output
        .write_all(bytes.as_bytes(ctx)?)
        .and_then(|_| output.flush());

    match (result, callback) {
        (Ok(()), Some(callback)) => {
            callback.defer((Value::new_null(ctx.clone()),))?;
            Ok(true)
        },
        (Ok(()), None) => Ok(true),
        // Without an 'error' event to emit, report the failure to the callback
        // when there is one and throw otherwise, so it is never silently lost.
        (Err(err), Some(callback)) => {
            callback.defer((write_error(ctx, &err)?,))?;
            Ok(false)
        },
        (Err(err), None) => Err(ctx.throw(write_error(ctx, &err)?)),
    }
}

/// An `Error` carrying Node.js's `syscall`, and `code` for a broken pipe.
fn write_error<'js>(ctx: &Ctx<'js>, err: &io::Error) -> Result<Value<'js>> {
    let exception = Exception::from_message(ctx.clone(), &err.to_string())?;
    if err.kind() == io::ErrorKind::BrokenPipe {
        exception.set("code", "EPIPE")?;
    }
    exception.set("syscall", "write")?;
    Ok(exception.into_value())
}

/// Like Node.js, any falsy value (`undefined`, `null`, `""`, `false`, `0`,
/// `NaN`, ...) falls back to UTF-8. Any other value must name a Buffer encoding.
fn to_encoder<'js>(ctx: &Ctx<'js>, encoding: Option<Value<'js>>) -> Result<Encoder> {
    let Some(encoding) = encoding else {
        return Ok(Encoder::Utf8);
    };
    let Coerced(is_truthy) = encoding.get::<Coerced<bool>>()?;
    if !is_truthy {
        return Ok(Encoder::Utf8);
    }
    if let Some(string) = encoding.as_string() {
        if let Ok(encoder) = Encoder::for_buffer(&string.to_string()?) {
            return Ok(encoder);
        }
    }
    let Coerced(name) = encoding.get::<Coerced<String>>()?;
    Err(Exception::throw_type(
        ctx,
        &["Unknown encoding: ", &name].concat(),
    ))
}

fn to_bytes<'js>(ctx: &Ctx<'js>, chunk: Value<'js>, encoder: Encoder) -> Result<ObjectBytes<'js>> {
    if chunk.is_string() {
        let string = get_lossy_string(chunk)?;
        // Same error as `Buffer.from(string, encoding)`.
        let bytes = encoder.decode_from_string(string).or_throw(ctx)?;
        return Ok(ObjectBytes::Vec(bytes));
    }

    if chunk.is_null() {
        return Err(Exception::throw_type(
            ctx,
            "May not write null values to stream",
        ));
    }

    // Only a real ArrayBufferView (Buffer, TypedArray or DataView). Node.js
    // rejects a bare ArrayBuffer and objects that merely have a `buffer`.
    if let Some(obj) = chunk.as_object() {
        let is_view = BasePrimordials::get(ctx)?
            .function_array_buffer_is_view
            .clone();
        if is_view.call::<_, bool>((obj.clone(),))? {
            if let Some(bytes) = ObjectBytes::from_array_buffer(obj)? {
                return Ok(bytes);
            }
        }
    }

    Err(Exception::throw_type(
        ctx,
        "The \"chunk\" argument must be of type string or an instance of Buffer, TypedArray, or DataView",
    ))
}
