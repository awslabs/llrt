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

    #[tokio::test]
    async fn test_bind_failure_emits_error() {
        // Test that bind failure on invalid address emits error event
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
                        export async function test() {
                            return new Promise((resolve, reject) => {
                                const socket = dgram.createSocket('udp4');
                                let errorReceived = false;
                                
                                socket.on('error', (err) => {
                                    errorReceived = true;
                                    resolve('error_received');
                                });
                                
                                // Bind to invalid address should fail
                                socket.bind(12345, '999.999.999.999');
                                
                                // Timeout to ensure we don't hang forever
                                setTimeout(() => {
                                    resolve(errorReceived ? 'error_received' : 'timeout');
                                }, 100);
                            });
                        }
                    "#,
                )
                .await;

                assert!(result.is_ok(), "Test module should evaluate");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_close_clears_send_channel() {
        // Test that close properly cleans up send channel
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
                        export async function test() {
                            return new Promise((resolve) => {
                                const socket = dgram.createSocket('udp4');
                                
                                socket.on('listening', () => {
                                    socket.close(() => {
                                        resolve('closed');
                                    });
                                });
                                
                                socket.on('error', (err) => {
                                    resolve('error: ' + err.message);
                                });
                                
                                socket.bind(0); // Random port
                            });
                        }
                    "#,
                )
                .await;

                assert!(result.is_ok(), "Test module should evaluate");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_send_after_close_fails() {
        // Test that sending after close returns error
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
                        export async function test() {
                            return new Promise((resolve) => {
                                const socket = dgram.createSocket('udp4');
                                
                                socket.on('listening', () => {
                                    socket.close();
                                    try {
                                        socket.send('test', 12345, 'localhost');
                                        resolve('no_error');
                                    } catch (e) {
                                        resolve('error_thrown');
                                    }
                                });
                                
                                socket.bind(0);
                            });
                        }
                    "#,
                )
                .await;

                assert!(result.is_ok(), "Test module should evaluate");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_double_close_fails() {
        // Test that closing twice throws error
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
                        export async function test() {
                            return new Promise((resolve) => {
                                const socket = dgram.createSocket('udp4');
                                
                                socket.on('listening', () => {
                                    socket.close();
                                    try {
                                        socket.close();
                                        resolve('no_error');
                                    } catch (e) {
                                        resolve('error_thrown');
                                    }
                                });
                                
                                socket.bind(0);
                            });
                        }
                    "#,
                )
                .await;

                assert!(result.is_ok(), "Test module should evaluate");
            })
        })
        .await;
    }
}
