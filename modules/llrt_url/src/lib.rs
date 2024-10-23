// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::inherent_to_string)]
pub mod url_class;
pub mod url_search_params;

use std::{path::PathBuf, str::FromStr};

use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    function::{Constructor, Func},
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    Class, Coerced, Ctx, Exception, Result, Value,
};
use url::{quirks, Url};

use self::url_class::{url_to_http_options, URL};
use self::url_search_params::URLSearchParams;

pub fn domain_to_unicode(domain: &str) -> String {
    quirks::domain_to_unicode(domain)
}

pub fn domain_to_ascii(domain: &str) -> String {
    quirks::domain_to_ascii(domain)
}

//options are ignored, no windows support yet
pub fn path_to_file_url<'js>(ctx: Ctx<'js>, path: String, _: Opt<Value>) -> Result<URL<'js>> {
    let url = Url::from_file_path(&path)
        .map_err(|_| Exception::throw_type(&ctx, &["Path is not absolute: ", &path].concat()))?;

    URL::from_url(ctx, url)
}

//options are ignored, no windows support yet
pub fn file_url_to_path<'js>(ctx: Ctx<'js>, url: Value<'js>) -> Result<String> {
    let url_string = if let Ok(url) = Class::<URL>::from_value(&url) {
        url.borrow().to_string()
    } else {
        url.get::<Coerced<String>>()?.to_string()
    };

    let path = if let Some(path) = &url_string.strip_prefix("file://") {
        path.to_string()
    } else {
        url_string
    };

    Ok(PathBuf::from_str(&path)
        .or_throw(&ctx)?
        .to_string_lossy()
        .to_string())
}

pub fn url_format<'js>(url: Class<'js, URL<'js>>, options: Opt<Value<'js>>) -> Result<String> {
    let url = url.borrow();
    let mut string = url.protocol();
    string.push_str("//");

    let mut include_fragment = true;
    let mut unicode_encode = false;
    let mut include_auth = true;
    let mut include_search = true;

    // Parse options if provided
    if let Some(options) = options.into_inner() {
        if let Some(options) = options.as_object() {
            if let Ok(value) = options.get("unicode") {
                unicode_encode = value;
            }
            if let Ok(value) = options.get("auth") {
                include_auth = value;
            }
            if let Ok(value) = options.get("fragment") {
                include_fragment = value;
            }
            if let Ok(value) = options.get("search") {
                include_search = value
            }
        }
    }

    if include_auth {
        let username = url.username();
        let password = url.password();
        if !username.is_empty() {
            string.push_str(&username);
            if !password.is_empty() {
                string.push(':');
                string.push_str(&password);
            }
            string.push('@');
        }
    }

    if unicode_encode {
        string.push_str(&domain_to_unicode(&url.host()));
    } else {
        string.push_str(&url.host());
    }

    string.push_str(&url.pathname());

    if include_search {
        string.push_str(&url.search());
    }

    if include_fragment {
        string.push_str(&url.hash());
    }

    Ok(string)
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<URLSearchParams>::define(&globals)?;
    Class::<URL>::define(&globals)?;

    Ok(())
}

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
