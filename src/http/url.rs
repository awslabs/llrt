// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Opt,
    prelude::This,
    Class, Ctx, Exception, FromJs, Function, Result, Value,
};
use url::Url;

use crate::utils::result::ResultExt;

use super::url_search_params::URLSearchParams;

#[derive(Clone)]
#[rquickjs::class]
pub struct URL<'js> {
    protocol: String,
    host: String,
    hostname: String,
    port: String,
    pathname: String,
    hash: String,
    search_params: Class<'js, URLSearchParams>,
}

impl<'js> Trace<'js> for URL<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.search_params.trace(tracer);
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> URL<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Result<Self> {
        if let Some(base) = base.0 {
            let base_string = get_string(&ctx, base)?;
            let path_string = get_string(&ctx, input)?;
            let base: Url = base_string.parse().or_throw_msg(&ctx, "Invalid URL")?;

            let url = base
                .join(path_string.as_str())
                .or_throw_msg(&ctx, "Invalid URL")?;
            return Self::create(ctx, url);
        }

        if input.is_string() {
            let string: String = input.get()?;
            Self::from_str(ctx, &string)
        } else if input.is_object() {
            Self::from_js(&ctx, input)
        } else {
            Err(Exception::throw_message(&ctx, "Invalid URL"))
        }
    }

    pub fn to_string(&self) -> String {
        let search = search_params_to_string(&self.search_params);
        let hash = &self.hash;
        let hash = if !hash.is_empty() {
            format!("#{}", &hash)
        } else {
            String::from("")
        };

        format!(
            "{}://{}{}{}{}",
            &self.protocol, &self.host, &self.pathname, &search, &hash
        )
    }

    #[qjs(get)]
    fn search_params(&self) -> Class<'js, URLSearchParams> {
        self.search_params.clone()
    }

    #[qjs(get)]
    fn href(&self) -> String {
        self.to_string()
    }

    #[qjs(set, rename = "href")]
    fn set_href(&mut self, ctx: Ctx<'js>, href: String) -> Result<String> {
        let new = Self::from_str(ctx, &href)?;

        self.protocol = new.protocol;
        self.host = new.host;
        self.hostname = new.hostname;
        self.port = new.port;
        self.pathname = new.pathname;
        self.hash = new.hash;
        self.search_params = new.search_params;
        Ok(href)
    }

    #[qjs(get)]
    fn origin(&self) -> String {
        format!("{}://{}", &self.protocol, &self.host)
    }

    #[qjs(get)]
    fn protocol(&self) -> String {
        format!("{}:", &self.protocol)
    }

    #[qjs(set, rename = "protocol")]
    fn set_protocol(&mut self, protocol: String) -> String {
        self.protocol = protocol.clone();
        protocol
    }

    #[qjs(get)]
    fn hostname(&self) -> String {
        self.hostname.clone()
    }

    #[qjs(set, rename = "hostname")]
    fn set_hostname(&mut self, hostname: String) -> String {
        self.hostname = hostname.clone();
        hostname
    }

    #[qjs(get)]
    fn host(&self) -> String {
        self.host.clone()
    }

    #[qjs(set, rename = "host")]
    fn set_host(&mut self, ctx: Ctx<'js>, host: String) -> Result<String> {
        let (name, port) = split_colon(&ctx, &host)?;
        self.hostname = name.to_string();
        self.port = port.to_string();
        if !port.is_empty() {
            self.host = format!("{}:{}", name, port);
        } else {
            self.host = name.to_string();
        }

        Ok(self.host.clone())
    }

    #[qjs(get)]
    fn pathname(&self) -> String {
        self.pathname.clone()
    }

    #[qjs(set, rename = "pathname")]
    fn set_pathname(&mut self, pathname: String) -> String {
        self.pathname = pathname.clone();
        pathname
    }

    #[qjs(get)]
    fn search(&self) -> String {
        search_params_to_string(&self.search_params)
    }

    #[qjs(set, rename = "search")]
    fn set_search(&mut self, ctx: Ctx<'js>, search: String) -> Result<String> {
        let search_params = URLSearchParams::from_str(&search);
        let search_params = Class::instance(ctx, search_params)?;
        self.search_params = search_params;
        Ok(search)
    }

    #[qjs(get)]
    fn hash(&self) -> String {
        self.hash.clone()
    }

    #[qjs(set, rename = "hash")]
    fn set_hash(&mut self, hash: String) -> String {
        let pound_hash = format!("#{}", &hash);
        self.hash = hash;
        pound_hash
    }
}

impl<'js> URL<'js> {
    fn create(ctx: Ctx<'js>, url: Url) -> Result<Self> {
        let query = url.query().unwrap_or_default();
        let search_params = URLSearchParams::from_str(query);

        let hostname = url.host().map(|h| h.to_string()).unwrap_or_default();
        let port = url.port().map(|p| p.to_string());
        let host = format!(
            "{}{}",
            &hostname,
            &port.clone().map(|p| format!(":{}", &p)).unwrap_or_default()
        );

        let search_params = Class::instance(ctx, search_params)?;

        Ok(Self {
            protocol: url.scheme().to_string(),
            host,
            hostname,
            port: port.unwrap_or_default(),
            pathname: url.path().to_string(),
            hash: url.fragment().map(|f| f.to_string()).unwrap_or_default(),
            search_params,
        })
    }

    pub fn from_str(ctx: Ctx<'js>, input: &str) -> Result<Self> {
        let url: Url = input.parse().or_throw_msg(&ctx, "Invalid URL")?;
        Self::create(ctx, url)
    }
}

fn get_string(ctx: &Ctx, input: Value) -> Result<String> {
    if input.is_string() {
        input.get()
    } else if input.is_object() {
        let obj = input.as_object().unwrap();
        let to_string_fn: Function = obj.get(PredefinedAtom::ToString)?;
        to_string_fn.call((This(input),))
    } else {
        Err(Exception::throw_type(ctx, "Invalid URL"))
    }
}

fn search_params_to_string(search_params: &Class<'_, URLSearchParams>) -> String {
    let search_params = search_params.borrow().to_string();

    if !search_params.is_empty() {
        format!("?{}", &search_params)
    } else {
        search_params
    }
}

fn split_colon<'js>(ctx: &Ctx, s: &'js str) -> Result<(&'js str, &'js str)> {
    let mut parts = s.split(':');
    let first = parts.next().unwrap_or("");
    let second = parts.next().unwrap_or("");
    if parts.next().is_some() || (first.is_empty() && second.is_empty()) {
        return Err(Exception::throw_message(
            ctx,
            "String contains more than one ':'",
        ));
    }
    Ok((first, second))
}
