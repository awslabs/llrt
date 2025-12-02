// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
use std::{cell::RefCell, rc::Rc};

use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, Class, Coerced, Ctx, Exception, FromJs,
    IntoJs, Null, Object, Result, Value,
};
use url::{quirks, Url};

use super::{convert_trailing_space, url_search_params::URLSearchParams};

/// Naively checks for hostname delimiter, a colon ":", that's *probably* not
/// part of an IPv6 address
///
/// # Arguments
///
/// * `hostname` - The hostname.
///
/// # Returns
///
/// Returns whether the hostname contains a colon that's not followed by a
/// closing square bracket.
fn has_colon_delimiter(hostname: &str) -> bool {
    if let Some(last_colon_index) = hostname.rfind(':') {
        // Check if there's any closing bracket after the last colon
        !hostname[last_colon_index..].contains(']')
    } else {
        false
    }
}

/// Represents a JavaScript
/// [`URL`](https://developer.mozilla.org/en-US/docs/Web/API/URL/URL) as defined
/// by the [WHATWG URL standard](https://url.spec.whatwg.org/) in the JavaScript
/// context.
///
/// # Examples
///
/// ```rust,ignore
/// // This is JavaScript
/// const url = new URL("https://url.spec.whatwg.org/");
/// console.log(url.href);
/// ```
#[derive(Clone, Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct URL<'js> {
    // URL and URLSearchParams work together to manipulate URLs, so using a
    // reference counter (Rc) allows them to have shared ownership of the
    // undering Url, and a RefCell allows interior mutability.
    #[qjs(skip_trace)]
    url: Rc<RefCell<Url>>,
    search_params: Class<'js, URLSearchParams>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> URL<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Result<Self> {
        let input: Result<Coerced<String>> = Coerced::from_js(&ctx, input);
        if let Some(base) = base.into_inner() {
            if let Some(base) = base.as_string() {
                if let Ok(base) = base.to_string() {
                    let mut url: Url = base.parse().map_err(|err| {
                        Exception::throw_type(&ctx, format!("Invalid base URL: {}", err).as_str())
                    })?;

                    if let Ok(input) = input {
                        url = url.join(input.as_str()).map_err(|err| {
                            Exception::throw_type(&ctx, format!("Invalid URL: {}", err).as_str())
                        })?;
                    }

                    return Self::from_url(ctx, url);
                }
            }
        }

        if let Ok(input) = input {
            Self::from_str(ctx, input.as_str())
        } else {
            Err(Exception::throw_message(&ctx, "Invalid URL"))
        }
    }

    //
    // Properties
    //

    #[qjs(get)]
    pub fn hash(&self) -> String {
        quirks::hash(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "hash")]
    pub fn set_hash(&mut self, hash: String) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        quirks::set_hash(&mut self.url.borrow_mut(), hash.as_str());
        hash
    }

    #[qjs(get)]
    pub fn host(&self) -> String {
        quirks::host(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "host")]
    pub fn set_host(&mut self, host: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        let _ = quirks::set_host(&mut self.url.borrow_mut(), host.as_str());
        host.0
    }

    #[qjs(get)]
    pub fn hostname(&self) -> String {
        quirks::hostname(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "hostname")]
    pub fn set_hostname(&mut self, hostname: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        // TODO: This should be fixed in Url
        if !has_colon_delimiter(hostname.as_str()) {
            let _ = quirks::set_hostname(&mut self.url.borrow_mut(), hostname.as_str());
        }
        hostname.0
    }

    #[qjs(get)]
    pub fn href(&self) -> String {
        quirks::href(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "href")]
    pub fn set_href(&mut self, href: String) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        let _ = quirks::set_href(&mut self.url.borrow_mut(), href.as_str());
        href
    }

    #[qjs(get)]
    pub fn origin(&self) -> String {
        quirks::origin(&self.url.borrow()).to_string()
    }

    #[qjs(get)]
    pub fn password(&self) -> String {
        quirks::password(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "password")]
    pub fn set_password(&mut self, password: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        let _ = quirks::set_password(&mut self.url.borrow_mut(), password.as_str());
        password.0
    }

    #[qjs(get)]
    pub fn pathname(&self) -> String {
        quirks::pathname(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "pathname")]
    pub fn set_pathname(&mut self, pathname: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        quirks::set_pathname(&mut self.url.borrow_mut(), pathname.as_str());
        pathname.0
    }

    #[qjs(get)]
    pub fn port(&self) -> String {
        quirks::port(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "port")]
    pub fn set_port(&mut self, ctx: Ctx<'js>, port: Value<'js>) -> Value<'js> {
        convert_trailing_space(&mut self.url.borrow_mut());

        // TODO: negative ports should be handled in Url
        if port.is_null()
            || port.is_undefined()
            || (port.is_int() && unsafe { port.as_int().unwrap_unchecked() } < 0)
        {
            return port;
        }

        let port_string: Result<Coerced<String>> = Coerced::from_js(&ctx, port.clone());
        if let Ok(port_string) = port_string {
            let _ = quirks::set_port(&mut self.url.borrow_mut(), port_string.as_str());
        }
        port
    }

    #[qjs(get)]
    pub fn protocol(&self) -> String {
        quirks::protocol(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "protocol")]
    pub fn set_protocol(&mut self, protocol: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        let _ = quirks::set_protocol(&mut self.url.borrow_mut(), protocol.as_str());
        protocol.0
    }

    #[qjs(get)]
    pub fn search(&self) -> String {
        quirks::search(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "search")]
    pub fn set_search(&mut self, search: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        quirks::set_search(&mut self.url.borrow_mut(), search.as_str());
        search.0
    }

    #[qjs(get)]
    pub fn search_params(&self) -> &Value<'js> {
        self.search_params.as_value()
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(URL)
    }

    #[qjs(get)]
    pub fn username(&self) -> String {
        quirks::username(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "username")]
    pub fn set_username(&mut self, username: Coerced<String>) -> String {
        convert_trailing_space(&mut self.url.borrow_mut());

        let _ = quirks::set_username(&mut self.url.borrow_mut(), username.as_str());
        username.0
    }

    //
    // Static methods
    //

    #[qjs(static)]
    pub fn can_parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> bool {
        Self::new(ctx, input, base).is_ok()
    }

    #[qjs(static)]
    pub fn parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Result<Value<'js>> {
        Self::new(ctx.clone(), input, base)
            .map_or_else(|_| Null.into_js(&ctx), |instance| instance.into_js(&ctx))
    }

    //
    // Instance methods
    //

    #[qjs(rename = PredefinedAtom::ToJSON)]
    pub fn to_json(&self) -> String {
        // https://developer.mozilla.org/en-US/docs/Web/API/URL/toJSON
        self.to_string()
    }

    pub fn to_string(&self) -> String {
        self.url.borrow().to_string()
    }
}

