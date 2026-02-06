// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};

pub use self::config::*;

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod client;
mod config;

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod agent;

#[cfg(any(
    feature = "tls-rust",
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
pub use self::{agent::Agent, client::*};

pub struct HttpsModule;

impl ModuleDef for HttpsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        #[cfg(any(
            feature = "tls-rust",
            feature = "tls-ring",
            feature = "tls-aws-lc",
            feature = "tls-graviola",
            feature = "tls-openssl"
        ))]
        declare.declare(stringify!(Agent))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            #[cfg(any(
                feature = "tls-rust",
                feature = "tls-ring",
                feature = "tls-aws-lc",
                feature = "tls-graviola",
                feature = "tls-openssl"
            ))]
            rquickjs::Class::<Agent>::define(default)?;

            let _ = default;
            Ok(())
        })
    }
}

impl From<HttpsModule> for ModuleInfo<HttpsModule> {
    fn from(val: HttpsModule) -> Self {
        ModuleInfo {
            name: "https",
            module: val,
        }
    }
}
