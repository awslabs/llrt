// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashSet;

use llrt_modules::exceptions::DOMException;
use llrt_utils::{
    clone::clone_platform_object,
    object::ObjectExt,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt, This},
    Array, ArrayBuffer, Ctx, Exception, Function, IntoJs, Null, Object, Result, Type, Value,
};

use crate::modules::{
    buffer::{Blob, File},
    crypto::CryptoKey,
};

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
    let primordials = BasePrimordials::get(ctx)?;
    let mut transfer_set = None;

    if let Some(options) = options.0 {
        if let Some(transfer_array) = options.get_optional::<_, Array>("transfer")? {
            let mut set = HashSet::with_capacity(transfer_array.len());

            for item in transfer_array.iter::<Value>() {
                let item = item?;
                if !set.insert(item.clone()) {
                    return Err(DOMException::data_clone_error(
                        ctx,
                        "Transfer list contains duplicate values",
                    ));
                }
                if ArrayBuffer::from_value(item).is_none() {
                    return Err(DOMException::data_clone_error(
                        ctx,
                        "Value is not transferable",
                    ));
                }
            }
            transfer_set = Some(set);
        }
    }

    let mut tape = Vec::<TapeItem>::with_capacity(10);
    let mut stack = Vec::with_capacity(10);
    let mut visited = Vec::<(Value<'js>, usize)>::with_capacity(10);
    let mut index = 0usize;
    let data_view_ctor = &primordials.constructor_data_view;
    let weak_map_ctor = &primordials.constructor_weak_map;
    let weak_set_ctor = &primordials.constructor_weak_set;

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
                    Type::Proxy => {
                        return Err(Exception::throw_type(
                            ctx,
                            "A Proxy value could not be cloned",
                        ));
                    },
                    Type::Function | Type::Symbol | Type::Promise | Type::Constructor => {
                        return Err(DOMException::data_clone_error(
                            ctx,
                            "Value could not be cloned",
                        ));
                    },
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

                        let is_weak_collection = object.is_instance_of(weak_map_ctor)
                            || object.is_instance_of(weak_set_ctor);
                        if is_weak_collection {
                            return Err(DOMException::data_clone_error(
                                ctx,
                                "Value could not be cloned",
                            ));
                        }

                        if let Some(value) = try_clone_platform_object(ctx, object)? {
                            tape.push(TapeItem {
                                parent,
                                object_key,
                                array_index,
                                value: TapeValue::Value(value),
                            });
                            index += 1;
                            continue;
                        }

                        if object.is_instance_of(&primordials.constructor_date) {
                            append_ctor_value(
                                &mut tape,
                                object,
                                &primordials.constructor_date,
                                parent,
                                object_key,
                                array_index,
                            )?;
                            index += 1;
                            continue;
                        }

                        if object.is_instance_of(&primordials.constructor_regexp) {
                            append_ctor_value(
                                &mut tape,
                                object,
                                &primordials.constructor_regexp,
                                parent,
                                object_key,
                                array_index,
                            )?;
                            index += 1;
                            continue;
                        }

                        if object.is_instance_of(&primordials.constructor_number)
                            || object.is_instance_of(&primordials.constructor_string)
                            || object.is_instance_of(&primordials.constructor_bool)
                        {
                            let value_of: Function = object.get("valueOf")?;
                            let primitive: Value = value_of.call((This(object.clone()),))?;
                            let clone: Value =
                                primordials.constructor_object.construct((primitive,))?;
                            tape.push(TapeItem {
                                parent,
                                object_key,
                                array_index,
                                value: TapeValue::Value(clone),
                            });
                            index += 1;
                            continue;
                        }

                        let is_collection = if object.is_instance_of(&primordials.constructor_set) {
                            Some(ObjectType::Set)
                        } else if object.is_instance_of(&primordials.constructor_map) {
                            Some(ObjectType::Map)
                        } else {
                            None
                        };

                        if let Some(collection_type) = is_collection {
                            append_collection(
                                &mut tape,
                                &primordials.function_array_from,
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

                        if primordials
                            .function_array_buffer_is_view
                            .call::<_, bool>((value.clone(),))?
                        {
                            let is_data_view = object.is_instance_of(data_view_ctor);
                            append_buffer(
                                &mut tape,
                                object,
                                parent,
                                object_key,
                                array_index,
                                is_data_view,
                            )?;
                            index += 1;
                            continue;
                        }

                        if let Some(buffer) = ArrayBuffer::from_value(value.clone()) {
                            let slice: Function = buffer.get("slice")?;
                            let clone: Value = slice.call((This(buffer),))?;
                            tape.push(TapeItem {
                                parent,
                                object_key,
                                array_index,
                                value: TapeValue::Value(clone),
                            });
                            index += 1;
                            continue;
                        }

                        let new: Object<'_> =
                            if object.is_instance_of(&primordials.constructor_error) {
                                primordials.constructor_error.construct(("",))
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
                        //unsafe OK since we're guaranteed to be object by the match
                        let array = unsafe { value.as_array().unwrap_unchecked() };
                        let length = array.len();
                        let keys = array
                            .as_object()
                            .keys::<String>()
                            .collect::<Result<Vec<_>>>()?;
                        let new = Array::new(ctx.clone())?;
                        new.as_object().set(PredefinedAtom::Length, length)?;
                        tape.push(TapeItem {
                            parent,
                            object_key,
                            array_index,
                            value: TapeValue::Array(new),
                        });
                        stack.push(StackItem::ObjectEnd);

                        for key in keys.into_iter().rev() {
                            let item = array.as_object().get(&key)?;
                            stack.push(StackItem::Value(index, item, Some(key), None));
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
                if let Some(key) = object_key {
                    array.as_object().set(key, value)?;
                } else {
                    array.set(array_index.unwrap(), value)?;
                }
            },
            TapeValue::Object(object) => {
                let string = object_key.unwrap();
                object.set(string, value)?;
            },
            TapeValue::Collection(collection_value, collection_type) => {
                match collection_type {
                    ObjectType::Set => {
                        collection_value.replace(primordials.constructor_set.construct((value,))?);
                    },
                    ObjectType::Map => {
                        collection_value.replace(primordials.constructor_map.construct((value,))?);
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
    is_data_view: bool,
) -> Result<()> {
    let ctor: Constructor = object.get(PredefinedAtom::Constructor)?;
    // Views are cloned over a copy of their whole backing buffer, retaining
    // the original offset and length per the HTML spec.
    let buffer: Object = object.get("buffer")?;
    let byte_offset: usize = object.get("byteOffset")?;
    let view_len: usize = if is_data_view {
        object.get("byteLength")?
    } else {
        object.get("length")?
    };
    let slice: Function = buffer.get("slice")?;
    let cloned_buffer: Value = slice.call((This(buffer),))?;
    let new = ctor.construct((cloned_buffer, byte_offset, view_len))?;
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
fn check_circular<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    visited: &mut Vec<(Value<'js>, usize)>,
    value: &Value<'js>,
    parent: usize,
    object_key: &mut Option<String>,
    array_index: Option<usize>,
    index: usize,
) -> bool {
    if let Some(visited) = visited.iter().find(|v| &v.0 == value) {
        append_circular(tape, visited, object_key, parent, array_index);
        return true;
    }
    visited.push((value.clone(), index));
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
    let value = if let Some(ab) = ArrayBuffer::from_value(value.clone()) {
        ab.get::<_, Function>("transfer")?.call((This(ab),))?
    } else {
        value.clone()
    };

    tape.push(TapeItem {
        parent,
        object_key,
        array_index,
        value: TapeValue::Value(value),
    });
    Ok(())
}

#[inline(always)]
#[cold]
fn append_circular<'js>(
    tape: &mut Vec<TapeItem<'js>>,
    visited: &(Value<'js>, usize),
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

fn try_clone_platform_object<'js>(
    ctx: &Ctx<'js>,
    object: &Object<'js>,
) -> Result<Option<Value<'js>>> {
    if let Some(value) = clone_platform_object::<File>(ctx, object)? {
        return Ok(Some(value));
    }
    if let Some(value) = clone_platform_object::<Blob>(ctx, object)? {
        return Ok(Some(value));
    }
    if let Some(value) = clone_platform_object::<CryptoKey>(ctx, object)? {
        return Ok(Some(value));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {

    use llrt_test::test_sync_with;
    use rquickjs::{function::Opt, Object, Value};

    use llrt_utils::primordials::{BasePrimordials, Primordial};

    use super::structured_clone;

    #[tokio::test]
    async fn clone() {
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
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
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
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
