use core::num;
use std::collections::HashSet;
#[cfg(feature = "nightly")]
use std::simd::{u8x16, Mask, Simd, SimdPartialEq, SimdPartialOrd, ToBitMask};

use std::time::Instant;

use rayon::iter::ParallelIterator;
use rquickjs::convert::Coerced;
use rquickjs::{
    atom::PredefinedAtom, function::This, Array, Ctx, Function, IntoJs, Null, Object, Result, Value,
};
use rquickjs::{Exception, Type, Undefined};
use simd_json::borrowed::Value as JsonValue;
use simd_json::{Node, StaticNode};

use std::fmt::Write;

static JSON_ESCAPE_CHARS: [u8; 256] = [
    0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
    17u8, 18u8, 19u8, 20u8, 21u8, 22u8, 23u8, 24u8, 25u8, 26u8, 27u8, 28u8, 29u8, 30u8, 31u8, 34u8,
    34u8, 32u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 33u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
];
static JSON_ESCAPE_QUOTES: [&str; 34usize] = [
    "\\u0000", "\\u0001", "\\u0002", "\\u0003", "\\u0004", "\\u0005", "\\u0006", "\\u0007", "\\b",
    "\\t", "\\n", "\\u000b", "\\f", "\\r", "\\u000e", "\\u000f", "\\u0010", "\\u0011", "\\u0012",
    "\\u0013", "\\u0014", "\\u0015", "\\u0016", "\\u0017", "\\u0018", "\\u0019", "\\u001a",
    "\\u001b", "\\u001c", "\\u001d", "\\u001e", "\\u001f", "\\\"", "\\\\",
];

const ESCAPE_LEN: usize = 34;

pub fn escape_json(bytes: &[u8]) -> String {
    let mut output = String::new();
    escape_json_string(&mut output, bytes);
    output
}

#[cfg(not(feature = "nightly"))]
pub fn escape_json_string(output: &mut String, bytes: &[u8]) {
    let len = bytes.len();
    let mut start = 0;
    output.reserve(len);

    for (i, byte) in bytes.iter().enumerate() {
        let c = JSON_ESCAPE_CHARS[*byte as usize] as usize;
        if c < ESCAPE_LEN {
            if start < i {
                output.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
            }
            output.push_str(JSON_ESCAPE_QUOTES[c]);
            start = i + 1;
        }
    }
    if start < len {
        output.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..len]) });
    }
}

#[cfg(feature = "nightly")]
pub fn escape_json_string(output: &mut String, bytes: &[u8]) {
    const ESCAPE_LEN: usize = 34;
    const BELOW_SPACE: u8 = b' ' - 1;
    const B: u8 = b'"';
    const C: u8 = b'\\';

    let v_below_space: u8x16 = u8x16::splat(BELOW_SPACE);
    let v_b: u8x16 = u8x16::splat(B);
    let v_c: u8x16 = u8x16::splat(C);

    let len = bytes.len();
    output.reserve(len);

    #[inline(always)]
    fn process_padded_chunk(
        bytes: &[u8],
        result: &mut String,
        v_below_space: u8x16,
        v_b: u8x16,
        v_c: u8x16,
    ) {
        let len = bytes.len();
        if len > 0 {
            let mut padded_bytes = [b'_'; 16]; //can be max 16 *2 offset
            padded_bytes[..len].copy_from_slice(bytes);
            let byte_vector = u8x16::from_slice(&padded_bytes);
            process_chunk(
                &padded_bytes,
                result,
                byte_vector,
                len,
                v_below_space,
                v_b,
                v_c,
            );
        }
    }

    #[inline(always)]
    fn process_chunk(
        chunk: &[u8],
        output: &mut String,
        byte_vector: Simd<u8, 16>,
        max_len: usize,
        v_below_space: u8x16,
        v_b: u8x16,
        v_c: u8x16,
    ) {
        let mut mask = (byte_vector.simd_eq(v_b)
            | byte_vector.simd_eq(v_c)
            | (byte_vector).simd_lt(v_below_space))
        .to_bitmask();

        if mask != 0 {
            let mut cur = mask.trailing_zeros() as usize;
            let mut start = 0;

            while cur < max_len {
                let c = JSON_ESCAPE_CHARS[chunk[cur] as usize] as usize;
                if c < ESCAPE_LEN {
                    if start < cur {
                        output
                            .push_str(unsafe { std::str::from_utf8_unchecked(&chunk[start..cur]) });
                    }
                    output.push_str(JSON_ESCAPE_QUOTES[c]);
                    start = cur + 1;
                }
                mask ^= 1 << cur;
                if mask == 0 {
                    break;
                }
                cur = mask.trailing_zeros() as usize;
            }
        } else {
            output.push_str(unsafe { std::str::from_utf8_unchecked(&chunk[..max_len]) });
        }
    }

    fn process(bytes: &[u8], output: &mut String, v_below_space: u8x16, v_b: u8x16, v_c: u8x16) {
        let iter = bytes.chunks_exact(16);

        let rem = iter.remainder();

        for chunk in iter {
            let a = u8x16::from_slice(&chunk);
            process_chunk(chunk, output, a, 16, v_below_space, v_b, v_c);
        }

        process_padded_chunk(rem, output, v_below_space, v_b, v_c);
    }

    if len < 16 {
        process_padded_chunk(bytes, output, v_below_space, v_b, v_c);
        return;
    }

    process(bytes, output, v_below_space, v_b, v_c);
}

