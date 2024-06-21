// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(rust_nightly)] //FIXME remove when std::simd is stable
use std::simd::{
    prelude::{SimdPartialEq, SimdPartialOrd},
    u8x16, Simd,
};

static JSON_ESCAPE_CHARS: [u8; 256] = [
    0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
    17u8, 18u8, 19u8, 20u8, 21u8, 22u8, 23u8, 24u8, 25u8, 26u8, 27u8, 28u8, 29u8, 30u8, 31u8, 34u8,
    34u8, 32u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 33u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
    34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8, 34u8,
];
static JSON_ESCAPE_QUOTES: [&str; 34usize] = [
    "\\u0000", "\\u0001", "\\u0002", "\\u0003", "\\u0004", "\\u0005", "\\u0006", "\\u0007", "\\b",
    "\\t", "\\n", "\\u000b", "\\f", "\\r", "\\u000e", "\\u000f", "\\u0010", "\\u0011", "\\u0012",
    "\\u0013", "\\u0014", "\\u0015", "\\u0016", "\\u0017", "\\u0018", "\\u0019", "\\u001a",
    "\\u001b", "\\u001c", "\\u001d", "\\u001e", "\\u001f", "\\\"", "\\\\",
];

const ESCAPE_LEN: usize = 34;

#[allow(dead_code)]
pub fn escape_json(bytes: &[u8]) -> String {
    let mut result = String::new();
    escape_json_string(&mut result, bytes);
    result
}

#[inline(always)]
pub fn escape_json_string_simple(result: &mut String, bytes: &[u8]) {
    let len = bytes.len();
    let mut start = 0;
    result.reserve(len);

    for (i, byte) in bytes.iter().enumerate() {
        let c = JSON_ESCAPE_CHARS[*byte as usize] as usize;
        if c < ESCAPE_LEN {
            if start < i {
                result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
            }
            result.push_str(JSON_ESCAPE_QUOTES[c]);
            start = i + 1;
        }
    }
    if start < len {
        result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..len]) });
    }
}

#[cfg(not(rust_nightly))]
pub fn escape_json_string(result: &mut String, bytes: &[u8]) {
    escape_json_string_simple(result, bytes);
}

#[cfg(rust_nightly)]
pub fn escape_json_string(result: &mut String, bytes: &[u8]) {
    use std::mem;

    const USIZE_BYTES: usize = mem::size_of::<usize>();

    let len = bytes.len();
    if len < USIZE_BYTES * 2 {
        return escape_json_string_simple(result, bytes);
    }

    const ESCAPE_LEN: usize = 34;
    const BELOW_SPACE: u8 = b' ' - 1;
    const B: u8 = b'"';
    const C: u8 = b'\\';

    let v_below_space: u8x16 = u8x16::splat(BELOW_SPACE);
    let v_b: u8x16 = u8x16::splat(B);
    let v_c: u8x16 = u8x16::splat(C);

    result.reserve(len);

    #[inline(always)]
    fn process_padded_chunk(
        bytes: &[u8],
        result: &mut String,
        v_below_space: u8x16,
        v_b: u8x16,
        v_c: u8x16,
    ) {
        let len = bytes.len();
        if len > 0 {
            let mut padded_bytes = [b'_'; 16]; //can be max 16 *2 offset
            padded_bytes[..len].copy_from_slice(bytes);
            let byte_vector = u8x16::from_slice(&padded_bytes);
            process_chunk(
                &padded_bytes,
                result,
                byte_vector,
                len,
                v_below_space,
                v_b,
                v_c,
            );
        }
    }

    #[inline(always)]
    fn process_chunk(
        chunk: &[u8],
        result: &mut String,
        byte_vector: Simd<u8, 16>,
        max_len: usize,
        v_below_space: u8x16,
        v_b: u8x16,
        v_c: u8x16,
    ) {
        let mut mask = (byte_vector.simd_eq(v_b)
            | byte_vector.simd_eq(v_c)
            | (byte_vector).simd_lt(v_below_space))
        .to_bitmask();

        if mask != 0 {
            let mut cur = mask.trailing_zeros() as usize;
            let mut start = 0;

            while cur < max_len {
                let c = JSON_ESCAPE_CHARS[chunk[cur] as usize] as usize;
                if c < ESCAPE_LEN {
                    if start < cur {
                        result
                            .push_str(unsafe { std::str::from_utf8_unchecked(&chunk[start..cur]) });
                    }
                    result.push_str(JSON_ESCAPE_QUOTES[c]);
                    start = cur + 1;
                }
                mask ^= 1 << cur;
                if mask == 0 {
                    break;
                }
                cur = mask.trailing_zeros() as usize;
            }
            if start < max_len {
                result.push_str(unsafe { std::str::from_utf8_unchecked(&chunk[start..max_len]) });
            }
        } else {
            result.push_str(unsafe { std::str::from_utf8_unchecked(&chunk[..max_len]) });
        }
    }

    fn process(bytes: &[u8], result: &mut String, v_below_space: u8x16, v_b: u8x16, v_c: u8x16) {
        let iter = bytes.chunks_exact(16);

        let rem = iter.remainder();

        for chunk in iter {
            let a = u8x16::from_slice(&chunk);
            process_chunk(chunk, result, a, 16, v_below_space, v_b, v_c);
        }

        process_padded_chunk(rem, result, v_below_space, v_b, v_c);
    }

    if len < 16 {
        process_padded_chunk(bytes, result, v_below_space, v_b, v_c);
        return;
    }

    process(bytes, result, v_below_space, v_b, v_c);
}

#[cfg(test)]
mod tests {
    use crate::json::escape::escape_json;

    #[test]
    fn escape_json_simple() {
        assert_eq!(escape_json(b"Hello, World!"), "Hello, World!");
    }

    #[test]
    fn escape_json_quotes() {
        assert_eq!(escape_json(b"\"quoted\""), "\\\"quoted\\\"");
    }

    #[test]
    fn escape_json_backslash() {
        assert_eq!(escape_json(b"back\\slash"), "back\\\\slash");
    }

    #[test]
    fn escape_json_newline() {
        assert_eq!(escape_json(b"line\nbreak"), "line\\nbreak");
    }

    #[test]
    fn escape_json_tab() {
        assert_eq!(escape_json(b"tab\tcharacter"), "tab\\tcharacter");
    }

    #[test]
    fn escape_json_unicode() {
        assert_eq!(
            escape_json("unicode: \u{1F609}".as_bytes()),
            "unicode: \u{1F609}"
        );
    }

    #[test]
    fn escape_json_special_characters() {
        assert_eq!(
            escape_json(b"!@#$%^&*()_+-=[]{}|;':,.<>?/"),
            "!@#$%^&*()_+-=[]{}|;':,.<>?/"
        );
    }

    #[test]
    fn escape_json_mixed_characters() {
        assert_eq!(
            escape_json(b"123\"\"45678901\"234567"),
            "123\\\"\\\"45678901\\\"234567"
        );
    }
}
