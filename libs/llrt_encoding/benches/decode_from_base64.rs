// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use criterion::{criterion_group, criterion_main, Criterion};
use llrt_encoding::*;
use rquickjs::{Context, Runtime};

fn criterion_benchmark(c: &mut Criterion) {
    let seed = "VGhpcyBpcyBhIGxvbmcgYmFzZTY0IHN0cmluZyBleGFtcGxlIHRoYXQgcmVwcmVzZW50cyBhIGxhcmdlciBwYXlsb2FkLiBUaGlzIGlzIHVzZWQgdG8gc2ltdWxhdGUgYSBiaWdnZXIgZGF0YSBibG9iIGluIGEgcmVhbGlzdGljIHNjZW5hcmlvLg==";

    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(512 * 1024);

    let ctx = Context::full(&runtime).unwrap();

    let _ =
        ctx.with(|ctx| ctx.eval::<String, &[u8]>(b"import {decodeFromBase64} from 'llrt:codec';"));

    c.bench_function("[JS] Buffer::from(bytes,'base64')", |b| {
        let func = ["Buffer.from('", seed, "','base64');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[JS] Uint8Array.fromBase64()", |b| {
        let func = ["Uint8Array.fromBase64('", seed, "');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[JS] llrt:codec.decodeFromBase64()", |b| {
        let func = ["decodeFromBase64('", seed, "');"].concat();
        ctx.with(|ctx| b.iter(|| ctx.eval::<String, &[u8]>(func.as_bytes())));
    });

    c.bench_function("[Native] llrt_encoding::bytes_from_b64()", |b| {
        b.iter(|| bytes_from_b64(seed.as_bytes()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
