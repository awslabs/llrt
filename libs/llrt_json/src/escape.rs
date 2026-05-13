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

#[cold]
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
    // Do the escape-table check first. Bytes in the "simple" escape set
    // ({<32, 34, 92}) give `c < ESCAPE_LEN` and go through the fast
    // emit path. For those bytes we can entirely skip the 0xED surrogate
    // check (since JSON_ESCAPE_CHARS[0xED] == ESCAPE_LEN, a byte that
    // satisfies `c < ESCAPE_LEN` is guaranteed not to be 0xED).
    let c = JSON_ESCAPE_CHARS[byte as usize] as usize;
    if c < ESCAPE_LEN {
        // SAFETY: `c < ESCAPE_LEN == JSON_ESCAPE_QUOTES.len()` was just
        // checked on the line above, so indexing is in-bounds.
        let esc = unsafe { JSON_ESCAPE_QUOTES.get_unchecked(c) }.as_bytes();
        // SAFETY: the invariant `*start <= *i` is maintained throughout
        // (`start` is only ever set to a position <= the current `i`),
        // and `*i` is always in-bounds for `bytes`.
        let pending = unsafe { bytes.get_unchecked(*start..*i) };
        // Unified write: one reserve + two direct memcpys. When pending is
        // empty the first copy is a 0-byte no-op, leaving the second copy
        // to write the escape verbatim. Branch-free across escape density.
        //
        // SAFETY: `pending` is a slice of the input which is valid
        // UTF-8/WTF-8; `esc` is a 'static ASCII escape sequence.
        unsafe {
            let vec = result.as_mut_vec();
            let total = pending.len() + esc.len();
            vec.reserve(total);
            let cur = vec.len();
            let dst = vec.as_mut_ptr().add(cur);
            std::ptr::copy_nonoverlapping(pending.as_ptr(), dst, pending.len());
            std::ptr::copy_nonoverlapping(esc.as_ptr(), dst.add(pending.len()), esc.len());
            vec.set_len(cur + total);
        }
        *i += 1;
        *start = *i;
        return;
    }

    // Not a simple escape. If it's a WTF-8 lone surrogate (0xED followed
    // by 0xA0..=0xBF 0x80..=0xBF), emit the \uXXXX form. Otherwise this
    // is a pass-through byte (most common case on non-ASCII text).
    if byte == 0xED && *i + 2 < len && (bytes[*i + 1] & 0xF0) >= 0xA0 {
        if *start < *i {
            // SAFETY: same invariant as above — `*start <= *i <= len`.
            result.push_str(unsafe {
                std::str::from_utf8_unchecked(bytes.get_unchecked(*start..*i))
            });
        }
        *i += write_surrogate_escape(result, bytes, *i);
        *start = *i;
        return;
    }
    *i += 1;
}

#[inline(always)]
fn chunk_escape_mask(chunk: &[u8; 8]) -> u64 {
    // Byte-parallel (SWAR) escape-byte detector. For each byte matching
    // `< 32 || == 34 || == 92 || == 0xED`, the high bit of that byte in
    // the returned u64 is set. Other bits are garbage.
    //
    // `from_le_bytes` makes byte 0 of the chunk map to the low 8 bits of
    // the u64 regardless of platform endianness, so callers can use
    // `trailing_zeros() / 8` to recover the byte index.
    //
    // Each sub-check's pre-`& HIGH` value has meaningful bits only in the
    // high bit of each byte (the low bits are garbage). Since we only ever
    // inspect the high bits, we can OR the sub-checks first and apply
    // `& HIGH` once at the end.
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let x = u64::from_le_bytes(*chunk);
    let lt32 = x.wrapping_sub(0x20 * ONES) & !x;
    let eq34 = {
        let y = x ^ (0x22 * ONES);
        y.wrapping_sub(ONES) & !y
    };
    let eq92 = {
        let y = x ^ (0x5C * ONES);
        y.wrapping_sub(ONES) & !y
    };
    let eqed = {
        let y = x ^ (0xED * ONES);
        y.wrapping_sub(ONES) & !y
    };
    (lt32 | eq34 | eq92 | eqed) & HIGH
}

