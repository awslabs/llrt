// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{prelude::Opt, Result, Value};

use llrt_utils::object::ObjectExt;

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct Event {
    event_type: String,
    bubbles: bool,
    cancelable: bool,
    composed: bool,
}

#[rquickjs::methods]
impl Event {
    #[qjs(constructor)]
    pub fn new(event_type: String, options: Opt<Value<'_>>) -> Result<Self> {
        let mut bubbles = false;
        let mut cancelable = false;
        let mut composed = false;
        if let Some(options) = options.0 {
            if let Some(opt) = options.get_optional("bubbles")? {
                bubbles = opt;
            }
            if let Some(opt) = options.get_optional("cancelable")? {
                cancelable = opt;
            }
            if let Some(opt) = options.get_optional("composed")? {
                composed = opt;
            }
        }
        Ok(Self {
            event_type,
            bubbles,
            cancelable,
            composed,
        })
    }

    #[qjs(get)]
    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    #[qjs(get)]
    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    #[qjs(get)]
    pub fn composed(&self) -> bool {
        self.composed
    }

    #[qjs(get, rename = "type")]
    pub fn event_type(&self) -> String {
        self.event_type.clone()
    }
}
