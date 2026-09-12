// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Ensure only one TLS backend is selected
#[cfg(all(feature = "tls-rust", feature = "tls-ring"))]
compile_error!("Features `tls-rust` and `tls-ring` are mutually exclusive");

#[cfg(all(feature = "tls-rust", feature = "tls-aws-lc"))]
compile_error!("Features `tls-rust` and `tls-aws-lc` are mutually exclusive");

#[cfg(all(feature = "tls-rust", feature = "tls-graviola"))]
compile_error!("Features `tls-rust` and `tls-graviola` are mutually exclusive");

#[cfg(all(feature = "tls-rust", feature = "tls-openssl"))]
compile_error!("Features `tls-rust` and `tls-openssl` are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-aws-lc"))]
compile_error!("Features `tls-ring` and `tls-aws-lc` are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-graviola"))]
compile_error!("Features `tls-ring` and `tls-graviola` are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-openssl"))]
compile_error!("Features `tls-ring` and `tls-openssl` are mutually exclusive");

#[cfg(all(feature = "tls-aws-lc", feature = "tls-graviola"))]
compile_error!("Features `tls-aws-lc` and `tls-graviola` are mutually exclusive");

#[cfg(all(feature = "tls-aws-lc", feature = "tls-openssl"))]
compile_error!("Features `tls-aws-lc` and `tls-openssl` are mutually exclusive");

#[cfg(all(feature = "tls-graviola", feature = "tls-openssl"))]
compile_error!("Features `tls-graviola` and `tls-openssl` are mutually exclusive");

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola"
))]
mod rustls_config;

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola"
))]
pub use rustls_config::*;

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola"
))]
mod no_verification;

#[cfg(feature = "tls-openssl")]
mod openssl_config;

#[cfg(feature = "tls-openssl")]
pub use openssl_config::*;

// Once we are ready to add the node TLS module, it should be here.
// Right now this module is supporting the https/fetch modules.
