use hyper::Uri;
use rquickjs::{Ctx, Error, Exception, Result};
use std::sync::OnceLock;

static HTTP_ALLOW_LIST: OnceLock<Vec<Uri>> = OnceLock::new();

static HTTP_DENY_LIST: OnceLock<Vec<Uri>> = OnceLock::new();

pub fn set_allow_list(values: Vec<Uri>) {
    _ = HTTP_ALLOW_LIST.set(values);
}

pub fn get_allow_list() -> Option<&'static Vec<Uri>> {
    HTTP_ALLOW_LIST.get()
}

pub fn set_deny_list(values: Vec<Uri>) {
    _ = HTTP_DENY_LIST.set(values);
}

pub fn get_deny_list() -> Option<&'static Vec<Uri>> {
    HTTP_DENY_LIST.get()
}

pub fn ensure_url_access(ctx: &Ctx<'_>, uri: &Uri) -> Result<()> {
    if let Some(allow_list) = HTTP_ALLOW_LIST.get() {
        if !url_match(allow_list, uri) {
            return Err(url_restricted_error(ctx, "URL not allowed", uri));
        }
    }

    if let Some(deny_list) = HTTP_DENY_LIST.get() {
        if url_match(deny_list, uri) {
            return Err(url_restricted_error(ctx, "URL denied", uri));
        }
    }

    Ok(())
}

fn url_restricted_error(ctx: &Ctx<'_>, message: &str, uri: &Uri) -> Error {
    let uri_host = uri.host().unwrap_or_default();
    let mut message_string = String::with_capacity(message.len() + 100);
    message_string.push_str(message);
    message_string.push_str(": ");
    message_string.push_str(uri_host);
    if let Some(port) = uri.port_u16() {
        message_string.push(':');
        message_string.push_str(itoa::Buffer::new().format(port))
    }

    Exception::throw_message(ctx, &message_string)
}

fn url_match(list: &[Uri], uri: &Uri) -> bool {
    let host = uri.host().unwrap_or_default();
    let port = uri.port_u16().unwrap_or(80);
    list.iter().any(|entry| {
        host.ends_with(entry.host().unwrap_or_default()) && entry.port_u16().unwrap_or(80) == port
    })
}
