// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod style_text;
pub mod text_decoder;
pub mod text_encoder;

use llrt_logging::{build_formatted_string, format_plain, FormatOptions};
use llrt_utils::{
    class::CUSTOM_INSPECT_SYMBOL_DESCRIPTION,
    module::{export_default, ModuleInfo},
    object::ObjectExt,
};
use rquickjs::{
    function::{Func, Opt, Rest},
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Function, Object, Result, Symbol, Value,
};
use style_text::style_text;
use text_decoder::TextDecoder;
use text_encoder::TextEncoder;

fn inherits<'js>(ctor: Function<'js>, super_ctor: Function<'js>) -> Result<()> {
    let super_proto: Object<'js> = super_ctor.get("prototype")?;
    let proto: Object<'js> = ctor.get("prototype")?;
    proto.set_prototype(Some(&super_proto))?;
    ctor.set("super_", super_ctor)?;
    Ok(())
}

fn inspect<'js>(ctx: Ctx<'js>, value: Value<'js>, options: Opt<Object<'js>>) -> Result<String> {
    let colors = options
        .0
        .and_then(|opts| opts.get_optional("colors").ok().flatten())
        .unwrap_or(false);

    let mut result = String::new();
    let mut format_options = FormatOptions::new(&ctx, colors, true)?;
    build_formatted_string(&mut result, &ctx, Rest(vec![value]), &mut format_options)?;
    Ok(result)
}

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(format))?;
        declare.declare(stringify!(inherits))?;
        declare.declare(stringify!(styleText))?;
        declare.declare(stringify!(inspect))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set(
                "format",
                Func::from(|ctx, args| format_plain(ctx, true, args)),
            )?;
            default.set("inherits", Func::from(inherits))?;
            default.set("styleText", Func::from(style_text))?;

            let inspect_fn = Function::new(ctx.clone(), inspect)?.with_name("inspect")?;
            let inspect_custom =
                Symbol::new_global(ctx.clone(), CUSTOM_INSPECT_SYMBOL_DESCRIPTION)?;
            inspect_fn.set("custom", inspect_custom)?;
            default.set("inspect", inspect_fn)?;

            Ok(())
        })
    }
}

impl From<UtilModule> for ModuleInfo<UtilModule> {
    fn from(val: UtilModule) -> Self {
        ModuleInfo {
            name: "util",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::UtilModule;
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};
    use llrt_utils::primordials::{BasePrimordials, Primordial};

    #[tokio::test]
    async fn test_inspect() {
        test_async_with(|ctx| {
            Box::pin(async move {
                BasePrimordials::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<UtilModule>(ctx.clone(), "util")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { inspect } from 'util';

                        export async function test() {
                            return inspect({ a: 1, b: [1, 2] });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "{\n  a: 1,\n  b: [ 1, 2 ]\n}");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_inspect_custom() {
        test_async_with(|ctx| {
            Box::pin(async move {
                BasePrimordials::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<UtilModule>(ctx.clone(), "util")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { inspect } from 'util';

                        export async function test() {
                            const obj = {
                                [inspect.custom]: { customKey: "custom-value" },
                            };
                            return inspect(obj);
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "{\n  customKey: 'custom-value'\n}");
            })
        })
        .await;
    }
}
