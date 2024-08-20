// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use fxhash::{FxBuildHasher, FxHashSet};

use rquickjs::function::This;

use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt},
    Array, Ctx, Function, IntoJs, Null, Object, Result, Type, Value,
};

use super::object::ObjectExt;

#[derive(Debug)]
enum StackItem<'js> {
    Value(usize, Value<'js>, Option<String>, Option<usize>),
    ObjectEnd,
}

#[derive(Debug)]
enum ObjectType {
    Set,
    Map,
}

#[derive(Debug)]
enum TapeValue<'js> {
    Array(Array<'js>),
    Object(Object<'js>),
    Value(Value<'js>),
    Collection(Option<Value<'js>>, ObjectType),
}

#[derive(Debug)]
struct TapeItem<'js> {
    parent: usize,
    object_key: Option<String>,
    array_index: Option<usize>,
    value: TapeValue<'js>,
}

pub fn structured_clone<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    options: Opt<Object<'js>>,
) -> Result<Value<'js>> {
    let globals = ctx.globals();
    let date_ctor: Constructor = globals.get(PredefinedAtom::Date)?;
    let map_ctor: Constructor = globals.get(PredefinedAtom::Map)?;
    let set_ctor: Constructor = globals.get(PredefinedAtom::Set)?;
    let reg_exp_ctor: Constructor = globals.get(PredefinedAtom::RegExp)?;
    let error_ctor: Constructor = globals.get(PredefinedAtom::Error)?;
    let array_ctor: Constructor = globals.get(PredefinedAtom::Array)?;
    let array_from: Function = array_ctor.get(PredefinedAtom::From)?;
    let array_buffer: Constructor = globals.get(PredefinedAtom::ArrayBuffer)?;
    let is_view_fn: Function = array_buffer.get("isView")?;

    let mut transfer_set = None;

    if let Some(options) = options.0 {
        if let Some(transfer_array) = options.get_optional::<_, Array>("transfer")? {
            let mut set =
                FxHashSet::with_capacity_and_hasher(transfer_array.len(), FxBuildHasher::default());

            for item in transfer_array.iter::<Value>() {
                set.insert(item?);
            }
            transfer_set = Some(set);
        }
    }

    let mut tape = Vec::<TapeItem>::with_capacity(10);
    let mut stack = Vec::with_capacity(10);
    let mut visited = Vec::<(usize, usize)>::with_capacity(10);
    let mut index = 0usize;

    stack.push(StackItem::Value(0, value, None, None));

    while let Some(item) = stack.pop() {
        match item {
            StackItem::Value(parent, value, mut object_key, array_index) => {
                if let Some(set) = &transfer_set {
                    if let Some(value) = set.get(&value) {
                        append_transfer_value(&mut tape, value, parent, object_key, array_index)?;
                        index += 1;
                        continue;
                    }
                }
                match value.type_of() {
                    Type::Object => {
                        if check_circular(
                            &mut tape,
                            &mut visited,
                            &value,
                            parent,
                            &mut object_key,
                            array_index,
                            index,
                        ) {
                            index += 1;
                            continue;
                        }

                        //unsafe OK since we're guaranteed to be object by the match
                        let object = unsafe { value.as_object().unwrap_unchecked() };

                        if object.is_instance_of(&date_ctor) {
                            append_ctor_value(
                                &mut tape,
                                object,
                                &date_ctor,
                                parent,
                                object_key,
                                array_index,
                            )?;
                            index += 1;
                            continue;
                        }

                        if object.is_instance_of(&reg_exp_ctor) {
                            append_ctor_value(
                                &mut tape,
                                object,
                                &reg_exp_ctor,
                                parent,
                                object_key,
                                array_index,
                            )?;
                            index += 1;
                            continue;
                        }

                        let is_collection = if object.is_instance_of(&set_ctor) {
                            Some(ObjectType::Set)
                        } else if object.is_instance_of(&map_ctor) {
                            Some(ObjectType::Map)
                        } else {
                            None
                        };

                        if let Some(collection_type) = is_collection {
                            append_collection(
                                &mut tape,
                                &array_from,
                                object,
                                parent,
                                object_key,
                                array_index,
                                collection_type,
                                &mut stack,
                                index,
                            )?;

                            index += 1;
                            continue;
                        }

                        if is_view_fn.call::<_, bool>((value.clone(),))? {
                            append_buffer(&mut tape, object, parent, object_key, array_index)?;
                            index += 1;
                            continue;
                        }

                        let new: Object<'_> = if object.is_instance_of(&error_ctor) {
                            error_ctor.construct(("",))
                        } else {
                            Object::new(ctx.clone())
                        }?;

                        tape.push(TapeItem {
                            parent,
                            object_key,
                            array_index,
                            value: TapeValue::Object(new),
                        });
                        stack.push(StackItem::ObjectEnd);

                        for key in object.keys::<String>() {
                            let key = key?;
                            let value = object.get(&key)?;
                            stack.push(StackItem::Value(index, value, Some(key), None));
                        }
                    },
                    Type::Array => {
                        if check_circular(
                            &mut tape,
                            &mut visited,
                            &value,
                            parent,
                            &mut object_key,
                            array_index,
                            index,
                        ) {
                            index += 1;
                            continue;
                        }
                        let new = Array::new(ctx.clone())?;
                        tape.push(TapeItem {
                            parent,
                            object_key,
                            array_index,
                            value: TapeValue::Array(new),
                        });
                        stack.push(StackItem::ObjectEnd);
                        //unsafe OK since we're guaranteed to be object by the match
                        let array = unsafe { value.as_array().unwrap_unchecked() };

                        //reverse for loop of items in array
                        for array_index in (0usize..array.len()).rev() {
                            stack.push(StackItem::Value(
                                index,
                                array.get(array_index)?,
                                None,
                                Some(array_index),
                            ));
                        }
                    },
                    _ => {
                        tape.push(TapeItem {
                            parent,
                            object_key,
                            array_index,
                            value: TapeValue::Value(value),
                        });
                    },
                }
                index += 1;
            },
            StackItem::ObjectEnd => {
                visited.pop();
            },
        }
    }

    while let Some(item) = tape.pop() {
        let value = match item.value {
            TapeValue::Array(array) => array.into_value(),
            TapeValue::Object(object) => object.into_value(),
            TapeValue::Value(value) => value,
            TapeValue::Collection(mut value, _) => value.take().unwrap(),
        };
        if tape.is_empty() {
            return Ok(value);
        }
        let parent = &mut tape[item.parent];
        let array_index = item.array_index;
        let object_key = item.object_key;
        match &mut parent.value {
            TapeValue::Array(array) => {
                array.set(array_index.unwrap(), value)?;
            },
            TapeValue::Object(object) => {
                let string = object_key.unwrap();
                object.set(string, value)?;
            },
            TapeValue::Collection(collection_value, collection_type) => {
                match collection_type {
                    ObjectType::Set => {
                        collection_value.replace(set_ctor.construct((value,))?);
                    },
                    ObjectType::Map => {
                        collection_value.replace(map_ctor.construct((value,))?);
                    },
                };
            },
            _ => {},
        };
    }

    Null.into_js(ctx)
}