/// Append a JSON-escaped form of `bytes` to `result`.
///
/// Accepts valid UTF-8 or WTF-8 input (the latter is the encoding QuickJS
/// uses for JS strings that contain lone surrogates).
///
/// Algorithm:
///   1. A 64-byte outer stride. Each iteration runs **eight independent
///      8-byte SWAR checks** (`chunk_escape_mask`) so the CPU can issue
///      all eight dependency chains in parallel and loop overhead is
///      amortized across 64 bytes of clean input.
///   2. If all eight masks are zero, advance `i` by 64 and continue —
///      nothing is copied until either an escape or the final flush.
///   3. Dirty halves are handled by `process_dirty_half`, which walks the
///      mask bits via `trailing_zeros` to jump directly to each byte
///      needing an escape, skipping the clean bytes between them.
///   4. The 0..=63-byte tail is swept with the same 8-byte SWAR in 8-byte
///      increments, then the final 0..=7 bytes fall through to the plain
///      byte-by-byte `process_byte`.
///
/// The SWAR `chunk_escape_mask` may report **false positives** (byte 32
/// preceded by byte <32, byte 35 after byte 34, etc. — see the `hasless`
/// formula's borrow-propagation caveat). `process_byte` absorbs those
/// safely via its `c < ESCAPE_LEN` guard and they cost only a table
/// lookup each.
#[inline(always)]
pub fn escape_json_string_simple(result: &mut String, bytes: &[u8]) {
    let len = bytes.len();
    let mut start = 0;
    let mut i = 0;
    // Reserve output capacity. Headroom depends on input size because
    // escape density can expand output dramatically for small inputs
    // (e.g., an 18-byte all-control-character string becomes 108 bytes of
    // \uXXXX escapes). For large inputs escape density is typically <<25%,
    // so a smaller headroom keeps waste bounded. The reserve is a no-op
    // when `result` already has sufficient capacity (common stringify-
    // accumulator case).
    let headroom = if len < 128 {
        // Small string: reserve up to 6x (covers the worst case of
        // all-control-chars becoming \uXXXX).
        len * 5 + 16
    } else {
        // Larger string: escape density is bounded in practice.
        len / 4 + 16
    };
    result.reserve(len + headroom);

    // Scan in 64-byte strides running eight independent 8-byte SWARs per
    // iteration: the CPU issues many dependency chains in parallel, and
    // loop overhead is amortized across 64 bytes of clean input. 128-byte
    // stride is marginally faster but doubles the unrolled code size, so
    // 64 is the simplicity/perf sweet spot.
    //
    // `process_byte` may advance `i` by 3 on a WTF-8 surrogate escape;
    // that puts it at most 2 bytes into the next 8-byte region, never
    // beyond it, so iteration stays in lockstep with `i`.
    let (chunks64, tail) = bytes.as_chunks::<64>();

    let mut base = 0usize;
    for chunk64 in chunks64 {
        // Manually unrolled: LLVM doesn't reliably fold const offsets when
        // iterating over a fixed-size array here, even with #[inline(always)]
        // on the callee. Keeping 8 independent dependency chains visible.
        macro_rules! mask_at {
            ($off:expr) => {
                chunk_escape_mask((&chunk64[$off..$off + 8]).try_into().unwrap())
            };
        }
        let m_0 = mask_at!(0);
        let m_1 = mask_at!(8);
        let m_2 = mask_at!(16);
        let m_3 = mask_at!(24);
        let m_4 = mask_at!(32);
        let m_5 = mask_at!(40);
        let m_6 = mask_at!(48);
        let m_7 = mask_at!(56);
        if (m_0 | m_1 | m_2 | m_3 | m_4 | m_5 | m_6 | m_7) == 0 {
            i = base + 64;
        } else {
            macro_rules! dispatch {
                ($off:expr, $mask:expr) => {
                    process_dirty_half(result, bytes, base + $off, $mask, &mut i, &mut start, len)
                };
            }
            dispatch!(0, m_0);
            dispatch!(8, m_1);
            dispatch!(16, m_2);
            dispatch!(24, m_3);
            dispatch!(32, m_4);
            dispatch!(40, m_5);
            dispatch!(48, m_6);
            dispatch!(56, m_7);
        }
        base += 64;
    }

    // Tail is 0..=63 bytes. Sweep any remaining 8-byte sub-chunks via
    // SWAR, then fall through to byte-by-byte for the final 0..=7.
    let (sub_chunks, _sub_tail) = tail.as_chunks::<8>();
    for (k, sub) in sub_chunks.iter().enumerate() {
        let mask = chunk_escape_mask(sub);
        process_dirty_half(result, bytes, base + k * 8, mask, &mut i, &mut start, len);
    }

    while i < len {
        process_byte(result, bytes, bytes[i], &mut i, &mut start, len);
    }

    if start < len {
        result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..len]) });
    }
}

