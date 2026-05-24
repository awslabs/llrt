// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod utils;

use criterion::{criterion_group, criterion_main, Criterion};
use llrt_encoding::*;
use rquickjs::{Context, Runtime};

use utils::random_ascii;

fn criterion_benchmark(c: &mut Criterion) {
    let seed = random_ascii(128);

    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(512 * 1024);

    let ctx = Context::full(&runtime).unwrap();

    let _ = ctx.with(|ctx| ctx.eval::<String, &[u8]>(b"import {encodeToHex} from 'llrt:codec';"));

    let func = ["const seed = Buffer.from('", &seed, "');"].concat();
    let _ = ctx.with(|ctx| ctx.eval::<String, &[u8]>(func.as_bytes()));

    c.bench_function("[JS] Buffer::from(seed).toString('hex')", |b| {
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(b"Buffer.from(seed).toString('hex')")));
    });

    c.bench_function("[JS] Uint8Array.toHex(seed)", |b| {
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(b"seed.toHex()")));
    });

    c.bench_function("[JS] llrt:codec.encodeToHex(seed)", |b| {
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(b"encodeToHex(seed)")));
    });

    c.bench_function("[Native] llrt_encoding::bytes_to_hex(seed)", |b| {
        b.iter(|| bytes_to_hex(seed.as_bytes()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
