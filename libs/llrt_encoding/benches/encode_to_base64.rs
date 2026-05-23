// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use criterion::{criterion_group, criterion_main, Criterion};
use llrt_encoding::*;
use rquickjs::{Context, Runtime};

fn criterion_benchmark(c: &mut Criterion) {
    let seed = "hello world this is a longer buffer example for iteration testing, hello world this is a longer buffer example for iteration testing";

    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(512 * 1024);

    let ctx = Context::full(&runtime).unwrap();

    let _ =
        ctx.with(|ctx| ctx.eval::<String, &[u8]>(b"import {encodeToBase64} from 'llrt:codec';"));

    let func = ["const seed = Buffer.from('", seed, "';"].concat();
    let _ = ctx.with(|ctx| ctx.eval::<String, &[u8]>(func.as_bytes()));

    c.bench_function("[JS] Buffer::from(bytes).toString('base64')", |b| {
        let func = ["Buffer.from('", seed, "').toString('base64');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[JS] Uint8Array.toBase64()", |b| {
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(b"seed.toBase64()")));
    });

    c.bench_function("[JS] llrt:codec.encodeToBase64()", |b| {
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(b"encodeToBase64(seed)")));
    });

    c.bench_function("[Native] llrt_encoding::bytes_to_b64()", |b| {
        b.iter(|| bytes_to_b64(seed.as_bytes()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
