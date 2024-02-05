use std::collections::{BTreeMap, HashMap};

use rquickjs::{
    atom::PredefinedAtom, function::Constructor, Array, ArrayBuffer, Ctx, Exception, FromJs,
    Function, IntoAtom, IntoJs, Object, Result, TypedArray, Value,
};

use super::{class::get_class_name, result::ResultExt};

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
) -> Result<BTreeMap<String, String>> {
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
        let checked_length = get_checked_len(string.len(), length, offset);
        return Ok(string.as_bytes()[offset..offset + checked_length].to_vec());
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
        if let Some(array_buffer) = obj_to_array_buffer(ctx, obj)? {
            return get_array_buffer_bytes(array_buffer, offset, length);
        }
    }

    Err(Exception::throw_message(
        ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or string",
    ))
}

pub fn obj_to_array_buffer<'js>(
    ctx: &Ctx<'js>,
    obj: &Object<'js>,
) -> Result<Option<ArrayBuffer<'js>>> {
    //most common
    if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    //second most common
    if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
        return Ok(Some(array_buffer));
    }

    let globals = ctx.globals();
    let data_view: Constructor = globals.get(PredefinedAtom::ArrayBuffer)?;
    let is_data_view: Function = data_view.get("isView")?;

    if is_data_view.call::<_, bool>((obj.clone(),))? {
        let class_name = get_class_name(obj)?.unwrap();

        let array_buffer = match class_name.as_str() {
            "Int8Array" => TypedArray::<i8>::from_object(obj.clone())?.arraybuffer(),
            "Uint16Array" => TypedArray::<u16>::from_object(obj.clone())?.arraybuffer(),
            "Int16Array" => TypedArray::<i16>::from_object(obj.clone())?.arraybuffer(),
            "Uint32Array" => TypedArray::<u32>::from_object(obj.clone())?.arraybuffer(),
            "Int32Array" => TypedArray::<i32>::from_object(obj.clone())?.arraybuffer(),
            "Uint64Array" => TypedArray::<u64>::from_object(obj.clone())?.arraybuffer(),
            "Int64Array" => TypedArray::<i64>::from_object(obj.clone())?.arraybuffer(),
            "Float32Array" => TypedArray::<f32>::from_object(obj.clone())?.arraybuffer(),
            "Float64Array" => TypedArray::<f64>::from_object(obj.clone())?.arraybuffer(),
            _ => {
                let array_buffer: ArrayBuffer = obj.get("buffer")?;
                return Ok(Some(array_buffer));
            }
        }?;
        return Ok(Some(array_buffer));
    }

    Ok(None)
}

fn get_array_buffer_bytes(
    array_buffer: ArrayBuffer<'_>,
    offset: usize,
    length: Option<usize>,
) -> Result<Vec<u8>> {
    let bytes: &[u8] = array_buffer.as_ref();
    let checked_length = get_checked_len(bytes.len(), length, offset);
    Ok(bytes[offset..offset + checked_length].to_vec())
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
