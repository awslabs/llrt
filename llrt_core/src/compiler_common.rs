// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
// Shared with build.rs and compiler.rs

pub struct DummyLoader;

impl rquickjs::loader::Loader for DummyLoader {
    fn load<'js>(
        &mut self,
        ctx: &rquickjs::Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        rquickjs::module::Module::declare(ctx.clone(), name, "")
    }
}

pub struct DummyResolver;

impl rquickjs::loader::Resolver for DummyResolver {
    fn resolve(
        &mut self,
        _ctx: &rquickjs::Ctx<'_>,
        _base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        Ok(name.into())
    }
}

pub fn human_file_size(size: usize) -> String {
    const UNITS: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
    let fsize = size as f64;
    let i = if size == 0 {
        0
    } else {
        (fsize.log2() / 1024f64.log2()).floor() as i32
    };
    let size = fsize / 1024f64.powi(i);

    let mut result = String::with_capacity(16);

    // Convert float to integer with 3 decimal places
    let scaled = (size * 1000.0).round() as i64;
    let integral = scaled / 1000;
    let fractional = scaled % 1000;

    // Custom integer to string conversion
    fn int_to_string(mut n: i64, buf: &mut String) {
        if n == 0 {
            buf.push('0');
            return;
        }
        let mut digits = [0u8; 20];
        let mut i = 0;
        while n > 0 {
            digits[i] = (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        for &digit in digits[..i].iter().rev() {
            buf.push((digit + b'0') as char);
        }
    }

    // Convert integral part
    int_to_string(integral, &mut result);
    result.push('.');

    let len_before = result.len();

    // Convert fractional part with zero-padding
    int_to_string(fractional, &mut result);
    for _ in (result.len() - len_before)..3 {
        result.push('0');
    }

    result.push(' ');
    result.push_str(UNITS[i as usize]);
    result
}
