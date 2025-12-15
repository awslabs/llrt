// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(
    any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"),
    not(feature = "tls-openssl")
))]
mod rustls_config;

#[cfg(all(
    any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"),
    not(feature = "tls-openssl")
))]
pub use rustls_config::*;

#[cfg(all(
    any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"),
    not(feature = "tls-openssl")
))]
mod no_verification;

#[cfg(feature = "tls-openssl")]
mod openssl_config;

#[cfg(feature = "tls-openssl")]
pub use openssl_config::*;

// Once we are ready to add the node TLS module, it should be here.
// Right now this module is supporting the https/fetch modules.