enum ValueItem<'js> {
    Object(Object<'js>),
    Array(Array<'js>),
}

struct PathItem<'js> {
    value: ValueItem<'js>,
    index: usize,
    len: usize,
    parent_index: usize,
    parent_key: Option<String>,
}
impl<'js> PathItem<'js> {
    fn array(
        array: Array<'js>,
        len: usize,
        parent_index: usize,
        parent_key: Option<String>,
    ) -> Self {
        Self {
            value: ValueItem::Array(array),
            index: 0,
            len,
            parent_index,
            parent_key,
        }
    }

    fn object(
        object: Object<'js>,
        len: usize,
        parent_index: usize,
        parent_key: Option<String>,
    ) -> Self {
        Self {
            value: ValueItem::Object(object),
            index: 0,
            len,
            parent_index,
            parent_key,
        }
    }
}

pub fn json_stringify(ctx: &Ctx<'_>, value: Value) -> Result<Option<String>> {
    json_stringify_replacer_space(ctx, value, None, None)
}

pub fn json_stringify_replacer(
    ctx: &Ctx<'_>,
    value: Value,
    replacer: Option<Value>,
) -> Result<Option<String>> {
    json_stringify_replacer_space(ctx, value, replacer, None)
}

pub fn json_stringify_replacer_space(
    ctx: &Ctx<'_>,
    value: Value,
    replacer: Option<Value>,
    indentation: Option<String>,
) -> Result<Option<String>> {
    const CIRCULAR_REF_DETECTION_DEPTH: usize = 20;

    #[inline(always)]
    fn write_primitive(
        string: &mut String,
        value: &Value,
        key: Option<&str>,
        indentation: Option<&str>,
        depth: usize,
    ) -> Result<bool> {
        let type_of = value.type_of();

        if matches!(type_of, Type::Symbol | Type::Undefined) {
            return Ok(true);
        }
        let mut has_indent = false;
        if let Some(indentation) = indentation {
            string.push_str(&indentation.repeat(depth));
            has_indent = true;
        }

        if let Some(key) = key {
            write_key(string, key, has_indent);
        }

        match type_of {
            Type::Null => string.push_str("null"),
            Type::Bool => string.push_str(match value.as_bool().unwrap() {
                true => "true",
                false => "false",
            }),
            Type::Int => {
                let mut buffer = itoa::Buffer::new();
                string.push_str(buffer.format(value.as_int().unwrap()))
            }
            Type::Float => {
                let mut buffer = ryu::Buffer::new();
                string.push_str(buffer.format(value.as_float().unwrap()))
            }
            Type::String => write_string(string, &value.as_string().unwrap().to_string()?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    let mut result = String::with_capacity(128);
    if write_primitive(&mut result, &value, None, None, 0)? {
        return Ok(Some(result));
    }

    #[inline(always)]
    fn detect_circular_reference(
        ctx: &Ctx<'_>,
        value: &Object<'_>,
        key: Option<&str>,
        index: Option<usize>,
        parent: Option<&Object<'_>>,
        ancestors: &mut Vec<(usize, String)>,
    ) -> Result<()> {
        let parent_ptr = unsafe { parent.unwrap().as_raw().u.ptr as usize };
        let current_ptr = unsafe { value.as_raw().u.ptr as usize };

        while !ancestors.is_empty()
            && match ancestors.last() {
                Some((ptr, _)) => ptr != &parent_ptr,
                _ => false,
            }
        {
            ancestors.pop();
        }

        if ancestors.iter().any(|(ptr, _)| ptr == &current_ptr) {
            let mut iter = ancestors.into_iter();

            let first = &iter.next().unwrap().1;

            let mut path = iter
                .rev()
                .take(4)
                .rev()
                .fold(String::new(), |mut acc, (_, key)| {
                    if !key.starts_with("[") {
                        acc.push('.');
                    }
                    acc.push_str(key);
                    acc
                });

            if !first.starts_with("[") {
                path.push('.');
            }

            path.push_str(first);

            return Err(Exception::throw_type(
                ctx,
                &format!("Circular reference detected at: \"..{}\"", path),
            ));
        }
        ancestors.push((
            current_ptr,
            key.map(|k| k.to_string())
                .unwrap_or_else(|| format!("[{}]", index.unwrap_or_default())),
        ));

        Ok(())
    }

    #[inline(always)]
    fn append_value(
        ctx: &Ctx<'_>,
        result: &mut String,
        val: Value<'_>,
        depth: usize,
        indentation: Option<&str>,
        key: Option<&str>,
        index: Option<usize>,
        parent: Option<&Object<'_>>,
        ancestors: &mut Vec<(usize, String)>,
        replacer_fn: Option<&Function>,
        replacer_filter: Option<&HashSet<String>>,
    ) -> Result<()> {
        if !write_primitive(result, &val, None, indentation, depth)? {
            iterate(
                ctx,
                result,
                &val,
                depth + 1,
                indentation,
                key,
                index,
                parent,
                ancestors,
                replacer_fn,
                replacer_filter,
            )?;
        }

        Ok(())
    }

    #[inline(always)]
    fn write_key(string: &mut String, key: &str, indent: bool) {
        string.push('"');
        escape_json_string(string, key.as_bytes());
        if indent {
            string.push_str("\": ");
        } else {
            string.push_str("\":");
        }
    }

    #[inline(always)]
    fn write_string(string: &mut String, value: &str) {
        string.push('"');
        escape_json_string(string, value.as_bytes());
        string.push('"');
    }

    #[inline(always)]
    fn iterate(
        ctx: &Ctx<'_>,
        result: &mut String,
        elem: &Value,
        depth: usize,
        indentation: Option<&str>,
        key: Option<&str>,
        index: Option<usize>,
        parent: Option<&Object<'_>>,
        ancestors: &mut Vec<(usize, String)>,
        replacer_fn: Option<&Function>,
        replacer_filter: Option<&HashSet<String>>,
    ) -> Result<()> {
        let mut add_comma;
        match elem.type_of() {
            Type::Object => {
                let js_object = elem.as_object().unwrap();
                if js_object.contains_key(PredefinedAtom::ToJSON)? {
                    let to_json = js_object.get::<_, Function>(PredefinedAtom::ToJSON)?;
                    let val = to_json.call((This(js_object.clone()),))?;
                    append_value(
                        ctx,
                        result,
                        val,
                        depth,
                        indentation,
                        key,
                        None,
                        Some(js_object),
                        ancestors,
                        replacer_fn,
                        replacer_filter,
                    )?;
                    return Ok(());
                }

                //only start detect circular reference at this level
                if depth > CIRCULAR_REF_DETECTION_DEPTH {
                    detect_circular_reference(ctx, js_object, key, index, parent, ancestors)?;
                }

                result.push('{');

                add_comma = false;
                for key in js_object.keys::<String>() {
                    if add_comma {
                        if indentation.is_some() {
                            result.push_str(",\n");
                        } else {
                            result.push(',');
                        }
                    } else if indentation.is_some() {
                        result.push('\n');
                    }
                    let key = key?;
                    let val = js_object.get(&key)?;

                    if !write_primitive(result, &val, Some(&key), indentation, depth)? {
                        iterate(
                            ctx,
                            result,
                            &val,
                            depth + 1,
                            indentation,
                            Some(&key),
                            None,
                            Some(js_object),
                            ancestors,
                            replacer_fn,
                            replacer_filter,
                        )?;
                    }
                    add_comma = true;
                }
                if add_comma {
                    if let Some(indentation) = indentation {
                        result.push('\n');
                        result.push_str(&indentation.repeat(depth - 1));
                    }
                }
                result.push('}');
            }
            Type::Array => {
                result.push('[');
                add_comma = false;
                let js_array = elem.as_array().unwrap();
                //only start detect circular reference at this level
                if depth > CIRCULAR_REF_DETECTION_DEPTH {
                    detect_circular_reference(
                        ctx,
                        js_array.as_object(),
                        key,
                        index,
                        parent,
                        ancestors,
                    )?;
                }
                for (i, val) in js_array.iter::<Value>().enumerate() {
                    if add_comma {
                        if indentation.is_some() {
                            result.push_str(",\n");
                        } else {
                            result.push(',');
                        }
                    } else if indentation.is_some() {
                        result.push('\n');
                    }
                    let val = val?;
                    append_value(
                        ctx,
                        result,
                        val,
                        depth,
                        indentation,
                        None,
                        Some(i),
                        Some(js_array.as_object()),
                        ancestors,
                        replacer_fn,
                        replacer_filter,
                    )?;
                    add_comma = true;
                }
                if add_comma {
                    if let Some(indentation) = indentation {
                        result.push('\n');
                        result.push_str(&indentation.repeat(depth - 1));
                    }
                }
                result.push(']');
            }
            _ => {}
        }
        Ok(())
    }

    let mut replacer_fn = None;
    let mut replacer_filter = None;

    let tmp_function;

    if let Some(replacer) = replacer {
        if let Some(function) = replacer.as_function() {
            tmp_function = function.clone();
            replacer_fn = Some(&tmp_function);
        } else if let Some(array) = replacer.as_array() {
            let mut filter = HashSet::with_capacity(array.len());
            for value in array.clone().into_iter() {
                let value = value?;
                if let Some(string) = value.as_string() {
                    filter.insert(string.to_string()?);
                } else if let Some(number) = value.as_int() {
                    let mut buffer = itoa::Buffer::new();
                    filter.insert(buffer.format(number).to_string());
                } else if let Some(number) = value.as_float() {
                    let mut buffer = ryu::Buffer::new();
                    filter.insert(buffer.format(number).to_string());
                }
            }
            replacer_filter = Some(filter);
        } else {
            return Err(Exception::throw_type(
                ctx,
                "\"replacer\" argument must be a Function or Array",
            ));
        }
    }

    let mut ancestors = Vec::with_capacity(10);
    iterate(
        ctx,
        &mut result,
        &value,
        1,
        indentation.as_deref(),
        None,
        None,
        None,
        &mut ancestors,
        replacer_fn,
        replacer_filter.as_ref(),
    )?;
    Ok(Some(result))
}

/// Parse json into a JavaScript value.
pub fn json_parse<'js>(ctx: &Ctx<'js>, mut json: Vec<u8>) -> Result<Value<'js>> {
    let _now = Instant::now();

    let tape = simd_json::to_tape(&mut json).unwrap();

    let mut str_key = "";
    let mut last_is_string = false;

    let tape = tape.0;
    let first = tape.first();

    if first.is_none() {
        return Undefined.into_js(ctx);
    }
    let first = first.unwrap();

    match first {
        Node::String(value) => {
            return value.into_js(ctx);
        }
        Node::Static(node) => return static_node_to_value(ctx, *node),
        _ => {}
    };

    let mut path_data = Vec::<PathItem>::with_capacity(10);

    #[inline(always)]
    fn static_node_to_value<'js>(ctx: &Ctx<'js>, node: StaticNode) -> Result<Value<'js>> {
        Ok(match node {
            StaticNode::I64(value) => value.into_js(ctx)?,
            StaticNode::U64(value) => value.into_js(ctx)?,
            StaticNode::F64(value) => value.into_js(ctx)?,
            StaticNode::Bool(value) => value.into_js(ctx)?,
            StaticNode::Null => Null.into_js(ctx)?,
        })
    }

    let mut current_obj;

    for val in tape {
        match val {
            Node::String(value) => {
                current_obj = path_data.last_mut().unwrap();

                match &current_obj.value {
                    ValueItem::Object(obj) => {
                        if !last_is_string {
                            str_key = value;
                            last_is_string = true;
                            continue;
                        } else {
                            obj.set(str_key, value)?;
                            current_obj.index += 1;
                            last_is_string = false
                        }
                    }
                    ValueItem::Array(array) => {
                        array.set(current_obj.index, value)?;
                        current_obj.index += 1;
                    }
                }
            }
            Node::Object { len, count: _ } => {
                let js_object = Object::new(ctx.clone())?;
                let item = if let Some(current_obj) = path_data.last_mut() {
                    current_obj.index += 1;
                    PathItem::object(
                        js_object,
                        len,
                        current_obj.index - 1,
                        match current_obj.value {
                            ValueItem::Object(_) => Some(str_key.to_string()),
                            ValueItem::Array(_) => None,
                        },
                    )
                } else {
                    PathItem::object(js_object, len, 0, None)
                };

                path_data.push(item);
                last_is_string = false;
            }
            Node::Array { len, count: _ } => {
                let js_array = Array::new(ctx.clone())?;
                let item = if let Some(current_obj) = path_data.last_mut() {
                    current_obj.index += 1;
                    PathItem::array(
                        js_array,
                        len,
                        current_obj.index - 1,
                        match current_obj.value {
                            ValueItem::Object(_) => Some(str_key.to_string()),
                            ValueItem::Array(_) => None,
                        },
                    )
                } else {
                    PathItem::array(js_array, len, 0, None)
                };
                path_data.push(item);
                last_is_string = false;
            }
            Node::Static(node) => {
                last_is_string = false;
                current_obj = path_data.last_mut().unwrap();
                let value = static_node_to_value(ctx, node);
                match &current_obj.value {
                    ValueItem::Object(obj) => obj.set(str_key, value)?,
                    ValueItem::Array(arr) => arr.set(current_obj.index, value)?,
                }
                current_obj.index += 1;
            }
        }

        current_obj = path_data.last_mut().unwrap();
        while current_obj.index == current_obj.len {
            let data = path_data.pop().unwrap();
            if let Some(last_obj) = path_data.last_mut() {
                let value = match data.value {
                    ValueItem::Object(obj) => obj,
                    ValueItem::Array(arr) => arr.into_object(),
                };
                match &last_obj.value {
                    ValueItem::Object(obj) => obj.set(data.parent_key.unwrap(), value)?,
                    ValueItem::Array(arr) => arr.set(data.parent_index, value)?,
                }
                current_obj = last_obj
            } else {
                let res = match &data.value {
                    ValueItem::Object(obj) => obj.clone().into_value(),
                    ValueItem::Array(arr) => arr.clone().into_value(),
                };
                return Ok(res);
            }
        }
    }

    Undefined.into_js(ctx)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rquickjs::{Array, CatchResultExt, Null, Object, Undefined, Value};

    use crate::{
        json::{escape_json, json_parse, json_stringify, json_stringify_replacer_space},
        test_utils::utils::with_runtime,
    };

    static JSON: &str = r#"{"organization":{"name":"TechCorp","founding_year":2000,"departments":[{"name":"Engineering","head":{"name":"Alice Smith","title":"VP of Engineering","contact":{"email":"alice.smith@techcorp.com","phone":"+1 (555) 123-4567"}},"employees":[{"id":101,"name":"Bob Johnson","position":"Software Engineer","contact":{"email":"bob.johnson@techcorp.com","phone":"+1 (555) 234-5678"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Developing a revolutionary software solution for clients.","start_date":"2023-01-15","end_date":null,"team":[{"id":201,"name":"Sara Davis","role":"UI/UX Designer"},{"id":202,"name":"Charlie Brown","role":"Quality Assurance Engineer"}]},{"project_id":"P002","name":"Project B","status":"Completed","description":"Upgrading existing systems to enhance performance.","start_date":"2022-05-01","end_date":"2022-11-30","team":[{"id":203,"name":"Emily White","role":"Systems Architect"},{"id":204,"name":"James Green","role":"Database Administrator"}]}]},{"id":102,"name":"Carol Williams","position":"Senior Software Engineer","contact":{"email":"carol.williams@techcorp.com","phone":"+1 (555) 345-6789"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Working on the backend development of Project A.","start_date":"2023-01-15","end_date":null,"team":[{"id":205,"name":"Alex Turner","role":"DevOps Engineer"},{"id":206,"name":"Mia Garcia","role":"Software Developer"}]},{"project_id":"P003","name":"Project C","status":"Planning","description":"Researching and planning for a future project.","start_date":null,"end_date":null,"team":[]}]}]},{"name":"Marketing","head":{"name":"David Brown","title":"VP of Marketing","contact":{"email":"david.brown@techcorp.com","phone":"+1 (555) 456-7890"}},"employees":[{"id":201,"name":"Eva Miller","position":"Marketing Specialist","contact":{"email":"eva.miller@techcorp.com","phone":"+1 (555) 567-8901"},"campaigns":[{"campaign_id":"C001","name":"Product Launch","status":"Upcoming","description":"Planning for the launch of a new product line.","start_date":"2023-03-01","end_date":null,"team":[{"id":301,"name":"Oliver Martinez","role":"Graphic Designer"},{"id":302,"name":"Sophie Johnson","role":"Content Writer"}]},{"campaign_id":"C002","name":"Brand Awareness","status":"Ongoing","description":"Executing strategies to increase brand visibility.","start_date":"2022-11-15","end_date":"2023-01-31","team":[{"id":303,"name":"Liam Taylor","role":"Social Media Manager"},{"id":304,"name":"Ava Clark","role":"Marketing Analyst"}]}]}]}]}}"#;

    #[test]
    fn escape_json_simple() {
        assert_eq!(escape_json(b"Hello, World!"), "Hello, World!");
    }

    #[test]
    fn escape_json_quotes() {
        assert_eq!(escape_json(b"\"quoted\""), "\\\"quoted\\\"");
    }

    #[test]
    fn escape_json_backslash() {
        assert_eq!(escape_json(b"back\\slash"), "back\\\\slash");
    }

    #[test]
    fn escape_json_newline() {
        assert_eq!(escape_json(b"line\nbreak"), "line\\nbreak");
    }

    #[test]
    fn escape_json_tab() {
        assert_eq!(escape_json(b"tab\tcharacter"), "tab\\tcharacter");
    }

    #[test]
    fn escape_json_unicode() {
        assert_eq!(
            escape_json("unicode: \u{1F609}".as_bytes()),
            "unicode: \u{1F609}"
        );
    }

    #[test]
    fn escape_json_special_characters() {
        assert_eq!(
            escape_json(b"!@#$%^&*()_+-=[]{}|;':,.<>?/"),
            "!@#$%^&*()_+-=[]{}|;':,.<>?/"
        );
    }

    #[test]
    fn escape_json_mixed_characters() {
        assert_eq!(
            escape_json(b"123\"\"45678901\"234567"),
            "123\\\"\\\"45678901\\\"234567"
        );
    }

    #[tokio::test]
    async fn json_parser() {
        with_runtime(|ctx| {
            let json_data = [
                r#"{"aa\"\"aaaaaaaaaaaaaaaa":"a","b":"bbb"}"#,
                r#"{"a":"aaaaaaaaaaaaaaaaaa","b":"bbb"}"#,
                r#"{"a":["a","a","aaaa","a"],"b":"b"}"#,
                r#"{"type":"Buffer","data":[10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10]}"#,
                r#"{"a":[{"object2":{"key1":"value1","key2":123,"key3":false,"nestedObject":{"nestedKey":"nestedValue"}},"string":"Hello, World!","emptyObj":{},"emptyArr":[],"number":42,"boolean":true,"nullValue":null,"array":[1,2,3,"four",5.5,true,null],"object":{"key1":"value1","key2":123,"key3":false,"nestedObject":{"nestedKey":"nestedValue"}}}]}"#,
                JSON,
            ];

            for json_str in json_data {
                println!("==========");
                println!("{}", json_str);
                println!("==========");
                let json = json_str.to_string();
                let json2 = json.clone();

                let value = json_parse(&ctx, json2.into_bytes())?;
                let new_json = json_stringify_replacer_space(&ctx, value.clone(),None,Some("  ".into()))?.unwrap();
                let builtin_json = ctx.json_stringify_replacer_space(value,Null,"  ".to_string())?.unwrap().to_string()?;
                println!("==========");
                println!("{}", new_json);
                assert_eq!(new_json, builtin_json);
            }

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_stringify_objects() {
        with_runtime(|ctx| {
            let date: Value = ctx.eval("new Date(0)")?;
            let stringified = json_stringify(&ctx, date.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(date)?.unwrap().to_string()?;
            assert_eq!(stringified, stringified_2);
            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_circular_ref() {
        with_runtime(|ctx| {
            let obj1 = Object::new(ctx.clone())?;
            let obj2 = Object::new(ctx.clone())?;
            let obj3 = Object::new(ctx.clone())?;
            let obj4 = Object::new(ctx.clone())?;
            obj4.set("key", "value")?;
            obj3.set("sub2", obj4.clone())?;
            obj2.set("sub1", obj3)?;
            obj1.set("root1", obj2.clone())?;
            obj1.set("root2", obj2.clone())?;
            obj1.set("root3", obj2.clone())?;

            let value = obj1.clone().into_value();

            let stringified = json_stringify(&ctx, value.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(value.clone())?.unwrap().to_string()?;
            assert_eq!(stringified, stringified_2);

            obj4.set("recursive", obj1.clone())?;

            let stringified = json_stringify(&ctx, value.clone());

            if let Err(error_message) = stringified.catch(&ctx) {
                let error_str = error_message.to_string();
                assert_eq!(
                    "Error: Circular reference detected at: \"...root1.sub1.sub2.recursive\"\n",
                    error_str
                )
            } else {
                panic!("Expected an error, but got Ok");
            }

            let array1 = Array::new(ctx.clone())?;
            let array2 = Array::new(ctx.clone())?;
            let array3 = Array::new(ctx.clone())?;

            let obj5 = Object::new(ctx.clone())?;
            obj5.set("key", obj1.clone())?;
            array3.set(2, obj5)?;
            array2.set(1, array3)?;
            array1.set(0, array2)?;

            obj4.remove("recursive")?;
            obj1.set("recursiveArray", array1)?;

            let stringified = json_stringify(&ctx, value.clone());

            if let Err(error_message) = stringified.catch(&ctx) {
                let error_str = error_message.to_string();
                assert_eq!(
                    "Error: Circular reference detected at: \"...recursiveArray[0][1][2].key\"\n",
                    error_str
                )
            } else {
                panic!("Expected an error, but got Ok");
            }

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_perf() {
        let json = JSON.to_string();

        fn generate_json(child_json: &str, size: usize) -> String {
            let mut json = String::with_capacity(child_json.len() * size);
            json.push('{');
            for i in 0..size {
                json.push_str("\"obj");
                json.push_str(&i.to_string());
                json.push_str("\":");
                json.push_str(child_json);
                json.push(',');
            }
            json.push_str("\"array\":[");
            for i in 0..size {
                json.push_str(child_json);
                if i < size - 1 {
                    json.push(',');
                }
            }

            json.push_str("]}");
            json
        }

        let data = [
            json.clone(),
            generate_json(&json, 10),
            generate_json(&json, 100),
            generate_json(&json, 1000),
        ];

        with_runtime(|ctx| {
            for (_i, data) in data.into_iter().enumerate() {
                let size = data.len();
                let data2 = data.clone().into_bytes();
                let now = Instant::now();
                let value = json_parse(&ctx, data2).unwrap();

                let t1 = now.elapsed();

                let now = Instant::now();
                let value2 = ctx.json_parse(data).unwrap();
                let t2 = now.elapsed();

                let value3 = value.clone();

                let now = Instant::now();
                let json_string1 = json_stringify(&ctx, value3).unwrap().unwrap().to_string();

                let t3 = now.elapsed();

                let now = Instant::now();
                let json_string2 = ctx
                    .json_stringify(value2)
                    .unwrap()
                    .unwrap()
                    .to_string()
                    .unwrap();
                let t4 = now.elapsed();

                let json_string3 = ctx
                    .json_stringify(value)
                    .unwrap()
                    .unwrap()
                    .to_string()
                    .unwrap();

                let json_1_len = json_string1.len();
                let json_2_len = json_string2.len();
                let json_3_len = json_string3.len();

                //we can't check for full equality since simd-json uses HashMap that randomizes key order when parsing. See https://github.com/simd-lite/simd-json/issues/270
                assert_eq!(json_1_len, json_2_len);
                assert_eq!(json_2_len, json_3_len);
                assert_eq!(json_1_len, json_3_len);

                println!(
                    "Size {}:\n\tparse: {:?} vs. {:?}\n\tstringify: {:?} vs. {:?}",
                    size, t1, t2, t3, t4
                );
            }
            Ok(())
        })
        .await;
    }
}
