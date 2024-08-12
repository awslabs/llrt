// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llrt_core::json::{parse::json_parse, stringify::json_stringify};
use rquickjs::{Context, Runtime};

static JSON: &str = r#"{"organization":{"name":"TechCorp","founding_year":2000,"departments":[{"name":"Engineering","head":{"name":"Alice Smith","title":"VP of Engineering","contact":{"email":"alice.smith@techcorp.com","phone":"+1 (555) 123-4567"}},"employees":[{"id":101,"name":"Bob Johnson","position":"Software Engineer","contact":{"email":"bob.johnson@techcorp.com","phone":"+1 (555) 234-5678"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Developing a revolutionary software solution for clients.","start_date":"2023-01-15","end_date":null,"team":[{"id":201,"name":"Sara Davis","role":"UI/UX Designer"},{"id":202,"name":"Charlie Brown","role":"Quality Assurance Engineer"}]},{"project_id":"P002","name":"Project B","status":"Completed","description":"Upgrading existing systems to enhance performance.","start_date":"2022-05-01","end_date":"2022-11-30","team":[{"id":203,"name":"Emily White","role":"Systems Architect"},{"id":204,"name":"James Green","role":"Database Administrator"}]}]},{"id":102,"name":"Carol Williams","position":"Senior Software Engineer","contact":{"email":"carol.williams@techcorp.com","phone":"+1 (555) 345-6789"},"projects":[{"project_id":"P001","name":"Project A","status":"In Progress","description":"Working on the backend development of Project A.","start_date":"2023-01-15","end_date":null,"team":[{"id":205,"name":"Alex Turner","role":"DevOps Engineer"},{"id":206,"name":"Mia Garcia","role":"Software Developer"}]},{"project_id":"P003","name":"Project C","status":"Planning","description":"Researching and planning for a future project.","start_date":null,"end_date":null,"team":[]}]}]},{"name":"Marketing","head":{"name":"David Brown","title":"VP of Marketing","contact":{"email":"david.brown@techcorp.com","phone":"+1 (555) 456-7890"}},"employees":[{"id":201,"name":"Eva Miller","position":"Marketing Specialist","contact":{"email":"eva.miller@techcorp.com","phone":"+1 (555) 567-8901"},"campaigns":[{"campaign_id":"C001","name":"Product Launch","status":"Upcoming","description":"Planning for the launch of a new product line.","start_date":"2023-03-01","end_date":null,"team":[{"id":301,"name":"Oliver Martinez","role":"Graphic Designer"},{"id":302,"name":"Sophie Johnson","role":"Content Writer"}]},{"campaign_id":"C002","name":"Brand Awareness","status":"Ongoing","description":"Executing strategies to increase brand visibility.","start_date":"2022-11-15","end_date":"2023-01-31","team":[{"id":303,"name":"Liam Taylor","role":"Social Media Manager"},{"id":304,"name":"Ava Clark","role":"Marketing Analyst"}]}]}]}]}}"#;

static JSON_MIN: &str = r#"{"glossary":{"title":"example glossary","GlossDiv":{"title":"S","GlossList":{"GlossEntry":{"ID":"SGML","SortAs":"SGML","GlossTerm":"Standard Generalized Markup Language","Acronym":"SGML","Abbrev":"ISO 8879:1986","GlossDef":{"para":"A meta-markup language, used to create markup languages such as DocBook.","GlossSeeAlso":["GML","XML"]},"GlossSee":"markup"}}}}}"#;

// fn generate_json(child_json: &str, size: usize) -> String {
//     let mut json = String::with_capacity(child_json.len() * size);
//     json.push('{');
//     for i in 0..size {
//         json.push_str("\"obj");
//         json.push_str(&i.to_string());
//         json.push_str("\":");
//         json.push_str(child_json);
//         json.push(',');
//     }
//     json.push_str("\"array\":[");
//     for i in 0..size {
//         json.push_str(child_json);
//         if i < size - 1 {
//             json.push(',');
//         }
//     }

//     json.push_str("]}");
//     json
// }

pub fn criterion_benchmark(c: &mut Criterion) {
    // let mut group = c.benchmark_group("Parsing");

    // let json = JSON.to_owned();
    // for (id, json) in [
    //     json.clone(),
    //     generate_json(&json, 1),
    //     generate_json(&json, 10),
    //     generate_json(&json, 100),
    // ]
    // .into_iter()
    // .enumerate()
    // {
    //     let runtime = Runtime::new().unwrap();
    //     runtime.set_max_stack_size(512 * 1024);

    //     let ctx = Context::full(&runtime).unwrap();
    //     group.bench_with_input(BenchmarkId::new("Custom", id), &json, |b, json| {
    //         ctx.with(|ctx| {
    //             b.iter(|| json_parse(&ctx, json.clone()));
    //         });
    //     });
    //     group.bench_with_input(BenchmarkId::new("Built-in", id), &json, |b, json| {
    //         ctx.with(|ctx| {
    //             b.iter(|| ctx.json_parse(json.clone()));
    //         });
    //     });
    // }

    // group.finish();

    c.bench_function("json parse", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.set_max_stack_size(512 * 1024);

        let ctx = Context::full(&runtime).unwrap();

        ctx.with(|ctx| b.iter(|| json_parse(&ctx, black_box(JSON_MIN))));
    });

    c.bench_function("json stringify", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.set_max_stack_size(512 * 1024);

        let ctx = Context::full(&runtime).unwrap();

        ctx.with(|ctx| {
            let obj = json_parse(&ctx, black_box(JSON)).unwrap();

            b.iter(|| json_stringify(&ctx, obj.clone()))
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
