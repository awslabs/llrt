// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, HashMap};

use rquickjs::{Array, Coerced, Ctx, FromJs, Object, Result, Value};

#[allow(dead_code)]
pub fn array_to_hash_map<'js>(
    ctx: &Ctx<'js>,
    array: Array<'js>,
) -> Result<HashMap<String, String>> {
    let value = object_from_entries(ctx, array)?;
    let value = value.into_value();
    HashMap::from_js(ctx, value)
}

pub fn array_to_btree_map<'js>(
    ctx: &Ctx<'js>,
    array: Array<'js>,
) -> Result<BTreeMap<String, Coerced<String>>> {
    let value = object_from_entries(ctx, array)?;
    let value = value.into_value();
    BTreeMap::from_js(ctx, value)
}

pub fn object_from_entries<'js>(ctx: &Ctx<'js>, array: Array<'js>) -> Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    for value in array.into_iter().flatten() {
        if let Some(entry) = value.as_array() {
            if let Ok(key) = entry.get::<Value>(0) {
                if let Ok(value) = entry.get::<Value>(1) {
                    let _ = obj.set(key, value); //ignore result of failed
                }
            }
        }
    }
    Ok(obj)
}
