// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, HashMap};

use rquickjs::{
    atom::PredefinedAtom, Array, ArrayBuffer, Coerced, Ctx, Exception, FromJs, Function, IntoAtom,
    IntoJs, Object, Result, Symbol, TypedArray, Value,
};

use super::result::ResultExt;

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

pub fn map_to_entries<'js, K, V, M>(ctx: &Ctx<'js>, map: M) -> Result<Array<'js>>
where
    M: IntoIterator<Item = (K, V)>,
    K: IntoJs<'js>,
    V: IntoJs<'js>,
{
    let array = Array::new(ctx.clone())?;
    for (idx, (key, value)) in map.into_iter().enumerate() {
        let entry = Array::new(ctx.clone())?;
        entry.set(0, key)?;
        entry.set(1, value)?;
        array.set(idx, entry)?;
    }

    Ok(array)
}

pub fn get_checked_len(source_len: usize, target_len: Option<usize>, offset: usize) -> usize {
    let target_len = target_len.unwrap_or(source_len);

    if offset >= target_len {
        return 0;
    }
    if (offset + target_len) > source_len {
        return source_len;
    }
    target_len
}

pub fn get_bytes_offset_length<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    offset: Option<usize>,
    length: Option<usize>,
) -> Result<Vec<u8>> {
    let offset = offset.unwrap_or(0);

    if let Some(val) = value.as_string() {
        let string = val.to_string()?;
        return Ok(bytes_from_js_string(string, offset, length));
    }
    if value.is_array() {
        let array = value.as_array().unwrap();
        let checked_length = get_checked_len(array.len(), length, offset);
        let mut bytes: Vec<u8> = Vec::with_capacity(checked_length);

        for val in array.iter::<u8>().skip(offset).take(checked_length) {
            let val: u8 = val.or_throw_msg(ctx, "array value is not u8")?;
            bytes.push(val);
        }

        return Ok(bytes);
    }

    if let Some(obj) = value.as_object() {
        if let Some((array_buffer, max_length)) = obj_to_array_buffer(obj)? {
            return get_array_buffer_bytes(array_buffer.as_ref(), offset, length, max_length)
                .map(|v| v.to_vec());
        }
    }

    if let Ok(val) = value.get::<Coerced<String>>() {
        let string = val.to_string();
        return Ok(bytes_from_js_string(string, offset, length));
    };

    Err(Exception::throw_message(
        ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or interpretable as string",
    ))
}

fn bytes_from_js_string(string: String, offset: usize, length: Option<usize>) -> Vec<u8> {
    let checked_length = get_checked_len(string.len(), length, offset);
    string.as_bytes()[offset..offset + checked_length].to_vec()
}

pub fn obj_to_array_buffer<'js>(obj: &Object<'js>) -> Result<Option<(ArrayBuffer<'js>, usize)>> {
    //most common
    if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
        let byte_length = typed_array.len();
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }
    //second most common
    if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
        let byte_length = array_buffer.len();
        return Ok(Some((array_buffer, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<i8>::from_object(obj.clone()) {
        let byte_length = typed_array.len();
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<u16>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 2;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<i16>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 2;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<u32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<i32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<u64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<i64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<f32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(typed_array) = TypedArray::<f64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        return Ok(Some((typed_array.arraybuffer()?, byte_length)));
    }

    if let Ok(array_buffer) = obj.get::<_, ArrayBuffer>("buffer") {
        let length = array_buffer.len();
        return Ok(Some((array_buffer, length)));
    }

    Ok(None)
}

fn get_array_buffer_bytes(
    bytes: &[u8],
    offset: usize,
    desired_length: Option<usize>,
    source_length: usize,
) -> Result<&[u8]> {
    let checked_length = get_checked_len(source_length, desired_length, offset);
    Ok(&bytes[offset..offset + checked_length])
}

pub fn get_bytes<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Vec<u8>> {
    get_bytes_offset_length(ctx, value, None, None)
}

pub fn bytes_to_typed_array<'js>(ctx: Ctx<'js>, bytes: &[u8]) -> Result<Value<'js>> {
    TypedArray::<u8>::new(ctx.clone(), bytes).into_js(&ctx)
}

pub trait ObjectExt<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>>;
}

impl<'js> ObjectExt<'js> for Object<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        self.get::<K, Option<V>>(k)
    }
}

impl<'js> ObjectExt<'js> for Value<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_optional(k);
        }
        Ok(None)
    }
}

pub trait CreateSymbol<'js> {
    fn for_description(globals: &Object<'js>, description: &'static str) -> Result<Symbol<'js>>;
}

impl<'js> CreateSymbol<'js> for Symbol<'js> {
    fn for_description(globals: &Object<'js>, description: &'static str) -> Result<Symbol<'js>> {
        let symbol_function: Function = globals.get(PredefinedAtom::Symbol)?;
        let for_function: Function = symbol_function.get(PredefinedAtom::For)?;
        for_function.call((description,))
    }
}
