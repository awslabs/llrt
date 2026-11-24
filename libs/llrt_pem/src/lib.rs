// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

pub struct PemBlock<'a> {
    pub label: &'a [u8],
    /// Whether the block has a `Proc-Type` header, which traditional
    /// OpenSSL encryption adds.
    pub proc_type: bool,
    /// The cipher name in the `DEK-Info` header.
    pub dek_cipher: Option<&'a [u8]>,
    /// Whether an `-----END <label>-----` line closes the block.
    pub complete: bool,
    body: &'a [u8],
}

impl PemBlock<'_> {
    pub fn decode(&self) -> Option<Vec<u8>> {
        base64_simd::forgiving_decode_to_vec(self.body).ok()
    }
}

/// The first private-key block, which OpenSSL's `PEM_read_bio_PrivateKey` uses.
pub fn first_private_key_block(pem: &[u8]) -> Option<PemBlock<'_>> {
    let mut offset = 0;
    let mut lines = pem.split_inclusive(|&b| b == b'\n').map(|line| {
        offset += line.len();
        (offset - line.len(), line.trim_ascii_end())
    });
    let label = loop {
        let (_, line) = lines.next()?;
        if let Some(label) = line
            .strip_prefix(b"-----BEGIN ")
            .and_then(|line| line.strip_suffix(b"-----"))
        {
            if label.ends_with(b"PRIVATE KEY") {
                break label;
            }
        }
    };
    let mut block = PemBlock {
        label,
        proc_type: false,
        dek_cipher: None,
        complete: false,
        body: &[],
    };
    let mut body_start = None;
    let mut body_end = pem.len();
    for (start, line) in lines {
        if let Some(end) = line.strip_prefix(b"-----END ") {
            block.complete = end.strip_suffix(b"-----") == Some(label);
            body_end = start;
            break;
        }
        match line.iter().position(|&b| b == b':') {
            Some(colon) if body_start.is_none() => {
                let value = line[colon + 1..].trim_ascii();
                match line[..colon].trim_ascii() {
                    b"Proc-Type" => block.proc_type = true,
                    b"DEK-Info" => {
                        block.dek_cipher = value
                            .split(|&b| b == b',')
                            .next()
                            .map(<[u8]>::trim_ascii)
                            .filter(|cipher| !cipher.is_empty())
                    },
                    _ => {},
                }
            },
            _ if body_start.is_none() && !line.is_empty() => body_start = Some(start),
            _ => {},
        }
    }
    if let Some(start) = body_start {
        block.body = &pem[start..body_end];
    }
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str =
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAEC\nAw==\n-----END ENCRYPTED PRIVATE KEY-----\n";

    #[test]
    fn reads_the_first_private_key_block() {
        let pem = "text\n-----BEGIN CERTIFICATE-----\nBAU=\n-----END CERTIFICATE-----\n".to_owned()
            + KEY
            + "-----BEGIN PRIVATE KEY-----\nBgc=\n-----END PRIVATE KEY-----\n";
        for pem in [pem.clone(), pem.replace('\n', "\r\n")] {
            let block = first_private_key_block(pem.as_bytes()).unwrap();
            assert_eq!(block.label, b"ENCRYPTED PRIVATE KEY");
            assert!(block.complete && !block.proc_type);
            assert_eq!(block.decode().unwrap(), [0, 1, 2, 3]);
        }
        assert!(first_private_key_block(b"-----BEGIN CERTIFICATE-----\n").is_none());
    }

    #[test]
    fn reads_traditional_encryption_headers() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n\
                   DEK-Info: AES-256-CBC,00112233\n\nAAEC\nAw==\n-----END RSA PRIVATE KEY-----\n";
        let block = first_private_key_block(pem.as_bytes()).unwrap();
        assert!(block.complete && block.proc_type);
        assert_eq!(block.dek_cipher, Some(&b"AES-256-CBC"[..]));
        assert_eq!(block.decode().unwrap(), [0, 1, 2, 3]);
    }

    #[test]
    fn incomplete_blocks() {
        let missing_end = KEY.replace("-----END ENCRYPTED PRIVATE KEY-----\n", "");
        let wrong_end = KEY.replace("END ENCRYPTED", "END");
        for pem in [missing_end, wrong_end] {
            assert!(!first_private_key_block(pem.as_bytes()).unwrap().complete);
        }
        let bad_base64 = KEY.replace("AAEC", "@@@@");
        assert!(first_private_key_block(bad_base64.as_bytes())
            .unwrap()
            .decode()
            .is_none());
    }
}
