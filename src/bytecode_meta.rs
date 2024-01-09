pub const BYTECODE_VERSION: &str = "lrt01";
pub const BYTECODE_COMPRESSED: u8 = b'c';
pub const BYTECODE_UNCOMPRESSED: u8 = b'u';
#[allow(dead_code)]
pub const BYTECODE_EXT: &str = "lrt";
pub const SIGNATURE_LENGTH: usize = BYTECODE_VERSION.len() + 1;
