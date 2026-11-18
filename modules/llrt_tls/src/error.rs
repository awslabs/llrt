// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::borrow::Cow;
use std::fmt;

/// An error that Node.js reports with OpenSSL's details. `https.Agent`
/// throws it as is: `message`, plus `code`, `library`, `reason` and
/// `opensslErrorStack` when they are set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsError {
    pub message: Cow<'static, str>,
    pub code: Option<Cow<'static, str>>,
    pub library: Option<&'static str>,
    pub reason: Option<&'static str>,
    pub openssl_error_stack: Vec<String>,
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TlsError {}
