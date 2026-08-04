// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{fmt::Display, result::Result as StdResult};

use llrt_exceptions::DOMException;
use rquickjs::{Ctx, Error, Result};

use crate::provider::CryptoError;

pub trait IntoDomException {
    fn into_dom_exception(self, ctx: &Ctx, msg: &str) -> Error;
}

impl IntoDomException for CryptoError {
    fn into_dom_exception(self, ctx: &Ctx, msg: &str) -> Error {
        let message = with_message(&self, msg);
        match self {
            CryptoError::UnsupportedAlgorithm => DOMException::not_supported_error(ctx, message),
            CryptoError::InvalidLength
            | CryptoError::InvalidKey(_)
            | CryptoError::InvalidData(_)
            | CryptoError::InvalidSignature(_) => DOMException::data_error(ctx, message),
            CryptoError::SigningFailed(_)
            | CryptoError::VerificationFailed
            | CryptoError::OperationFailed(_)
            | CryptoError::DerivationFailed(_)
            | CryptoError::EncryptionFailed(_)
            | CryptoError::DecryptionFailed(_) => DOMException::operation_error(ctx, message),
            CryptoError::InvalidAccess(_) => DOMException::invalid_access_error(ctx, message),
        }
    }
}

pub trait ResultDomExt<T> {
    fn or_throw_dom(self, ctx: &Ctx) -> Result<T>;
    fn or_throw_dom_with_msg(self, ctx: &Ctx, msg: &str) -> Result<T>;
}

impl<T, E: IntoDomException> ResultDomExt<T> for StdResult<T, E> {
    fn or_throw_dom(self, ctx: &Ctx) -> Result<T> {
        self.map_err(|e| e.into_dom_exception(ctx, ""))
    }
    fn or_throw_dom_with_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| e.into_dom_exception(ctx, msg))
    }
}

impl<T> ResultDomExt<T> for Option<T> {
    fn or_throw_dom(self, ctx: &Ctx) -> Result<T> {
        self.ok_or_else(|| DOMException::not_supported_error(ctx, "Value is not present"))
    }
    fn or_throw_dom_with_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        let message = if msg.is_empty() {
            "Value is not present"
        } else {
            msg
        };
        self.ok_or_else(|| DOMException::not_supported_error(ctx, message))
    }
}

pub struct NotSupportedError<E>(pub E);

impl<E: Display> IntoDomException for NotSupportedError<E> {
    fn into_dom_exception(self, ctx: &Ctx, msg: &str) -> Error {
        DOMException::not_supported_error(ctx, with_message(self.0, msg))
    }
}

fn with_message<E: Display>(err: E, msg: &str) -> String {
    if msg.is_empty() {
        err.to_string()
    } else {
        [msg, ": ", &err.to_string()].concat()
    }
}
