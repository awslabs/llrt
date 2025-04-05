// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(rust_nightly, feature(portable_simd))]
use std::cmp::min;

use rquickjs::{
    atom::PredefinedAtom, function::Opt, prelude::Func, Ctx, IntoJs, Object, Result, Value,
};

pub mod escape;
pub mod parse;
pub mod stringify;

use crate::parse::json_parse_string;
use crate::stringify::json_stringify_replacer_space;

pub fn redefine_static_methods(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let json_module: Object = globals.get(PredefinedAtom::JSON)?;
    json_module.set("parse", Func::from(json_parse_string))?;
    json_module.set(
        "stringify",
        Func::from(|ctx, value, replacer, space| {
            struct StringifyArgs<'js>(Ctx<'js>, Value<'js>, Opt<Value<'js>>, Opt<Value<'js>>);
            let StringifyArgs(ctx, value, replacer, space) =
                StringifyArgs(ctx, value, replacer, space);

            let mut space_value = None;
            let mut replacer_value = None;

            if let Some(replacer) = replacer.0 {
                if let Some(space) = space.0 {
                    if let Some(space) = space.as_string() {
                        let mut space = space.clone().to_string()?;
                        space.truncate(20);
                        space_value = Some(space);
                    }
                    if let Some(number) = space.as_int() {
                        if number > 0 {
                            space_value = Some(" ".repeat(min(10, number as usize)));
                        }
                    }
                }
                replacer_value = Some(replacer);
            }

            json_stringify_replacer_space(&ctx, value, replacer_value, space_value)
                .map(|v| v.into_js(&ctx))?
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use llrt_test::test_sync_with;
    use rquickjs::{prelude::Func, Array, CatchResultExt, IntoJs, Null, Object, Undefined, Value};

    use crate::{
        parse::{json_parse, json_parse_string},
        stringify::{json_stringify, json_stringify_replacer_space},
    };

    static JSON: &str = r#"{"organization":{"name":"TechCorp","founding_year":2000,"departments":[{"name":"Engineering","head":{"name":"Alice Smith","title":"VP of Engineering","contact":{"email":"alice.smith@techcorp.com","phone":"+1 (555) 123-4567"}},"employees":[{"id":101,"name":"Bob Johnson","position":"Software Engineer","contact":{"email":"bob.johnson@techcorp.com","phone":"+1 (555) 234-5678"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Developing a revolutionary software solution for clients.","start_date":"2023-01-15","end_date":null,"team":[{"id":201,"name":"Sara Davis","role":"UI/UX Designer"},{"id":202,"name":"Charlie Brown","role":"Quality Assurance Engineer"}]},{"project_id":"P002","name":"Project B","status":"Completed","description":"Upgrading existing systems to enhance performance.","start_date":"2022-05-01","end_date":"2022-11-30","team":[{"id":203,"name":"Emily White","role":"Systems Architect"},{"id":204,"name":"James Green","role":"Database Administrator"}]}]},{"id":102,"name":"Carol Williams","position":"Senior Software Engineer","contact":{"email":"carol.williams@techcorp.com","phone":"+1 (555) 345-6789"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Working on the backend development of Project A.","start_date":"2023-01-15","end_date":null,"team":[{"id":205,"name":"Alex Turner","role":"DevOps Engineer"},{"id":206,"name":"Mia Garcia","role":"Software Developer"}]},{"project_id":"P003","name":"Project C","status":"Planning","description":"Researching and planning for a future project.","start_date":null,"end_date":null,"team":[]}]}]},{"name":"Marketing","head":{"name":"David Brown","title":"VP of Marketing","contact":{"email":"david.brown@techcorp.com","phone":"+1 (555) 456-7890"}},"employees":[{"id":201,"name":"Eva Miller","position":"Marketing Specialist","contact":{"email":"eva.miller@techcorp.com","phone":"+1 (555) 567-8901"},"campaigns":[{"campaign_id":"C001","name":"Product Launch","status":"Upcoming","description":"Planning for the launch of a new product line.","start_date":"2023-03-01","end_date":null,"team":[{"id":301,"name":"Oliver Martinez","role":"Graphic Designer"},{"id":302,"name":"Sophie Johnson","role":"Content Writer"}]},{"campaign_id":"C002","name":"Brand Awareness","status":"Ongoing","description":"Executing strategies to increase brand visibility.","start_date":"2022-11-15","end_date":"2023-01-31","team":[{"id":303,"name":"Liam Taylor","role":"Social Media Manager"},{"id":304,"name":"Ava Clark","role":"Marketing Analyst"}]}]}]}]}}"#;

    #[tokio::test]
    async fn json_parser() {
        test_sync_with(|ctx| {
            let json_data = [
                r#"{"aa\"\"aaaaaaaaaaaaaaaa":"a","b":"bbb"}"#,
                r#"{"a":"aaaaaaaaaaaaaaaaaa","b":"bbb"}"#,
                r#"{"a":["a","a","aaaa","a"],"b":"b"}"#,
                r#"{"type":"Buffer","data":[10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10,10]}"#,
                r#"{"a":[{"object2":{"key1":"value1","key2":123,"key3":false,"nestedObject":{"nestedKey":"nestedValue"}},"string":"Hello, World!","emptyObj":{},"emptyArr":[],"number":42,"boolean":true,"nullValue":null,"array":[1,2,3,"four",5.5,true,null],"object":{"key1":"value1","key2":123,"key3":false,"nestedObject":{"nestedKey":"nestedValue"}}}]}"#,
                JSON,
            ];

            for json_str in json_data {
                let json = json_str.to_string();
                let json2 = json.clone();

                let value = json_parse(&ctx, json2)?;
                let new_json = json_stringify_replacer_space(&ctx, value.clone(),None,Some("  ".into()))?.unwrap();
                let builtin_json = ctx.json_stringify_replacer_space(value,Null,"  ".to_string())?.unwrap().to_string()?;
                assert_eq!(new_json, builtin_json);
            }

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_parse_non_string() {
        test_sync_with(|ctx| {
            ctx.globals().set("parse", Func::from(json_parse_string))?;

            let result = ctx.eval::<(), _>("parse({})").catch(&ctx);

            if let Err(err) = result {
                assert_eq!(
                   err.to_string(),
                   "Error: \"[object Object]\" not valid JSON at index 1 ('o')\n    at <eval> (eval_script:1:1)\n"
               );
            } else {
                panic!("expected error")
            }

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_stringify_undefined() {
        test_sync_with(|ctx| {
            let stringified = json_stringify(&ctx, Undefined.into_js(&ctx)?)?;
            let stringified_2 = ctx
                .json_stringify(Undefined)?
                .map(|v| v.to_string().unwrap());
            assert_eq!(stringified, stringified_2);

            let obj: Value = ctx.eval(
                r#"let obj = { value: undefined, array: [undefined, null, 1, true, "hello", { [Symbol("sym")]: 1, [undefined]: 2}] };obj;"#,
            )?;

            let stringified = json_stringify(&ctx, obj.clone())?;
            let stringified_2 = ctx
                .json_stringify(obj)?
                .map(|v| v.to_string().unwrap());
            assert_eq!(stringified, stringified_2);

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_stringify_objects() {
        test_sync_with(|ctx| {
            let date: Value = ctx.eval("let obj = { date: new Date(0) };obj;")?;
            let stringified = json_stringify(&ctx, date.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(date)?.unwrap().to_string()?;
            assert_eq!(stringified, stringified_2);
            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn huge_numbers() {
        test_sync_with(|ctx| {

            let big_int_value = json_parse(&ctx, b"99999999999999999999999999999999999999999999999999999999999999999999999999999999999")?;

            let stringified = json_stringify(&ctx, big_int_value.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(big_int_value)?.unwrap().to_string()?.replace("e+", "e");
            assert_eq!(stringified, stringified_2);

            let big_int_value: Value = ctx.eval("999999999999")?;
            let stringified = json_stringify(&ctx, big_int_value.clone())?.unwrap();
            let stringified_2 = ctx.json_stringify(big_int_value)?.unwrap().to_string()?;
            assert_eq!(stringified, stringified_2);

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn json_circular_ref() {
        test_sync_with(|ctx| {
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
}
