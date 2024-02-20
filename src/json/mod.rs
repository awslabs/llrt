// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod escape;
pub mod parse;
pub mod stringify;

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rquickjs::{Array, CatchResultExt, Null, Object, Value};

    use crate::{
        json::{
            parse::json_parse,
            stringify::{json_stringify, json_stringify_replacer_space},
        },
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
        with_runtime(|ctx| {

            let big_int_value = json_parse(&ctx, b"99999999999999999999999999999999999999999999999999999999999999999999999999999999999".to_vec())?;

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
            for data in data.into_iter() {
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

                assert_eq!(json_string1, json_string2);
                assert_eq!(json_string2, json_string3);

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
