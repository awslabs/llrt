// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::inherent_to_string)]
pub mod url_class;
pub mod url_search_params;

use std::{path::PathBuf, str::FromStr};

use llrt_utils::{
    module::{export_default, ModuleInfo},
    primordials::{BasePrimordials, Primordial},
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

/// Returns whether the given scheme is a [special scheme](https://url.spec.whatwg.org/#special-scheme).
pub fn is_special_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ftp" | "ws" | "wss" | "file")
}

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

    let path = url_string.trim_start_matches("file://");

    Ok(PathBuf::from_str(path)
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

/// Encode trailing space as `%20` in opaque paths before a setter runs.
///
/// Used by [`URLSearchParams`] which mutates the shared [`Url`] directly.
pub fn convert_trailing_space(url: &mut Url) {
    if is_special_scheme(url.scheme()) {
        return;
    }

    let path = url.path();
    let has_remaining = url.fragment().is_some() || url.query().is_some();

    #[allow(clippy::manual_strip)]
    if path.ends_with(' ') && has_remaining {
        let new_path = [&path[..path.len() - 1], "%20"].concat();
        url.set_path(&new_path);
    }
}

/// Per WHATWG URL spec §4.5.3 ("URL serializer"), the `/.` path sentinel is
/// only inserted when a URL has no host AND its path starts with `//`. The
/// `url` crate inserts the sentinel during parsing and can leave it in the
/// serialization even after a host is set, breaking WPT `url-setters`
/// subtests like `<non-spec:/.//p>.hostname = 'h'`.
///
/// This strips the sentinel whenever the URL has a non-empty host and the
/// path begins with `/./`.
/// Per WHATWG URL spec §4.2, a file URL path segment matching `[A-Za-z]|`
/// followed by `/`, `\`, `?`, `#`, or end-of-path is a Windows drive letter.
/// Parsers normalize the `|` to `:`. The `url` crate doesn't perform this
/// rewrite itself, so we do it after parsing (WPT `url-constructor.any.js`
/// "Parsing: <file:///w|/m>").
/// When a `file://HOST/C:/...` string is parsed, the `url` crate drops
/// HOST (normalizing to `file:///C:/...`). Per WHATWG URL spec the host
/// must be preserved when non-empty (drive-letter state only applies when
/// host is null). Extract the host from the original source string and
/// re-set it on the parsed URL so downstream `join()` sees the host.
pub fn preserve_file_url_host(source: &str, mut url: Url) -> Url {
    if url.scheme() != "file" {
        return url;
    }
    if url.host_str().is_some_and(|h| !h.is_empty()) {
        return url;
    }
    // Look for `file://HOST/...` in the original string.
    let Some(rest) = source.strip_prefix("file://") else {
        return url;
    };
    let Some((host, _)) = rest.split_once('/') else {
        return url;
    };
    if host.is_empty() {
        return url;
    }
    let _ = url.set_host(Some(host));
    url
}

/// When resolving a relative URL against a file:// base whose first path
/// segment is a Windows drive letter (e.g. `file://h/C:/a/b`), the url crate
/// loses the host during `join`. Per WHATWG URL spec the host must be
/// preserved (WPT `url-constructor.any.js` "<file://h/C:/a/b>" base).
/// Patch the joined URL by restoring the base's host.
pub fn restore_file_url_host(base: &Url, joined: &mut Url) {
    if base.scheme() != "file" || joined.scheme() != "file" {
        return;
    }
    // Only when base had a host and joined has none / empty.
    let Some(base_host) = base.host_str() else {
        return;
    };
    if base_host.is_empty() {
        return;
    }
    if joined.host_str().is_some_and(|h| !h.is_empty()) {
        return;
    }
    // Only when base's first path segment is a Windows drive letter — that's
    // the code path that the url crate mishandles.
    let base_path = base.path();
    let is_drive_letter_first_seg = base_path
        .as_bytes()
        .get(1)
        .is_some_and(|b| b.is_ascii_alphabetic())
        && base_path.as_bytes().get(2) == Some(&b':')
        && matches!(base_path.as_bytes().get(3), Some(&b'/') | None);
    if !is_drive_letter_first_seg {
        return;
    }
    let _ = joined.set_host(Some(base_host));
}

pub fn normalize_windows_drive_letter(url: &mut Url) {
    if url.scheme() != "file" {
        return;
    }
    let path = url.path();
    let bytes = path.as_bytes();
    // Expect path like "/<letter>|/..." — 4+ bytes, leading slash, letter,
    // pipe, trailing slash.
    if bytes.len() < 4
        || bytes[0] != b'/'
        || !bytes[1].is_ascii_alphabetic()
        || bytes[2] != b'|'
        || bytes[3] != b'/'
    {
        return;
    }
    let new_path = ["/", &path[1..2], ":", &path[3..]].concat();
    url.set_path(&new_path);
}

/// Per WHATWG URL spec, a non-special URL with an empty host can have its
/// path erased (WPT `url-setters.any.js`). The `url` crate keeps a trailing
/// `/` after the authority; reparse the serialization with it stripped when
/// the caller has explicitly set an empty pathname on such a URL.
pub fn erase_empty_host_path(url: &mut Url) {
    if is_special_scheme(url.scheme()) {
        return;
    }
    if url.path() != "/" {
        return;
    }
    let serialized = url.as_str();
    // Serialized form must be `<scheme>://` + `/` to qualify. (`scheme:/`,
    // without authority, isn't eligible — the extra `/` is not a sentinel
    // but a real path character.)
    let Some(scheme_end) = serialized.find("://") else {
        return;
    };
    let authority_and_path = &serialized[scheme_end + 3..];
    // After "://": optional userinfo + host + port, then the path. If the
    // path is just "/" and everything before is empty, the full
    // authority_and_path is "/".
    if authority_and_path != "/" {
        return;
    }
    // Strip the trailing `/`.
    let stripped = &serialized[..serialized.len() - 1];
    if let Ok(reparsed) = Url::parse(stripped) {
        *url = reparsed;
    }
}

pub fn strip_path_sentinel(url: &mut Url) {
    if is_special_scheme(url.scheme()) {
        return;
    }
    // Path starting with `//` is what triggers the `/.` sentinel in the url
    // crate's serialization — but the sentinel is only spec-correct when
    // there's no authority. If the URL has `://` and its serialization still
    // contains `/./` at the path boundary, reparse with it stripped.
    if !url.path().starts_with("//") {
        return;
    }
    let serialized = url.as_str();
    // Authority is present iff serialization contains "://".
    let Some(auth_start) = serialized.find("://") else {
        return;
    };
    let after_auth = auth_start + 3;
    // Look for next `/` that starts the path region.
    let Some(path_start_rel) = serialized[after_auth..].find('/') else {
        return;
    };
    let path_idx = after_auth + path_start_rel;
    if serialized[path_idx..].starts_with("/./") {
        let stripped = [&serialized[..path_idx], &serialized[path_idx + 2..]].concat();
        if let Ok(reparsed) = Url::parse(&stripped) {
            *url = reparsed;
        }
    }
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
        BasePrimordials::init(ctx)?;
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
