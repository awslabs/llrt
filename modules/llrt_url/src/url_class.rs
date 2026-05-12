// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{cell::RefCell, rc::Rc};

use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, Class, Coerced, Ctx, Exception, FromJs,
    IntoJs, Null, Object, Result, Value,
};
use url::{quirks, Url};

use super::url_search_params::URLSearchParams;

/// Represents a JavaScript
/// [`URL`](https://developer.mozilla.org/en-US/docs/Web/API/URL/URL) as defined
/// by the [WHATWG URL standard](https://url.spec.whatwg.org/).
#[derive(Clone, Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct URL<'js> {
    #[qjs(skip_trace)]
    url: Rc<RefCell<Url>>,
    search_params: Class<'js, URLSearchParams>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> URL<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Result<Self> {
        // USVString conversion per WHATWG URL spec: lone UTF-16 surrogates
        // must be replaced with U+FFFD (not rejected) before the basic URL
        // parser runs (WPT `url-origin.any.js` passes URLs containing lone
        // surrogates and expects them to parse).
        let input: Result<String> = if input.is_string() {
            llrt_utils::bytes::get_lossy_string(input.clone())
        } else {
            Coerced::<String>::from_js(&ctx, input.clone()).map(|c| c.0)
        };
        if let Some(base) = base.into_inner() {
            if let Some(base) = base.as_string() {
                if let Ok(base) = base.to_string() {
                    let base_url: Url = base
                        .parse()
                        .map_err(|_| Exception::throw_type(&ctx, "Invalid base URL"))?;
                    // Work around a url-crate normalization that loses the
                    // host when a file:// URL's path starts with a Windows
                    // drive letter (WPT url-constructor.any.js file-URL-
                    // with-host base cases). Extract the host manually
                    // from the original source string and preserve it.
                    let base_url = super::preserve_file_url_host(&base, base_url);
                    if let Ok(input) = input {
                        let mut joined = base_url
                            .join(input.as_str())
                            .map_err(|_| Exception::throw_type(&ctx, "Invalid URL"))?;
                        super::restore_file_url_host(&base_url, &mut joined);
                        return Self::from_url(ctx, joined);
                    }
                    return Self::from_str(ctx, &base);
                }
            }
        }
        if let Ok(input) = input {
            Self::from_str(ctx, input.as_str())
        } else {
            Err(Exception::throw_message(&ctx, "Invalid URL"))
        }
    }

    #[qjs(get)]
    pub fn hash(&self) -> String {
        quirks::hash(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "hash")]
    pub fn set_hash(&mut self, hash: String) -> String {
        self.before_mutation();
        quirks::set_hash(&mut self.url.borrow_mut(), &hash);
        hash
    }

    #[qjs(get)]
    pub fn host(&self) -> String {
        quirks::host(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "host")]
    pub fn set_host(&mut self, host: Coerced<String>) -> String {
        self.before_mutation();
        let _ = quirks::set_host(&mut self.url.borrow_mut(), &host);
        host.0
    }

    #[qjs(get)]
    pub fn hostname(&self) -> String {
        quirks::hostname(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "hostname")]
    pub fn set_hostname(&mut self, hostname: Coerced<String>) -> String {
        self.before_mutation();
        let _ = quirks::set_hostname(&mut self.url.borrow_mut(), hostname.as_str());
        super::strip_path_sentinel(&mut self.url.borrow_mut());
        hostname.0
    }

    #[qjs(get)]
    pub fn href(&self) -> String {
        quirks::href(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "href")]
    pub fn set_href(&mut self, href: String) -> String {
        self.before_mutation();
        let _ = quirks::set_href(&mut self.url.borrow_mut(), &href);
        href
    }

    #[qjs(get)]
    pub fn origin(&self) -> String {
        let url = self.url.borrow();
        // Per WHATWG URL spec §6.2, origin of a blob URL is computed by parsing
        // the path as a URL. If the result's scheme is HTTP(S), return that
        // URL's origin; otherwise, return an opaque (null) origin. The `url`
        // crate returns the nested URL's origin even for non-HTTP schemes,
        // breaking WPT `url-origin.any.js` on cases like `blob:ftp://...` and
        // `blob:blob:https://...`.
        if url.scheme() == "blob" {
            return match url::Url::parse(url.path()) {
                Ok(inner) if matches!(inner.scheme(), "http" | "https") => quirks::origin(&inner),
                _ => "null".into(),
            };
        }
        quirks::origin(&url)
    }

    #[qjs(get)]
    pub fn password(&self) -> String {
        quirks::password(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "password")]
    pub fn set_password(&mut self, password: Coerced<String>) -> String {
        self.before_mutation();
        let _ = quirks::set_password(&mut self.url.borrow_mut(), &password);
        password.0
    }

    #[qjs(get)]
    pub fn pathname(&self) -> String {
        quirks::pathname(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "pathname")]
    pub fn set_pathname(&mut self, pathname: Coerced<String>) -> String {
        self.before_mutation();
        quirks::set_pathname(&mut self.url.borrow_mut(), pathname.as_str());
        // Per WHATWG URL spec, a non-special URL with an empty host can have
        // its path erased (WPT `url-setters.any.js` "Non-special URLs with
        // an empty host can have their paths erased"). The `url` crate
        // forces a single `/` after the authority; strip it when the caller
        // set an empty pathname on such a URL.
        if pathname.0.is_empty() {
            super::erase_empty_host_path(&mut self.url.borrow_mut());
        }
        pathname.0
    }

    #[qjs(get)]
    pub fn port(&self) -> String {
        quirks::port(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "port")]
    pub fn set_port(&mut self, ctx: Ctx<'js>, port: Value<'js>) -> Value<'js> {
        if port.is_null()
            || port.is_undefined()
            || (port.is_int() && unsafe { port.as_int().unwrap_unchecked() } < 0)
        {
            return port;
        }
        if let Ok(port_string) = Coerced::<String>::from_js(&ctx, port.clone()) {
            self.before_mutation();
            // Per WHATWG URL spec, the port-state parser strips tab/LF/CR
            // before reading. An empty STRIPPED value (but non-empty original)
            // makes port parsing fail, which per spec means no-op (keep
            // existing port). An empty ORIGINAL value, however, clears the
            // port.
            if port_string.is_empty() {
                let _ = quirks::set_port(&mut self.url.borrow_mut(), "");
            } else {
                let stripped: String = port_string
                    .chars()
                    .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
                    .collect();
                if !stripped.is_empty() {
                    let _ = quirks::set_port(&mut self.url.borrow_mut(), &stripped);
                }
                // stripped is empty → parse failure per spec → no-op
            }
        }
        port
    }

    #[qjs(get)]
    pub fn protocol(&self) -> String {
        quirks::protocol(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "protocol")]
    pub fn set_protocol(&mut self, protocol: Coerced<String>) -> String {
        self.before_mutation();
        let _ = quirks::set_protocol(&mut self.url.borrow_mut(), &protocol);
        protocol.0
    }

    #[qjs(get)]
    pub fn search(&self) -> String {
        quirks::search(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "search")]
    pub fn set_search(&mut self, search: Coerced<String>) -> String {
        self.before_mutation();
        quirks::set_search(&mut self.url.borrow_mut(), &search);
        search.0
    }

    #[qjs(get)]
    pub fn search_params(&self) -> &Value<'js> {
        self.search_params.as_value()
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(URL)
    }

    #[qjs(get)]
    pub fn username(&self) -> String {
        quirks::username(&self.url.borrow()).to_string()
    }

    #[qjs(set, rename = "username")]
    pub fn set_username(&mut self, username: Coerced<String>) -> String {
        self.before_mutation();
        let _ = quirks::set_username(&mut self.url.borrow_mut(), &username);
        username.0
    }

    #[qjs(static)]
    pub fn can_parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> bool {
        Self::new(ctx, input, base).is_ok()
    }

    #[qjs(static)]
    pub fn parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Result<Value<'js>> {
        Self::new(ctx.clone(), input, base)
            .map_or_else(|_| Null.into_js(&ctx), |instance| instance.into_js(&ctx))
    }

    #[qjs(rename = PredefinedAtom::ToJSON)]
    pub fn to_json(&self) -> String {
        self.to_string()
    }

    pub fn to_string(&self) -> String {
        self.href()
    }
}