#[inline(always)]
fn process_dirty_half(
    result: &mut String,
    bytes: &[u8],
    half_start: usize,
    mask: u64,
    i: &mut usize,
    start: &mut usize,
    len: usize,
) {
    let half_end = half_start + 8;
    if mask == 0 {
        *i = (*i).max(half_end);
        return;
    }
    // Invariant at entry: `*i` is in `[half_start, half_start + 2]` (up to
    // 2 bytes consumed by a surrogate straddling the previous half). Drop
    // mask bits for those already-consumed positions.
    let mut m = mask & (!0u64 << ((*i - half_start) * 8));
    // Fast path: exactly one dirty byte. Skip the mask-clearing shuffle.
    if m.count_ones() == 1 {
        *i = half_start + (m.trailing_zeros() as usize) / 8;
        process_byte(result, bytes, bytes[*i], i, start, len);
        *i = (*i).max(half_end);
        return;
    }
    while m != 0 {
        *i = half_start + (m.trailing_zeros() as usize) / 8;
        process_byte(result, bytes, bytes[*i], i, start, len);
        let consumed = *i - half_start;
        // Branch-free overshift-safe shift: `checked_shl` returns None when
        // consumed >= 8 (shift >= 64), which `.unwrap_or(0)` turns into a
        // zero mask that ends the loop cleanly on the next iteration.
        m &= (!0u64).checked_shl((consumed as u32) * 8).unwrap_or(0);
    }
    *i = (*i).max(half_end);
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

    // WTF-8 lone surrogate sequences — emitted by QuickJS when a String contains
    // lone surrogate code points (e.g. from JSON.stringify("\uD800")). These must
    // be escaped as `\uXXXX` even though they're not valid UTF-8.
    #[test]
    fn escape_json_lone_surrogate() {
        // U+D800 in WTF-8 is 0xED 0xA0 0x80.
        assert_eq!(escape_json(&[0xED, 0xA0, 0x80]), "\\ud800");
    }

    #[test]
    fn escape_json_lone_surrogate_with_context() {
        // Make sure surrogates at different alignments (within, across chunk
        // boundaries) are handled correctly.
        let mut input = b"abcdefg".to_vec(); // 7 bytes before surrogate
        input.extend_from_slice(&[0xED, 0xBF, 0xBF]); // U+DFFF
        input.extend_from_slice(b"xyz");
        assert_eq!(escape_json(&input), "abcdefg\\udfffxyz");
    }

    #[test]
    fn escape_json_surrogate_at_chunk_boundary() {
        // Surrogate starts at byte index 6, spans past the 8-byte chunk boundary.
        let mut input = b"abcdef".to_vec(); // 6 bytes
        input.extend_from_slice(&[0xED, 0xA0, 0x80]); // U+D800, ends at index 9
        input.extend_from_slice(b"xyz123456789");
        let expected = "abcdef\\ud800xyz123456789";
        assert_eq!(escape_json(&input), expected);
    }

    #[test]
    fn escape_json_korean_passthrough() {
        // Valid Korean Hangul (U+D6C8 "훈") is encoded 0xED 0x9B 0x88 — the
        // second byte has high nibble 0x90 < 0xA0 so it must NOT be escaped.
        let s = "훈훈훈";
        assert_eq!(escape_json(s.as_bytes()), s);
    }
}
