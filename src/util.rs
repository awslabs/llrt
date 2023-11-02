use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    result::Result as StdResult,
};

use tokio::fs::{self, DirEntry};

trait GenericMapEach<K, V> {
    fn for_each<F>(&self, cb: F)
    where
        F: Fn((&K, &V));
}

impl<K, V> GenericMapEach<K, V> for HashMap<K, V> {
    fn for_each<F>(&self, cb: F)
    where
        F: Fn((&K, &V)),
    {
        self.iter().for_each(cb)
    }
}

impl<K, V> GenericMapEach<K, V> for BTreeMap<K, V> {
    fn for_each<F>(&self, cb: F)
    where
        F: Fn((&K, &V)),
    {
        self.iter().for_each(cb)
    }
}

pub trait IteratorDef<'js>
where
    Self: 'js + JsClass<'js> + Sized,
{
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>>;

    fn js_iterator(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let value = self.js_entries(ctx)?;
        let obj = value.as_object();
        let values_fn: Function = obj.get(PredefinedAtom::Values)?;
        values_fn.call((This(value),))

        // res.set(
        //     PredefinedAtom::Next,
        //     Func::from(move |ctx: Ctx<'js>| -> Result<Object<'js>> {
        //         let res = Object::new(ctx)?;
        //         match &iter.next() {
        //             Some(value) => {
        //                 res.set(PredefinedAtom::Value, value.clone())?;
        //             }
        //             None => {
        //                 res.set(PredefinedAtom::Done, true)?;
        //             }
        //         }

        //         Ok(res)
        //     }),
        // )?;
        // Ok(res)
    }
}

use rquickjs::{
    atom::PredefinedAtom,
    class::JsClass,
    cstr,
    module::{Declarations, Exports, ModuleDef},
    prelude::This,
    Array, ArrayBuffer, Ctx, Exception, FromJs, Function, IntoAtom, IntoJs, Object, Result,
    String as JsString, TypedArray, Value,
};

pub fn get_class_name(value: &Value) -> Result<Option<String>> {
    value
        .get_optional::<_, Object>(PredefinedAtom::Constructor)?
        .and_then_ok(|ctor| {
            ctor.get_optional::<_, JsString>(PredefinedAtom::Name)
                .map(|name| name.map(|name| name.to_owned().to_string().unwrap()))
        })
}

#[allow(dead_code)]
pub fn instance_of(value: &Value, class_name: &str) -> Result<bool> {
    get_class_name(value).map(|name| name == Some(class_name.to_string()))
}

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

    if let Some(obj) = value.into_object() {
        if let Some(array_buffer) = obj_to_array_buffer(obj)? {
            return get_array_buffer_bytes(array_buffer, offset, length);
        }
    }

    Err(Exception::throw_message(
        ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or string",
    ))
}

pub fn obj_to_array_buffer(obj: Object<'_>) -> Result<Option<ArrayBuffer<'_>>> {
    if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
        return Ok(Some(array_buffer));
    }

    if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<i8>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<u16>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<i16>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<u32>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<i32>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<u64>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<i64>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<f32>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Ok(typed_array) = TypedArray::<f64>::from_object(obj.clone()) {
        return Ok(Some(typed_array.arraybuffer()?));
    }

    if let Some(class_name) = get_class_name(obj.as_value())? {
        if class_name == "DataView" {
            let array_buffer: ArrayBuffer = obj.get("buffer")?;
            return Ok(Some(array_buffer));
        }
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

pub fn get_basename_ext_name(path: &str) -> (String, String) {
    let path = path.strip_prefix("./").unwrap_or(path);
    let (basename, ext) = path.split_at(path.rfind('.').unwrap_or(path.len()));
    (basename.to_string(), ext.to_string())
}

pub static JS_EXTENSIONS: &[&str] = &[".js", ".mjs", ".cjs"];

pub fn get_js_path(path: &str) -> Option<PathBuf> {
    let (mut basename, ext) = get_basename_ext_name(path);

    let filepath = Path::new(path);
    let ext = ext;

    let exists = filepath.exists();

    if !ext.is_empty() && exists {
        return Some(filepath.to_owned());
    }

    if filepath.is_dir() && exists {
        basename = format!("{}/index", &basename);
    }

    for ext in JS_EXTENSIONS {
        let path = &format!("{}{}", &basename, ext);

        let path = Path::new(path);
        if path.exists() {
            return Some(path.to_owned());
        }
    }

    None
}

pub async fn walk_directory<F>(path: PathBuf, mut f: F) -> StdResult<(), std::io::Error>
where
    F: FnMut(&DirEntry) -> bool,
{
    let mut stack = vec![path];
    while let Some(dir) = stack.pop() {
        let mut stream = fs::read_dir(dir).await?;
        while let Some(entry) = stream.next_entry().await? {
            let entry_path = entry.path();

            if f(&entry) && entry_path.is_dir() {
                stack.push(entry_path);
            }
        }
    }
    Ok(())
}

pub fn export_default<'js, F>(ctx: &Ctx<'js>, exports: &mut Exports<'js>, f: F) -> Result<()>
where
    F: FnOnce(&Object<'js>) -> Result<()>,
{
    let default = Object::new(ctx.clone())?;
    f(&default)?;

    for name in default.keys::<String>() {
        let name = name?;
        let value: Value = default.get(&name)?;
        exports.export(name, value)?;
    }

    exports.export("default", default)?;

    Ok(())
}

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare_static(cstr!("default"))?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;

            Ok(())
        })
    }
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

pub trait ResultExt<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T>;
    fn or_throw(self, ctx: &Ctx) -> Result<T>;
}

pub trait CatchPanic<T> {
    fn unwrap_or_catch_panic(self, ctx: Ctx) -> T;
}

pub trait OptionExt<T> {
    fn and_then_ok<U, E, F>(self, f: F) -> StdResult<Option<U>, E>
    where
        F: FnOnce(T) -> StdResult<Option<U>, E>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for StdResult<T, E> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.map_err(|e| Exception::throw_message(ctx, &format!("{}. {}", msg, &e.to_string())))
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.map_err(|err| Exception::throw_message(ctx, &err.to_string()))
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn or_throw_msg(self, ctx: &Ctx, msg: &str) -> Result<T> {
        self.ok_or(Exception::throw_message(ctx, msg))
    }

    fn or_throw(self, ctx: &Ctx) -> Result<T> {
        self.ok_or(Exception::throw_message(ctx, "Value is not present"))
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn and_then_ok<U, E, F>(self, f: F) -> StdResult<Option<U>, E>
    where
        F: FnOnce(T) -> StdResult<Option<U>, E>,
    {
        match self {
            Some(v) => f(v),
            None => Ok(None),
        }
    }
}

pub trait JoinToString<T> {
    fn join_to_string<F>(&mut self, separator: &str, f: F) -> String
    where
        F: FnMut(&T) -> &str;
}

impl<T, I> JoinToString<T> for I
where
    I: Iterator<Item = T>,
{
    fn join_to_string<F>(&mut self, separator: &str, mut f: F) -> String
    where
        F: FnMut(&T) -> &str,
    {
        let mut result = String::new();

        if let Some(first_item) = self.next() {
            result.push_str(f(&first_item));

            for item in self {
                result.push_str(separator);
                result.push_str(f(&item));
            }
        }

        result
    }
}
