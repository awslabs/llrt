// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{fmt::Write, result::Result as StdResult};

use rquickjs::{Ctx, Exception, Result};

pub enum By {
    Internal,
    Message,
    Range,
    Reference,
    Syntax,
    Type,
}

pub trait ResultExt<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T>;
    fn or_throw_range(self, ctx: &Ctx, msg: &str) -> Result<T>;
    fn or_throw_type(self, ctx: &Ctx, msg: &str) -> Result<T>;
    fn or_throw(self, ctx: &Ctx) -> Result<T>;
    fn or_throw_by(self, ctx: &Ctx, by: By) -> Result<T>;
}

pub trait OptionExt<T> {
    fn and_then_ok<U, E, F>(self, f: F) -> StdResult<Option<U>, E>
    where
        F: FnOnce(T) -> StdResult<Option<U>, E>;

    fn unwrap_or_else_ok<E, F>(self, f: F) -> StdResult<T, E>
    where
        F: FnOnce() -> StdResult<T, E>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for StdResult<T, E> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| {
            let mut message = String::with_capacity(100);
            message.push_str(msg);
            message.push_str(". ");
            write!(message, "{}", e).unwrap();
            Exception::throw_message(ctx, &message)
        })
    }

    fn or_throw_range(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| {
            let mut message = String::with_capacity(100);
            if !message.is_empty() {
                message.push_str(msg);
                message.push_str(". ");
            }
            write!(message, "{}", e).unwrap();
            Exception::throw_range(ctx, &message)
        })
    }

    fn or_throw_type(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| {
            let mut message = String::with_capacity(100);
            if !msg.is_empty() {
                message.push_str(msg);
                message.push_str(". ");
            }
            write!(message, "{}", e).unwrap();
            Exception::throw_type(ctx, &message)
        })
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.map_err(|err| Exception::throw_message(ctx, &err.to_string()))
    }

    fn or_throw_by(self, ctx: &Ctx, by: By) -> Result<T> {
        match by {
            By::Internal => self.map_err(|err| Exception::throw_internal(ctx, &err.to_string())),
            By::Message => self.map_err(|err| Exception::throw_message(ctx, &err.to_string())),
            By::Range => self.map_err(|err| Exception::throw_range(ctx, &err.to_string())),
            By::Reference => self.map_err(|err| Exception::throw_reference(ctx, &err.to_string())),
            By::Syntax => self.map_err(|err| Exception::throw_syntax(ctx, &err.to_string())),
            By::Type => self.map_err(|err| Exception::throw_type(ctx, &err.to_string())),
        }
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.ok_or_else(|| Exception::throw_message(ctx, msg))
    }

    fn or_throw_range(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.ok_or_else(|| Exception::throw_range(ctx, msg))
    }

    fn or_throw_type(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.ok_or_else(|| Exception::throw_type(ctx, msg))
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.ok_or_else(|| Exception::throw_message(ctx, "Value is not present"))
    }

    fn or_throw_by(self, ctx: &Ctx, kind: By) -> Result<T> {
        let msg = "Value is not present";
        match kind {
            By::Internal => self.ok_or_else(|| Exception::throw_internal(ctx, msg)),
            By::Message => self.ok_or_else(|| Exception::throw_message(ctx, msg)),
            By::Range => self.ok_or_else(|| Exception::throw_range(ctx, msg)),
            By::Reference => self.ok_or_else(|| Exception::throw_reference(ctx, msg)),
            By::Syntax => self.ok_or_else(|| Exception::throw_syntax(ctx, msg)),
            By::Type => self.ok_or_else(|| Exception::throw_type(ctx, msg)),
        }
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn and_then_ok<U, E, F>(self, f: F) -> StdResult<Option<U>, E>
    where
        F: FnOnce(T) -> StdResult<Option<U>, E>,
    {
        match self {
            Some(v) => f(v),
            None => Ok(None),
        }
    }

    fn unwrap_or_else_ok<E, F>(self, f: F) -> StdResult<T, E>
    where
        F: FnOnce() -> StdResult<T, E>,
    {
        match self {
            Some(v) => Ok(v),
            None => f(),
        }
    }
}
