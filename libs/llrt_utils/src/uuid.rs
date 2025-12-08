pub fn uuid_v4() -> String {
    let uuid = rand::random::<u128>() & 0xFFFFFFFFFFFF4FFFBFFFFFFFFFFFFFFF | 0x40008000000000000000;

    static HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let bytes = uuid.to_be_bytes();

    let mut buf = [0u8; 36];

    // Precomputed positions for 32 hex digits (excluding hyphens)
    static HEX_POS: [usize; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35,
    ];

    // Map each byte to its hex representation
    let mut hex_idx = 0;
    for &byte in &bytes[..] {
        let high = HEX_CHARS[(byte >> 4) as usize];
        let low = HEX_CHARS[(byte & 0x0f) as usize];

        buf[HEX_POS[hex_idx]] = high;
        buf[HEX_POS[hex_idx + 1]] = low;
        hex_idx += 2;
    }

    // Insert hyphens at standard positions
    buf[8] = b'-';
    buf[13] = b'-';
    buf[18] = b'-';
    buf[23] = b'-';

    // SAFETY: The buffer only contains valid UTF-8 characters (hex digits and hyphens)
    // that were explicitly set from the HEX_CHARS array and hyphen literals
    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
}