#[inline(always)]
#[cold]
fn append_buffer<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    object: &Object<'js>,
    parent: usize,
    object_key: Option<String>,
    array_index: Option<usize>,
) -> Result<()> {
    let ctor: Constructor = object.get(PredefinedAtom::Constructor)?;
    let slice: Function = object.get("slice")?;
    let clone: Value = slice.call((This(object.clone()),))?;
    let new = ctor.construct((clone,))?;
    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Value(new),
    });
    Ok(())
}

#[inline(always)]
#[cold]
#[allow(clippy::too_many_arguments)]
fn append_collection<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    array_from: &Function<'js>,
    object: &Object<'js>,
    parent: usize,
    object_key: Option<String>,
    array_index: Option<usize>,
    collection_type: ObjectType,
    stack: &mut Vec<StackItem<'js>>,
    index: usize,
) -> Result<()> {
    let array: Array = array_from.call((object.clone(),))?;
    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Collection(None, collection_type),
    });
    stack.push(StackItem::ObjectEnd);
    stack.push(StackItem::Value(index, array.into(), None, None));
    Ok(())
}

#[inline(always)]
fn check_circular(
    tape: &mut Vec<TapeItem>,
    visited: &mut Vec<(usize, usize)>,
    value: &Value<'_>,
    parent: usize,
    object_key: &mut Option<String>,
    array_index: Option<usize>,
    index: usize,
) -> bool {
    let hash = fxhash::hash(value);
    if let Some(visited) = visited.iter().find(|v| v.0 == hash) {
        append_circular(tape, visited, object_key, parent, array_index);
        return true;
    }
    visited.push((hash, index));
    false
}

