// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, RwLock};

use llrt_events::{Emitter, EventList, Events};
use llrt_utils::time;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    Ctx, JsLifetime, Object, Result,
};

#[rquickjs::class]
#[derive(Clone)]
pub struct Performance<'js> {
    pub events: Events<'js>,
}

unsafe impl<'js> JsLifetime<'js> for Performance<'js> {
    type Changed<'to> = Performance<'to>;
}

impl<'js> Emitter<'js> for Performance<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.events.clone()
    }
}

impl<'js> Trace<'js> for Performance<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.trace_event_emitter(tracer);
    }
}

impl<'js> Default for Performance<'js> {
    fn default() -> Self {
        Self::new()
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Performance<'js> {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            #[allow(clippy::arc_with_non_send_sync)]
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[qjs(get)]
    fn time_origin() -> f64 {
        let time_origin = time::origin_nanos() as f64;

        time_origin / 1e6
    }

    fn now() -> f64 {
        let now = time::now_nanos();
        let started = time::origin_nanos();
        let elapsed = now.checked_sub(started).unwrap_or_default();

        (elapsed as f64) / 1e6
    }

    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json(ctx: Ctx<'_>) -> Result<Object<'_>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("timeOrigin", Self::time_origin())?;

        Ok(obj)
    }
}
