// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result,
};

fn isatty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) != 0 }
}

pub struct TtyModule;

impl ModuleDef for TtyModule {
    fn declare(declare: &Declarations<'_>) -> Result<()> {
        declare.declare("isatty")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("isatty", Func::from(isatty))?;
            Ok(())
        })
    }
}

impl From<TtyModule> for ModuleInfo<TtyModule> {
    fn from(val: TtyModule) -> Self {
        ModuleInfo {
            name: "tty",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TtyModule;
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};
    use std::io::{stderr, stdin, stdout, IsTerminal};

    #[tokio::test]
    async fn test_isatty() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TtyModule>(ctx.clone(), "tty")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { isatty } from 'tty';

                        export async function test() {
                            return new Array(3).fill(0).map((_, i) => +isatty(i)).join('')
                        }
                    "#,
                )
                .await
                .unwrap();
                let expect = [
                    stdin().is_terminal(),
                    stdout().is_terminal(),
                    stderr().is_terminal(),
                ]
                .map(|i| (i as u8).to_string())
                .join("");
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, expect);
            })
        })
        .await;
    }
}
