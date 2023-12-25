use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::Hasher;
use std::ops::BitXor;
use std::thread;
use std::time::Instant;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rquickjs::Undefined;
use rquickjs::{
    atom::PredefinedAtom, function::This, Array, Ctx, Function, IntoJs, Null, Object, Result,
    Type::Uninitialized, Value,
};
use simd_json::borrowed::Value as JsonValue;
use simd_json::{Node, StaticNode};

use tracing::trace;
use v_jsonescape::escape;

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

use crate::util::ResultExt;

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

enum JsonString {
    Value(String),
    String(String),
    Ignored,
}

#[inline(always)]
fn to_json_string(value: &Value) -> Result<Option<JsonString>> {
    Ok(Some(match value.type_of() {
        rquickjs::Type::Undefined => JsonString::Ignored,
        rquickjs::Type::Null => JsonString::Value("null".into()),
        rquickjs::Type::Bool => JsonString::Value(value.as_bool().unwrap().to_string()),
        rquickjs::Type::Int => JsonString::Value(value.as_int().unwrap().to_string()),
        rquickjs::Type::Float => JsonString::Value(value.as_float().unwrap().to_string()),
        rquickjs::Type::String => {
            JsonString::String(escape(&value.as_string().unwrap().to_string()?).to_string())
        }
        rquickjs::Type::Symbol => JsonString::Ignored,
        _ => return Ok(None),
    }))
}

pub fn json_stringify(ctx: &Ctx<'_>, value: Value) -> Result<Option<String>> {
    let mut result = String::with_capacity(10);
    if let Some(primitive) = to_json_string(&value)? {
        return Ok(match primitive {
            JsonString::Value(value) => Some(value),
            JsonString::Ignored => None,
            JsonString::String(value) => Some(format!("\"{}\"", value)),
        });
    }

    #[inline(always)]
    fn append_value(ctx: &Ctx<'_>, result: &mut String, val: Value<'_>) -> Result<()> {
        if let Some(primitive) = to_json_string(&val)? {
            match primitive {
                JsonString::Value(value) => result.push_str(&value),
                JsonString::Ignored => {}
                JsonString::String(value) => write_string(result, value),
            }
        } else {
            iterate(ctx, result, &val)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_key(result: &mut String, key: &str) {
        result.push('"');
        result.push_str(&escape(key).to_string());
        result.push_str("\":");
    }

    #[inline(always)]
    fn write_string(result: &mut String, value: String) {
        result.push('"');
        result.push_str(&value);
        result.push('"');
    }

    #[inline(always)]
    fn iterate(ctx: &Ctx<'_>, result: &mut String, elem: &Value) -> Result<()> {
        match elem.type_of() {
            rquickjs::Type::Object => {
                let js_object = elem.as_object().unwrap();
                if js_object.contains_key(PredefinedAtom::ToJSON)? {
                    let to_json = js_object.get::<_, Function>(PredefinedAtom::ToJSON)?;
                    let val = to_json.call((This(js_object.clone()),))?;
                    append_value(ctx, result, val)?;
                    return Ok(());
                }
                result.push('{');
                let keys = js_object.keys::<String>();
                let length = keys.len();

                for (idx, key) in keys.enumerate() {
                    let key = key?;
                    let val = js_object.get(&key)?;
                    if let Some(primitive) = to_json_string(&val)? {
                        match primitive {
                            JsonString::Value(value) => {
                                write_key(result, &key);
                                result.push_str(&value);
                            }
                            JsonString::Ignored => {}
                            JsonString::String(value) => {
                                write_key(result, &key);
                                write_string(result, value);
                            }
                        }
                    } else {
                        write_key(result, &key);
                        iterate(ctx, result, &val)?;
                    }
                    if idx < length - 1 {
                        result.push(',');
                    }
                }
                result.push('}');
            }
            rquickjs::Type::Array => {
                result.push('[');
                let js_array = elem.as_array().unwrap();
                let length = js_array.len();
                for (idx, val) in js_array.iter::<Value>().enumerate() {
                    let val = val?;
                    append_value(ctx, result, val)?;
                    if idx < length - 1 {
                        result.push(',');
                    }
                }
                result.push(']');
            }
            _ => {}
        }
        Ok(())
    }

    iterate(ctx, &mut result, &value)?;
    Ok(Some(result))
}

/// Parse json into a JavaScript value.
pub fn json_parse2<'js>(ctx: &Ctx<'js>, mut bytes: Vec<u8>) -> Result<Value<'js>> {
    let now = Instant::now();
    let root = simd_json::to_borrowed_value(&mut bytes).or_throw(ctx)?;
    println!("simd_json parse took: {:?}", now.elapsed());
    if let Some(value) = get_primitive(ctx, &root)? {
        return Ok(value);
    }

    fn iterate<'js>(elem: &JsonValue, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match elem {
            JsonValue::Array(json_array) => {
                let js_array = Array::new(ctx.clone())?;

                for (idx, val) in json_array.iter().enumerate() {
                    if let Some(primitive) = get_primitive(ctx, val)? {
                        js_array.set(idx, primitive)?;
                    } else {
                        js_array.set(idx, iterate(val, ctx)?)?;
                    }
                }
                return Ok(js_array.into_value());
            }
            JsonValue::Object(json_object) => {
                let js_object = Object::new(ctx.clone())?;
                for (key, val) in json_object.iter() {
                    if let Some(primitive) = get_primitive(ctx, val)? {
                        js_object.set(key.to_string(), primitive)?;
                    } else {
                        js_object.set(key.to_string(), iterate(val, ctx)?)?;
                    }
                }
                return Ok(js_object.into_value());
            }
            _ => unreachable!(),
        }
    }

    iterate(&root, ctx)
}