impl<'js> URL<'js> {
    pub fn from_str(ctx: Ctx<'js>, input: &str) -> Result<Self> {
        let url: Url = input
            .parse()
            .map_err(|_| Exception::throw_type(&ctx, "Invalid URL"))?;
        Self::from_url(ctx, url)
    }

    pub fn from_url(ctx: Ctx<'js>, url: Url) -> Result<Self> {
        let url = Rc::new(RefCell::new(url));
        let search_params = URLSearchParams::from_url(&url);
        let search_params = Class::instance(ctx, search_params)?;

        Ok(Self { url, search_params })
    }
}

pub fn url_to_http_options<'js>(ctx: Ctx<'js>, url: Class<'js, URL<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx)?;

    let url = url.borrow();

    let port = url.port();
    let username = url.username();
    let search = url.search();
    let hash = url.url.borrow().fragment().unwrap_or("").to_string();

    obj.set("protocol", url.protocol())?;
    obj.set("hostname", url.hostname())?;

    if !hash.is_empty() {
        obj.set("hash", hash)?;
    }
    if !search.is_empty() {
        obj.set("search", search)?;
    }

    obj.set("pathname", url.pathname())?;
    obj.set("path", [url.pathname(), url.search()].join(""))?;
    obj.set("href", url.href())?;

    if !username.is_empty() {
        obj.set("auth", [username, ":".to_string(), url.password()].join(""))?;
    }

    if !port.is_empty() {
        obj.set("port", port)?;
    }

    Ok(obj)
}
