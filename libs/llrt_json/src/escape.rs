// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

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

#[inline(always)]
fn write_surrogate_escape(result: &mut String, bytes: &[u8], i: usize) -> usize {
    let code_point = ((bytes[i] as u16 & 0x0F) << 12)
        | ((bytes[i + 1] as u16 & 0x3F) << 6)
        | (bytes[i + 2] as u16 & 0x3F);

    result.push_str("\\u");
    let hex = [
        (code_point >> 12) as u8,
        ((code_point >> 8) & 0xF) as u8,
        ((code_point >> 4) & 0xF) as u8,
        (code_point & 0xF) as u8,
    ];
    for h in hex {
        result.push(if h < 10 {
            (b'0' + h) as char
        } else {
            (b'a' + h - 10) as char
        });
    }
    3
}

#[allow(dead_code)]
pub fn escape_json(bytes: &[u8]) -> String {
    let mut result = String::new();
    escape_json_string(&mut result, bytes);
    result
}

#[inline(always)]
fn process_byte(
    result: &mut String,
    bytes: &[u8],
    byte: u8,
    i: &mut usize,
    start: &mut usize,
    len: usize,
) {
    if *i + 2 < len && byte == 0xED && *i + 1 < len && (bytes[*i + 1] & 0xF0) >= 0xA0 {
        if *start < *i {
            result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[*start..*i]) });
        }
        *i += write_surrogate_escape(result, bytes, *i);
        *start = *i;
        return;
    }

    let c = JSON_ESCAPE_CHARS[byte as usize] as usize;
    if c < ESCAPE_LEN {
        if *start < *i {
            result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[*start..*i]) });
        }
        result.push_str(JSON_ESCAPE_QUOTES[c]);
        *start = *i + 1;
    }
    *i += 1;
}

#[inline(always)]
pub fn escape_json_string_simple(result: &mut String, bytes: &[u8]) {
    let len = bytes.len();
    let mut start = 0;
    let mut i = 0;
    result.reserve(len);

    let (chunks, tail) = bytes.as_chunks::<8>();

    for chunk in chunks {
        if chunk.iter().any(|&b| b < 32 || b == 34 || b == 92) {
            for &byte in chunk {
                process_byte(result, bytes, byte, &mut i, &mut start, len);
            }
        } else {
            i += 8;
        }
    }

    for &byte in tail {
        process_byte(result, bytes, byte, &mut i, &mut start, len);
    }

    if start < len {
        result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..len]) });
    }
}

pub fn escape_json_string(result: &mut String, bytes: &[u8]) {
    escape_json_string_simple(result, bytes);
}

#[cfg(test)]
mod tests {
    use crate::escape::escape_json;

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
