use crate::utils::result::ResultExt;
use rquickjs::{Array, Ctx, IntoJs, Null, Object, Result, Undefined, Value};
use simd_json::{Node, StaticNode};

pub fn json_parse<'js, T: Into<Vec<u8>>>(ctx: &Ctx<'js>, json: T) -> Result<Value<'js>> {
    let mut json: Vec<u8> = json.into();
    let tape = simd_json::to_tape(&mut json).or_throw(ctx)?;
    let tape = tape.0;

    if let Some(first) = tape.first() {
        return match first {
            Node::String(value) => value.into_js(ctx),
            Node::Static(node) => static_node_to_value(ctx, *node),
            _ => parse_node(ctx, &tape, 0).map(|(value, _)| value),
        };
    }

    Undefined.into_js(ctx)
}

#[inline(always)]
fn static_node_to_value<'js>(ctx: &Ctx<'js>, node: StaticNode) -> Result<Value<'js>> {
    match node {
        StaticNode::I64(value) => value.into_js(ctx),
        StaticNode::U64(value) => value.into_js(ctx),
        StaticNode::F64(value) => value.into_js(ctx),
        StaticNode::Bool(value) => value.into_js(ctx),
        StaticNode::Null => Null.into_js(ctx),
    }
}

fn parse_node<'js>(ctx: &Ctx<'js>, tape: &[Node], index: usize) -> Result<(Value<'js>, usize)> {
    match &tape[index] {
        Node::String(value) => Ok((value.into_js(ctx)?, index + 1)),
        Node::Static(node) => Ok((static_node_to_value(ctx, *node)?, index + 1)),
        Node::Object { len, .. } => {
            let js_object = Object::new(ctx.clone())?;
            let mut current_index = index + 1;

            for _ in 0..*len {
                if let Node::String(key) = &tape[current_index] {
                    current_index += 1;
                    let (value, new_index) = parse_node(ctx, tape, current_index)?;
                    current_index = new_index;
                    js_object.set(*key, value)?;
                }
            }

            Ok((js_object.into_value(), current_index))
        },
        Node::Array { len, .. } => {
            let js_array = Array::new(ctx.clone())?;
            let mut current_index = index + 1;

            for i in 0..*len {
                let (value, new_index) = parse_node(ctx, tape, current_index)?;
                current_index = new_index;
                js_array.set(i, value)?;
            }

            Ok((js_array.into_value(), current_index))
        },
    }
}
