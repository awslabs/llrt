// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::result::Result as StdResult;

use rquickjs::{Ctx, Exception, Result};

pub trait ResultExt<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T>;
    fn or_throw(self, ctx: &Ctx) -> Result<T>;
}

pub trait OptionExt<T> {
    fn and_then_ok<U, E, F>(self, f: F) -> StdResult<Option<U>, E>
    where
        F: FnOnce(T) -> StdResult<Option<U>, E>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for StdResult<T, E> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| Exception::throw_message(ctx, &format!("{}. {}", msg, &e.to_string())))
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.map_err(|err| Exception::throw_message(ctx, &err.to_string()))
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.ok_or(Exception::throw_message(ctx, msg))
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.ok_or(Exception::throw_message(ctx, "Value is not present"))
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
}
