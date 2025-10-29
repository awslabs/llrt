// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

macro_rules! define_extension {
    ($base:ident, $file:ident, $ext:expr) => {
        #[allow(dead_code)]
        pub const $base: &str = $ext;
        #[allow(dead_code)]
        pub const $file: &str = concat!(".", $ext);
    };
}

define_extension!(BYTECODE_EXT, BYTECODE_FILE_EXT, "lrt");

macro_rules! define_supported_extensions {
    // Accepts a list of supported extensions and a single additional constant extension
    ($constant_ext:ident, $($ext:literal),*) => {
        // Define the array of extensions as a constant
        pub const SUPPORTED_EXTENSIONS: &[&str] = &[$($ext),*, $constant_ext];

        pub const JS_EXTENSIONS: &[&str] = &[$($ext),*];

        // Define the function `is_supported_ext` using a match statement
        pub fn is_supported_ext(ext: &str) -> bool {
            matches!(ext, $($ext)|* | $constant_ext)
        }
    };
}

define_supported_extensions!(BYTECODE_FILE_EXT, ".js", ".mjs", ".cjs");