#[inline(always)]
#[cold]
fn append_transfer_value<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    value: &Value<'js>,
    parent: usize,
    object_key: Option<String>,
    array_index: Option<usize>,
) -> Result<()> {
    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Value(value.clone()),
    });
    Ok(())
}

#[inline(always)]
#[cold]
fn append_circular(
    tape: &mut Vec<TapeItem<'_>>,
    visited: &(usize, usize),
    object_key: &mut Option<String>,
    parent: usize,
    array_index: Option<usize>,
) {
    let value = match &tape[visited.1].value {
        TapeValue::Array(array) => array.clone().into_value(),
        TapeValue::Object(object) => object.clone().into_value(),
        TapeValue::Value(value) => value.clone(),
        TapeValue::Collection(value, _) => value.clone().unwrap(),
    };

    let object_key = object_key.take();

    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Value(value),
    });
}

#[inline(always)]
#[cold]
fn append_ctor_value<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    object: &Object<'js>,
    ctor: &Constructor<'js>,
    parent: usize,
    object_key: Option<String>,
    array_index: Option<usize>,
) -> Result<()> {
    let clone: Value = ctor.construct((object.clone(),))?;
    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Value(clone),
    });
    Ok(())
}

#[cfg(test)]
mod tests {

    use rquickjs::{function::Opt, Object, Value};

    use crate::{test_utils::utils::with_js_runtime, utils::clone::structured_clone};

    #[tokio::test]
    async fn clone() {
        with_js_runtime(|ctx| {
            crate::modules::buffer::init(&ctx)?;
            let value: Object = ctx.eval(
                r#"
const a = {
   "foo":{
      "bar":"baz"
   },
   "foo1":{
      "bar1":"baz1",
      "bar11":"baz11"
   }
};
a
"#,
            )?;

            let cloned = structured_clone(&ctx, value.clone().into_value(), Opt(None))?
                .into_object()
                .unwrap();

            let json = ctx
                .json_stringify(value.clone())?
                .unwrap()
                .to_string()?
                .to_string();

            let clone_json = ctx
                .json_stringify(cloned.clone())?
                .unwrap()
                .to_string()?
                .to_string();

            assert_eq!(json, clone_json);

            assert_ne!(
                value.get::<_, Value>("foo")?,
                cloned.get::<_, Value>("foo")?
            );

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn clone_circular() {
        with_js_runtime(|ctx| {
            let _value: Object = ctx.eval(
                r#"
const originalObject = { foo: { bar: "baz",arr: [1,2,3] }  };
originalObject.foo.circularRef = originalObject;
originalObject.foo.circularRef2 = originalObject;
originalObject.foo.circularRef3 = originalObject.foo;
originalObject.ref2 = originalObject;
"#,
            )?;

            Ok(())
        })
        .await
    }
}
