// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_events::Emitter;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Result, Value,
};

mod socket;

use self::socket::Socket;

pub struct DgramModule;

impl ModuleDef for DgramModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createSocket")?;
        declare.declare(stringify!(Socket))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Socket>::define(default)?;
            Socket::add_event_emitter_prototype(ctx)?;

            default.set(
                "createSocket",
                Func::from(|ctx: Ctx<'js>, type_or_options: Value<'js>| {
                    Socket::ctor(ctx, type_or_options)
                }),
            )?;

            Ok(())
        })?;
        Ok(())
    }
}

impl From<DgramModule> for ModuleInfo<DgramModule> {
    fn from(val: DgramModule) -> Self {
        ModuleInfo {
            name: "dgram",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_module_loads() {
        test_async_with(|ctx| {
            Box::pin(async move {
                // Test that the dgram module can be loaded without errors
                let result = ModuleEvaluator::eval_rust::<DgramModule>(ctx.clone(), "dgram").await;
                assert!(result.is_ok(), "Dgram module should load successfully");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_socket_function_exists() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<DgramModule>(ctx.clone(), "dgram")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import dgram from 'dgram';
                        typeof dgram.createSocket === 'function'
                    "#,
                )
                .await;

                assert!(result.is_ok(), "createSocket should be accessible");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_socket_creation() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<DgramModule>(ctx.clone(), "dgram")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import dgram from 'dgram';
                        try {
                            const socket = dgram.createSocket('udp4');
                            true
                        } catch (e) {
                            false
                        }
                    "#,
                )
                .await;

                assert!(result.is_ok(), "Socket creation should not throw");
            })
        })
        .await;
    }
}
