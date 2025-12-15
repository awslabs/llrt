// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
mod rustls_config;

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
pub use rustls_config::*;

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
mod no_verification;

#[cfg(feature = "tls-openssl")]
mod openssl_config;

#[cfg(feature = "tls-openssl")]
pub use openssl_config::*;

// Once we are ready to add the node TLS module, it should be here.
// Right now this module is supporting the https/fetch modules.
