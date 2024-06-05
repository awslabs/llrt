use std::{path::PathBuf, str::FromStr};

// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Opt,
    prelude::This,
    Class, Coerced, Ctx, Exception, FromJs, Function, IntoJs, Null, Object, Result, Value,
};
use url::Url;

use crate::utils::result::ResultExt;

use super::url_search_params::URLSearchParams;

static DEFAULT_PORTS: &[&str] = &["21", "80", "443"];
static DEFAULT_PROTOCOLS: &[&str] = &["ftp", "http", "https"];

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
    username: String,
    password: String,
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

    #[qjs(static)]
    pub fn can_parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> bool {
        !Self::parse(ctx.clone(), input.clone(), Opt(base.clone())).is_null()
    }

    #[qjs(static)]
    pub fn parse(ctx: Ctx<'js>, input: Value<'js>, base: Opt<Value<'js>>) -> Value<'js> {
        if let Some(base) = base.0 {
            let base_string = match get_string(&ctx.clone(), base) {
                Ok(s) => s,
                Err(_) => return Null.into_js(&ctx).unwrap(),
            };
            let path_string = match get_string(&ctx.clone(), input) {
                Ok(s) => s,
                Err(_) => return Null.into_js(&ctx).unwrap(),
            };

            match base_string.parse::<Url>() {
                Ok(base_url) => {
                    if let Ok(parsed_url) = base_url.join(&path_string) {
                        return URL::create(ctx.clone(), parsed_url)
                            .unwrap()
                            .into_js(&ctx.clone())
                            .unwrap();
                    }
                },
                Err(_) => return Null.into_js(&ctx).unwrap(),
            }
        } else if input.is_string() {
            if let Ok(string_val) = input.get::<String>() {
                if let Ok(parsed_url) = Url::parse(&string_val) {
                    return URL::create(ctx.clone(), parsed_url)
                        .unwrap()
                        .into_js(&ctx.clone())
                        .unwrap();
                }
            }
        }
        Null.into_js(&ctx).unwrap()
    }

    pub fn to_string(&self) -> String {
        self.format(true, true, true, false)
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
        self.username = new.username;
        self.password = new.password;
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
    fn set_protocol(&mut self, mut protocol: String) -> String {
        if protocol.ends_with(':') {
            protocol.pop();
        }
        self.protocol.clone_from(&protocol);
        self.update_port_host();

        protocol
    }

    #[qjs(get)]
    fn port(&self) -> String {
        self.port.clone()
    }

    #[qjs(set, rename = "port")]
    fn set_port(&mut self, port: Coerced<String>) -> String {
        let port_string = port.to_string();
        self.port.clone_from(&port_string);
        self.update_port_host();

        port_string
    }

    #[qjs(get)]
    fn hostname(&self) -> String {
        self.hostname.clone()
    }

    #[qjs(set, rename = "hostname")]
    fn set_hostname(&mut self, hostname: String) -> String {
        self.hostname.clone_from(&hostname);
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
        self.update_port_host();

        Ok(self.host.clone())
    }

    #[qjs(get)]
    fn pathname(&self) -> String {
        self.pathname.clone()
    }

    #[qjs(set, rename = "pathname")]
    fn set_pathname(&mut self, pathname: String) -> String {
        self.pathname.clone_from(&pathname);
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
    #[qjs(get)]
    fn username(&self) -> String {
        self.username.clone()
    }

    #[qjs(set, rename = "username")]
    fn set_username(&mut self, username: String) -> String {
        self.username.clone_from(&username);
        username
    }

    #[qjs(get)]
    fn password(&self) -> String {
        self.password.clone()
    }

    #[qjs(set, rename = "password")]
    fn set_password(&mut self, password: String) -> String {
        self.password.clone_from(&password);
        password
    }

    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json(&self) -> String {
        // https://developer.mozilla.org/en-US/docs/Web/API/URL/toJSON
        self.to_string()
    }
}

impl<'js> URL<'js> {
    fn create(ctx: Ctx<'js>, url: Url) -> Result<Self> {
        let query = url.query().unwrap_or_default();
        let search_params = URLSearchParams::from_str(query);

        let hostname = url.host().map(|h| h.to_string()).unwrap_or_default();
        let protocol = url.scheme().to_string();

        let port = filtered_port(
            &protocol,
            &url.port().map(|p| p.to_string()).unwrap_or_default(),
        );

        let host = format!(
            "{}{}",
            &hostname,
            &port.clone().map(|p| format!(":{}", &p)).unwrap_or_default()
        );

        let username = url.username().to_string();
        let password = url.password().unwrap_or_default().to_string();

        let search_params = Class::instance(ctx, search_params)?;

        Ok(Self {
            protocol,
            host,
            hostname,
            port: port.unwrap_or_default(),
            pathname: url.path().to_string(),
            hash: url.fragment().map(|f| f.to_string()).unwrap_or_default(),
            search_params,
            username,
            password,
        })
    }

    pub fn from_str(ctx: Ctx<'js>, input: &str) -> Result<Self> {
        let url: Url = input.parse().or_throw_msg(&ctx, "Invalid URL")?;
        Self::create(ctx, url)
    }

    fn update_port_host(&mut self) {
        if let Some(p) = filtered_port(&self.protocol, &self.port) {
            self.host = format!("{}:{}", self.hostname, self.port);
            self.port = p;
        } else {
            self.port.clear();
            self.host.clone_from(&self.hostname);
        }
    }

    fn format(
        &self,
        include_auth: bool,
        include_fragment: bool,
        include_search: bool,
        unicode_encode: bool,
    ) -> String {
        let search = if include_search {
            search_params_to_string(&self.search_params)
        } else {
            String::from("")
        };
        let hash = &self.hash;
        let hash = if include_fragment && !hash.is_empty() {
            format!("#{}", &hash)
        } else {
            String::from("")
        };

        let mut user_info = String::new();
        if include_auth && !self.username.is_empty() {
            user_info.push_str(&self.username);
            if !self.password.is_empty() {
                user_info.push(':');
                user_info.push_str(&self.password)
            }
            user_info.push('@')
        }

        let host = if unicode_encode {
            domain_to_unicode(&self.host)
        } else {
            self.host.clone()
        };

        format!(
            "{}://{}{}{}{}{}",
            &self.protocol, user_info, host, &self.pathname, &search, &hash
        )
    }
}

fn filtered_port(protocol: &str, port: &str) -> Option<String> {
    if let Some(pos) = DEFAULT_PROTOCOLS.iter().position(|&p| p == protocol) {
        if DEFAULT_PORTS[pos] == port {
            return None;
        }
    }
    if port.is_empty() {
        return None;
    }
    Some(port.to_string())
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

pub fn url_to_http_options<'js>(ctx: Ctx<'js>, url: Class<'js, URL<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx)?;

    let url = url.borrow();

    let port = url.port();
    let username = url.username();
    let search = url.search();
    let hash = url.hash();

    obj.set("protocol", url.protocol())?;
    obj.set("hostname", url.hostname())?;

    if !hash.is_empty() {
        obj.set("hash", url.hash())?;
    }
    if !search.is_empty() {
        obj.set("search", url.search())?;
    }

    obj.set("pathname", url.pathname())?;
    obj.set("path", format!("{}{}", url.pathname(), url.search()))?;
    obj.set("href", url.href())?;

    if !username.is_empty() {
        obj.set("auth", format!("{}:{}", username, url.password()))?;
    }

    if !port.is_empty() {
        obj.set("port", url.port())?;
    }

    Ok(obj)
}

pub fn domain_to_unicode(domain: &str) -> String {
    let (url, result) = idna::domain_to_unicode(domain);
    if result.is_err() {
        return String::from("");
    }
    url
}

pub fn domain_to_ascii(domain: &str) -> String {
    idna::domain_to_ascii(domain).unwrap_or_default()
}

//options are ignored, no windows support yet
pub fn path_to_file_url<'js>(ctx: Ctx<'js>, path: String, _: Opt<Value>) -> Result<URL<'js>> {
    let url = Url::from_file_path(path).unwrap();
    URL::create(ctx, url)
}

//options are ignored, no windows support yet
pub fn file_url_to_path<'js>(ctx: Ctx<'js>, url: Value<'js>) -> Result<String> {
    let url_string = if let Ok(url) = Class::<URL>::from_value(url.clone()) {
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
    let mut fragment = true;
    let mut unicode = false;
    let mut auth = true;
    let mut search = true;

    // Parse options if provided
    if let Some(options) = options.0 {
        if options.is_object() {
            let options = options.as_object().unwrap();
            if let Some(value) = options.get("fragment")? {
                fragment = value;
            }
            if let Ok(value) = options.get("unicode") {
                unicode = value;
            }
            if let Ok(value) = options.get("auth") {
                auth = value;
            }
            if let Ok(value) = options.get("search") {
                search = value
            }
        }
    }

    Ok(url.borrow().format(auth, fragment, search, unicode))
}
