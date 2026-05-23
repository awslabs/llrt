// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use criterion::{criterion_group, criterion_main, Criterion};
use llrt_encoding::*;

fn criterion_benchmark(c: &mut Criterion) {
    let hello_string = "hello";
    let hello_base64 = "aGVsbG8=";
    let hello_hex = "68656c6c6f";

    c.bench_function("bytes_from_hex", |b| {
        b.iter(|| bytes_from_hex(hello_hex.as_bytes()))
    });

    c.bench_function("bytes_to_hex", |b| {
        b.iter(|| bytes_to_hex(hello_string.as_bytes()))
    });

    c.bench_function("bytes_from_b64", |b| {
        b.iter(|| bytes_from_b64(hello_base64.as_bytes()))
    });

    c.bench_function("bytes_to_b64", |b| {
        b.iter(|| bytes_to_b64(hello_string.as_bytes()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