impl<'js> URL<'js> {
    pub fn from_str(ctx: Ctx<'js>, input: &str) -> Result<Self> {
        let mut url: Url = input
            .parse()
            .map_err(|_| Exception::throw_type(&ctx, "Invalid URL"))?;
        super::normalize_windows_drive_letter(&mut url);
        super::convert_trailing_space(&mut url);
        Self::build(ctx, url)
    }

    pub fn from_url(ctx: Ctx<'js>, mut url: Url) -> Result<Self> {
        super::normalize_windows_drive_letter(&mut url);
        super::convert_trailing_space(&mut url);
        Self::build(ctx, url)
    }

    /// Validate that a string parses as a URL without constructing a JS
    /// instance. Used by callers (e.g. `llrt_fetch`) that just need to know
    /// whether a user-supplied string is a valid URL.
    pub fn is_valid(input: &str) -> bool {
        input.parse::<Url>().is_ok()
    }

    fn build(ctx: Ctx<'js>, url: Url) -> Result<Self> {
        let shared = Rc::new(RefCell::new(url));
        let search_params = Class::instance(ctx, URLSearchParams::from_url(&shared))?;
        Ok(Self {
            url: shared,
            search_params,
        })
    }

    fn before_mutation(&mut self) {
        super::convert_trailing_space(&mut self.url.borrow_mut());
    }

    pub(crate) fn inner_url(&self) -> std::cell::Ref<'_, Url> {
        self.url.borrow()
    }
}

pub fn url_to_http_options<'js>(ctx: Ctx<'js>, url: Class<'js, URL<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx)?;
    let url = url.borrow();

    let port = url.port();
    let username = url.username();
    let search = url.search();
    let hash = url.inner_url().fragment().unwrap_or("").to_string();

    obj.set("protocol", url.protocol())?;
    obj.set("hostname", url.hostname())?;

    if !hash.is_empty() {
        obj.set("hash", hash)?;
    }

    let pathname = url.pathname();
    let path = [pathname.as_str(), search.as_str()].concat();
    if !search.is_empty() {
        obj.set("search", search)?;
    }
    obj.set("pathname", pathname)?;
    obj.set("path", path)?;
    obj.set("href", url.href())?;

    if !username.is_empty() {
        obj.set("auth", [username, url.password()].join(":"))?;
    }

    if !port.is_empty() {
        obj.set("port", port)?;
    }

    Ok(obj)
}