/// Parse json into a JavaScript value.
pub fn json_parse<'js>(ctx: &Ctx<'js>, mut json: Vec<u8>) -> Result<Value<'js>> {
    let now = Instant::now();

    let tape = simd_json::to_tape(&mut json).unwrap();

    let mut str_key = "";
    let mut last_is_string = false;

    let tape = tape.0;
    let first = tape.first();

    if let None = first {
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
            StaticNode::I64(value) => value.into_js(&ctx)?,
            StaticNode::U64(value) => value.into_js(&ctx)?,
            StaticNode::F64(value) => value.into_js(&ctx)?,
            StaticNode::Bool(value) => value.into_js(&ctx)?,
            StaticNode::Null => Null.into_js(&ctx)?,
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
            Node::Object { len, count } => {
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
            Node::Array { len, count } => {
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

#[inline(always)]
fn get_primitive<'js>(ctx: &Ctx<'js>, elem: &JsonValue<'_>) -> Result<Option<Value<'js>>> {
    Ok(match elem {
        JsonValue::Static(static_node) => Some(match static_node {
            simd_json::StaticNode::I64(val) => val.into_js(ctx)?,
            simd_json::StaticNode::U64(val) => val.into_js(ctx)?,
            simd_json::StaticNode::F64(val) => val.into_js(ctx)?,
            simd_json::StaticNode::Bool(val) => val.into_js(ctx)?,
            simd_json::StaticNode::Null => Null.into_js(ctx)?,
        }),
        JsonValue::String(string) => Some(string.into_js(ctx)?),
        _ => None,
    })
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use std::{fs, time::Instant};

    use rquickjs::Value;

    use crate::{
        json::{json_parse, json_stringify},
        test_utils::utils::with_runtime,
    };

    static JSON: &str = r#"{"organization":{"name":"TechCorp","founding_year":2000,"departments":[{"name":"Engineering","head":{"name":"Alice Smith","title":"VP of Engineering","contact":{"email":"alice.smith@techcorp.com","phone":"+1 (555) 123-4567"}},"employees":[{"id":101,"name":"Bob Johnson","position":"Software Engineer","contact":{"email":"bob.johnson@techcorp.com","phone":"+1 (555) 234-5678"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Developing a revolutionary software solution for clients.","start_date":"2023-01-15","end_date":null,"team":[{"id":201,"name":"Sara Davis","role":"UI/UX Designer"},{"id":202,"name":"Charlie Brown","role":"Quality Assurance Engineer"}]},{"project_id":"P002","name":"Project B","status":"Completed","description":"Upgrading existing systems to enhance performance.","start_date":"2022-05-01","end_date":"2022-11-30","team":[{"id":203,"name":"Emily White","role":"Systems Architect"},{"id":204,"name":"James Green","role":"Database Administrator"}]}]},{"id":102,"name":"Carol Williams","position":"Senior Software Engineer","contact":{"email":"carol.williams@techcorp.com","phone":"+1 (555) 345-6789"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Working on the backend development of Project A.","start_date":"2023-01-15","end_date":null,"team":[{"id":205,"name":"Alex Turner","role":"DevOps Engineer"},{"id":206,"name":"Mia Garcia","role":"Software Developer"}]},{"project_id":"P003","name":"Project C","status":"Planning","description":"Researching and planning for a future project.","start_date":null,"end_date":null,"team":[]}]}]},{"name":"Marketing","head":{"name":"David Brown","title":"VP of Marketing","contact":{"email":"david.brown@techcorp.com","phone":"+1 (555) 456-7890"}},"employees":[{"id":201,"name":"Eva Miller","position":"Marketing Specialist","contact":{"email":"eva.miller@techcorp.com","phone":"+1 (555) 567-8901"},"campaigns":[{"campaign_id":"C001","name":"Product Launch","status":"Upcoming","description":"Planning for the launch of a new product line.","start_date":"2023-03-01","end_date":null,"team":[{"id":301,"name":"Oliver Martinez","role":"Graphic Designer"},{"id":302,"name":"Sophie Johnson","role":"Content Writer"}]},{"campaign_id":"C002","name":"Brand Awareness","status":"Ongoing","description":"Executing strategies to increase brand visibility.","start_date":"2022-11-15","end_date":"2023-01-31","team":[{"id":303,"name":"Liam Taylor","role":"Social Media Manager"},{"id":304,"name":"Ava Clark","role":"Marketing Analyst"}]}]}]}]}}"#;

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
                let new_json = json_stringify(&ctx, value.clone())?.unwrap();
                let builtin_json = ctx.json_stringify(value)?.unwrap().to_string()?;
                println!("==========");
                assert_eq!(new_json, builtin_json);
            }

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_stringify_objects() {
        with_runtime(|ctx| {
            let date: Value = ctx.eval("new Date(0)").unwrap();
            let stringified = json_stringify(&ctx, date.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(date)?.unwrap().to_string()?;
            assert_eq!(stringified, stringified_2);
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
            generate_json(&json, 10_000),
            generate_json(&json, 100_000),
        ];

        with_runtime(|ctx| {
            for (i, data) in data.into_iter().enumerate() {
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
