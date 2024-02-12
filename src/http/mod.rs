// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod fetch;
mod headers;
mod request;
mod response;
mod url;
mod url_search_params;

use rquickjs::{Class, Ctx, Result};

use crate::http::headers::Headers;

use self::{request::Request, response::Response, url::URL, url_search_params::URLSearchParams};

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    fetch::init(ctx, &globals)?;

    Class::<Request>::define(&globals)?;
    Class::<Response>::define(&globals)?;
    Class::<Headers>::define(&globals)?;
    Class::<URLSearchParams>::define(&globals)?;
    Class::<URL>::define(&globals)?;

    Ok(())
}
