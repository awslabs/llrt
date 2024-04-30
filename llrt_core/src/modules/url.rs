// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    function::{Constructor, Func},
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default};

use super::http::url::{
    domain_to_ascii, domain_to_unicode, file_url_to_path, path_to_file_url, url_format,
    url_to_http_options,
};
pub struct UrlModule;

impl ModuleDef for UrlModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(URL))?;
        declare.declare(stringify!(URLSearchParams))?;
        declare.declare("urlToHttpOptions")?;
        declare.declare("domainToUnicode")?;
        declare.declare("domainToASCII")?;
        declare.declare("fileURLToPath")?;
        declare.declare("pathToFileURL")?;
        declare.declare("format")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let url: Constructor = globals.get(stringify!(URL))?;
        let url_search_params: Constructor = globals.get(stringify!(URLSearchParams))?;

        export_default(ctx, exports, |default| {
            default.set(stringify!(URL), url)?;
            default.set(stringify!(URLSearchParams), url_search_params)?;
            default.set("urlToHttpOptions", Func::from(url_to_http_options))?;
            default.set(
                "domainToUnicode",
                Func::from(|domain: String| domain_to_unicode(&domain)),
            )?;
            default.set(
                "domainToASCII",
                Func::from(|domain: String| domain_to_ascii(&domain)),
            )?;
            default.set("fileURLToPath", Func::from(file_url_to_path))?;
            default.set("pathToFileURL", Func::from(path_to_file_url))?;
            default.set("format", Func::from(url_format))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<UrlModule> for ModuleInfo<UrlModule> {
    fn from(val: UrlModule) -> Self {
        ModuleInfo {
            name: "url",
            module: val,
        }
    }
}
