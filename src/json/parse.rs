use std::time::Instant;

use rquickjs::{Array, Ctx, IntoJs, Null, Object, Result, Undefined, Value};

use simd_json::{Node, StaticNode};

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

pub fn json_parse<'js>(ctx: &Ctx<'js>, mut json: Vec<u8>) -> Result<Value<'js>> {
    let _now = Instant::now();

    let tape = simd_json::to_tape(&mut json).or_throw_msg(ctx, "Invalid json")?;

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
