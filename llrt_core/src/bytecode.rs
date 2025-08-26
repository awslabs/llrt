// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

pub const BYTECODE_VERSION: &str = "lrt01";
pub const BYTECODE_COMPRESSED: u8 = b'c';
pub const BYTECODE_UNCOMPRESSED: u8 = b'u';
pub const BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER: &[u8] = "lrtx".as_bytes();

macro_rules! define_extension {
    ($base:ident, $file:ident, $ext:expr) => {
        #[allow(dead_code)]
        pub const $base: &str = $ext;
        #[allow(dead_code)]
        pub const $file: &str = concat!(".", $ext);
    };
}

define_extension!(BYTECODE_EXT, BYTECODE_FILE_EXT, "lrt");
pub const SIGNATURE_LENGTH: usize = BYTECODE_VERSION.len() + 1;

#[allow(dead_code)]
pub fn add_bytecode_header(bytes: Vec<u8>, file_size: Option<u32>) -> Vec<u8> {
    let mut compressed_bytes = Vec::with_capacity(bytes.len());
    compressed_bytes.extend_from_slice(BYTECODE_VERSION.as_bytes());
    if let Some(file_size) = file_size {
        compressed_bytes.push(BYTECODE_COMPRESSED);
        compressed_bytes.extend_from_slice(&file_size.to_le_bytes());
    } else {
        compressed_bytes.push(BYTECODE_UNCOMPRESSED)
    }
    compressed_bytes.extend_from_slice(&bytes);
    compressed_bytes
}
