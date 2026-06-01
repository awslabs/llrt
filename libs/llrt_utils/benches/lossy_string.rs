// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Benchmark for `replace_invalid_utf8_and_utf16` (kernel) and
// `get_lossy_string` (full path through QuickJS FFI). Reports
// `METRIC <case>_us=<best>` lines and `METRIC total_us=<sum>`.

#![allow(clippy::uninlined_format_args)]

use std::hint::black_box;
use std::time::Instant;

use llrt_utils::bytes::{get_lossy_string, replace_invalid_utf8_and_utf16};
use rquickjs::{Context, Function, Runtime, Value};

fn make_ascii(len: usize) -> Vec<u8> {
    let pat = b"the quick brown fox jumps over the lazy dog ";
    let mut v = Vec::with_capacity(len);
    while v.len() < len {
        let take = (len - v.len()).min(pat.len());
        v.extend_from_slice(&pat[..take]);
    }
    v
}

fn make_utf8_mixed(len: usize) -> Vec<u8> {
    // Mix of ASCII, 2-byte (Latin-1 supplement), 3-byte (CJK), 4-byte (emoji).
    let pat = "Hello, 世界! Café — 🦀 emoji + ñ test ";
    let bytes = pat.as_bytes();
    let mut v = Vec::with_capacity(len);
    while v.len() + bytes.len() < len {
        v.extend_from_slice(bytes);
    }
    v
}

fn make_with_lone_surrogate(len: usize) -> Vec<u8> {
    // U+D83D in WTF-8: ED A0 BD. QuickJS emits this for `'\uD83D'`.
    let mut v = make_ascii(len.saturating_sub(3));
    v.extend_from_slice(&[0xED, 0xA0, 0xBD]);
    v
}

fn make_short_url() -> Vec<u8> {
    b"https://example.com/path?key=value&foo=bar".to_vec()
}

fn time_us<F: FnMut()>(iters: u32, mut f: F) -> u64 {
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    start.elapsed().as_micros() as u64
}

fn run_kernel_case(name: &'static str, input: &[u8], iters: u32) -> u64 {
    for _ in 0..(iters / 10).max(1) {
        black_box(replace_invalid_utf8_and_utf16(black_box(input)));
    }
    let mut best = u64::MAX;
    for _ in 0..5 {
        let t = time_us(iters, || {
            black_box(replace_invalid_utf8_and_utf16(black_box(input)));
        });
        if t < best {
            best = t;
        }
    }
    println!("METRIC {}_us={}", name, best);
    best
}

/// Build a JS Value from raw bytes. Lone-surrogate inputs are
/// constructed via `String.fromCharCode` since Rust strings can't hold
/// them.
fn make_js_string<'js>(
    ctx: &rquickjs::Ctx<'js>,
    bytes: &[u8],
    has_lone_surrogate: bool,
) -> Value<'js> {
    if has_lone_surrogate {
        // Treat input as: ASCII prefix + lone surrogate U+D83D at end.
        let prefix_len = bytes.len() - 3;
        let prefix: String = String::from_utf8(bytes[..prefix_len].to_vec()).unwrap();
        ctx.globals().set("__bench_prefix", prefix).unwrap();
        let concat: Function = ctx
            .eval(r#"(prefix) => prefix + String.fromCharCode(0xD83D)"#)
            .unwrap();
        let prefix_v: Value = ctx.globals().get("__bench_prefix").unwrap();
        concat.call::<_, Value>((prefix_v,)).unwrap()
    } else {
        let s: String = String::from_utf8(bytes.to_vec()).unwrap();
        ctx.globals().set("__bench_str", s).unwrap();
        ctx.globals().get("__bench_str").unwrap()
    }
}

fn run_e2e_case(
    ctx: &rquickjs::Ctx<'_>,
    name: &'static str,
    bytes: &[u8],
    has_lone_surrogate: bool,
    iters: u32,
) -> u64 {
    let v = make_js_string(ctx, bytes, has_lone_surrogate);
    // Warmup
    for _ in 0..(iters / 10).max(1) {
        black_box(get_lossy_string(black_box(v.clone())).unwrap());
    }
    let mut best = u64::MAX;
    for _ in 0..5 {
        let t = time_us(iters, || {
            black_box(get_lossy_string(black_box(v.clone())).unwrap());
        });
        if t < best {
            best = t;
        }
    }
    println!("METRIC {}_us={}", name, best);
    best
}

fn main() {
    let cases: &[(&str, Vec<u8>, u32, bool)] = &[
        ("ascii_short", make_short_url(), 200_000, false),
        ("ascii_64", make_ascii(64), 200_000, false),
        ("ascii_512", make_ascii(512), 50_000, false),
        ("ascii_4k", make_ascii(4096), 10_000, false),
        ("utf8_mixed_512", make_utf8_mixed(512), 50_000, false),
        ("utf8_mixed_4k", make_utf8_mixed(4096), 10_000, false),
        (
            "lone_surrogate_512",
            make_with_lone_surrogate(512),
            50_000,
            true,
        ),
    ];

    let mut total = 0u64;

    // Kernel-only bench.
    for (name, input, iters, _) in cases {
        let label: &'static str = match *name {
            "ascii_short" => "kernel_ascii_short",
            "ascii_64" => "kernel_ascii_64",
            "ascii_512" => "kernel_ascii_512",
            "ascii_4k" => "kernel_ascii_4k",
            "utf8_mixed_512" => "kernel_utf8_mixed_512",
            "utf8_mixed_4k" => "kernel_utf8_mixed_4k",
            "lone_surrogate_512" => "kernel_lone_surrogate_512",
            _ => "kernel_unknown",
        };
        total += run_kernel_case(label, input, *iters);
    }

    // End-to-end bench through QuickJS FFI.
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        for (name, input, iters, has_surrogate) in cases {
            let label: &'static str = match *name {
                "ascii_short" => "e2e_ascii_short",
                "ascii_64" => "e2e_ascii_64",
                "ascii_512" => "e2e_ascii_512",
                "ascii_4k" => "e2e_ascii_4k",
                "utf8_mixed_512" => "e2e_utf8_mixed_512",
                "utf8_mixed_4k" => "e2e_utf8_mixed_4k",
                "lone_surrogate_512" => "e2e_lone_surrogate_512",
                _ => "e2e_unknown",
            };
            // FFI is more expensive than the kernel; scale iters down.
            let scaled = (*iters / 4).max(1000);
            total += run_e2e_case(&ctx, label, input, *has_surrogate, scaled);
        }
    });

    println!("METRIC total_us={}", total);
}
