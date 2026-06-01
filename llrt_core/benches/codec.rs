// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Goal: prove that QuickJS's built-in Uint8Array.{to,from}{Base64,Hex} is as
// fast as our Rust `llrt:codec` impl across input sizes, so the Rust impl can
// be dropped. Each group sweeps sizes and compares, per size:
//   builtin  QuickJS native (Uint8Array.toBase64 / fromBase64 / toHex / fromHex)
//   codec    llrt:codec  (current Rust impl)
//   native   raw llrt_encoding fn (lower bound, FFI excluded)
//
// Each op is compiled once into a closure `() => <expr>.length` that captures
// the seed, so the result is consumed and the call Result is unwrapped (a JS
// exception fails loudly, avoiding "constant time regardless of size" artifacts).

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use llrt_encoding::*;
use rand::RngExt;
use rquickjs::{
    loader::{BuiltinResolver, ModuleLoader},
    CatchResultExt, Context, Function, Module, Runtime,
};

use llrt_core::modules::llrt::codec::LlrtCodecModule;

const SIZES: [usize; 4] = [64, 1024, 16384, 262144];
const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_bytes(len: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..len)
        .map(|_| ASCII[rng.random_range(0..ASCII.len())])
        .collect()
}

fn new_ctx() -> Context {
    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(512 * 1024);
    let resolver = BuiltinResolver::default().with_module("llrt:codec");
    let loader = ModuleLoader::default().with_module("llrt:codec", LlrtCodecModule);
    runtime.set_loader(resolver, loader);
    let ctx = Context::full(&runtime).unwrap();
    // install the `Buffer` global so the `master` path (Buffer.from) can be benched
    ctx.with(|ctx| llrt_modules::buffer::init(&ctx).unwrap());
    ctx
}

/// `seed_js` builds the JS seed literal. `builtin`/`codec` are expressions over
/// `seed` whose `.length` is returned. `native_input` is the slice the native
/// fn converts (already encoded for decode cases) so only the conversion is
/// timed.
#[allow(clippy::too_many_arguments)]
fn bench(
    c: &mut Criterion,
    name: &str,
    seed_js: impl Fn(&[u8]) -> String,
    master: &str,
    builtin: &str,
    codec: &str,
    native_input: impl Fn(&[u8]) -> Vec<u8>,
    native: impl Fn(&[u8]),
) {
    let ctx = new_ctx();
    let mut group = c.benchmark_group(name);
    for &len in &SIZES {
        let bytes = random_bytes(len);
        let ni = native_input(&bytes);
        let src = format!(
            "import {{encodeToBase64,decodeFromBase64,encodeToHex,decodeFromHex}} from 'llrt:codec';\n\
             const seed = {};\n\
             export const master = () => ({}).length;\n\
             export const builtin = () => ({}).length;\n\
             export const codec = () => ({}).length;",
            seed_js(&bytes), master, builtin, codec,
        );
        group.throughput(Throughput::Bytes(len as u64));

        ctx.with(|ctx| {
            let decl = Module::declare(ctx.clone(), "bench", src)
                .catch(&ctx)
                .unwrap();
            let (module, promise) = decl.eval().catch(&ctx).unwrap();
            promise.finish::<()>().catch(&ctx).unwrap();
            let master_fn: Function = module.get("master").unwrap();
            let builtin_fn: Function = module.get("builtin").unwrap();
            let codec_fn: Function = module.get("codec").unwrap();
            group.bench_function(format!("master/{len}"), |b| {
                b.iter(|| master_fn.call::<_, usize>(()).expect("master call failed"))
            });
            group.bench_function(format!("builtin/{len}"), |b| {
                b.iter(|| {
                    builtin_fn
                        .call::<_, usize>(())
                        .expect("builtin call failed")
                })
            });
            group.bench_function(format!("codec/{len}"), |b| {
                b.iter(|| codec_fn.call::<_, usize>(()).expect("codec call failed"))
            });
        });
        group.bench_with_input(format!("native/{len}"), &ni, |b, input| {
            b.iter(|| native(black_box(input)))
        });
    }
    group.finish();
}

fn encode_base64(c: &mut Criterion) {
    bench(
        c,
        "encode_base64",
        |b| format!("new Uint8Array({:?})", b),
        "Buffer.from(seed).toString('base64')",
        "seed.toBase64()",
        "encodeToBase64(seed)",
        |b| b.to_vec(),
        |b| {
            black_box(bytes_to_b64(b));
        },
    );
}

fn decode_base64(c: &mut Criterion) {
    bench(
        c,
        "decode_base64",
        |b| format!("'{}'", bytes_to_b64_string(b)),
        "Buffer.from(seed, 'base64')",
        "Uint8Array.fromBase64(seed)",
        "decodeFromBase64(seed)",
        |b| bytes_to_b64_string(b).into_bytes(),
        |b| {
            black_box(bytes_from_b64_strict(b).unwrap());
        },
    );
}

fn encode_hex(c: &mut Criterion) {
    bench(
        c,
        "encode_hex",
        |b| format!("new Uint8Array({:?})", b),
        "Buffer.from(seed).toString('hex')",
        "seed.toHex()",
        "encodeToHex(seed)",
        |b| b.to_vec(),
        |b| {
            black_box(bytes_to_hex(b));
        },
    );
}

fn decode_hex(c: &mut Criterion) {
    bench(
        c,
        "decode_hex",
        |b| format!("'{}'", bytes_to_hex_string(b)),
        "Buffer.from(seed, 'hex')",
        "Uint8Array.fromHex(seed)",
        "decodeFromHex(seed)",
        |b| bytes_to_hex_string(b).into_bytes(),
        |b| {
            black_box(bytes_from_hex(b).unwrap());
        },
    );
}

criterion_group!(
    benches,
    encode_base64,
    decode_base64,
    encode_hex,
    decode_hex
);
criterion_main!(benches);
