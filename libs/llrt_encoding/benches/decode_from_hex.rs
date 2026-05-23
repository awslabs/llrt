// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use criterion::{criterion_group, criterion_main, Criterion};
use llrt_encoding::*;
use rquickjs::{Context, Runtime};

fn criterion_benchmark(c: &mut Criterion) {
    let seed = "48656c6c6f20776f726c6420746869732069732061206c6f6e676572206865782073747248656c6c6f20776f726c6420746869732069732061206c6f6e6765722068657820737472";

    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(512 * 1024);

    let ctx = Context::full(&runtime).unwrap();

    let _ = ctx.with(|ctx| ctx.eval::<String, &[u8]>(b"import {decodeFromHex} from 'llrt:codec';"));

    c.bench_function("[JS] Buffer::from(bytes,'hex')", |b| {
        let func = ["Buffer.from('", seed, "','hex');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[JS] Uint8Array.fromHex()", |b| {
        let func = ["Uint8Array.fromHex('", seed, "');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[JS] llrt:codec.decodeFromHex()", |b| {
        let func = ["decodeFromHex('", seed, "');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[Native] llrt_encoding::bytes_from_hex()", |b| {
        b.iter(|| bytes_from_hex(seed.as_bytes()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
