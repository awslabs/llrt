// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use criterion::{criterion_group, criterion_main, Criterion};
use llrt_numbers::i64_to_base_n;
use rand::Rng;
use std::{fmt::Write, hint::black_box};

macro_rules! write_formatted {
    ($format:expr, $number:expr) => {{
        let digits = ($number as f64).log10() as usize + 2;
        let mut string = String::with_capacity(digits);
        write!(string, $format, $number).unwrap();
        string
    }};
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    c.bench_function("to_binary", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            i64_to_base_n(black_box(num), 2);
        })
    });

    c.bench_function("to_octal", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            i64_to_base_n(black_box(num), 2);
        })
    });

    c.bench_function("to_dec", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            i64_to_base_n(black_box(num), 10);
        })
    });

    c.bench_function("to_hex", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            i64_to_base_n(black_box(num), 16);
        })
    });

    c.bench_function("write_formatted bin", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            write_formatted!("{:b}", black_box(num));
        })
    });

    c.bench_function("write_formatted octal", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            write_formatted!("{:o}", black_box(num));
        })
    });

    c.bench_function("write_formatted dec", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            write_formatted!("{}", black_box(num));
        })
    });

    c.bench_function("write_formatted hex", |b| {
        b.iter(|| {
            let num: i64 = rng.gen();
            write_formatted!("{:x}", black_box(num));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
