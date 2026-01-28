// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Result,
};

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
pub use self::agent::Agent;
#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
pub use self::client::*;
pub use self::config::*;

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod agent;
mod client;
mod config;

// Here we should also add the http module.

pub struct HttpsModule;

impl ModuleDef for HttpsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        #[cfg(any(
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
        export_default(ctx, exports, |_default| {
            #[cfg(any(
                feature = "tls-ring",
                feature = "tls-aws-lc",
                feature = "tls-graviola",
                feature = "tls-openssl"
            ))]
            Class::<Agent>::define(_default)?;

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
