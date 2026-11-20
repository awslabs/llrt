// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::borrow::Cow;
use std::ffi::c_int;
use std::fmt;

use hex_simd::AsciiCase;
use openssl::error::{Error, ErrorStack};
use openssl::pkey::{PKey, Private};

use crate::client_auth::is_encrypted_pem_key;
use crate::TlsError;

const INTERRUPTED_OR_CANCELLED: TlsError = TlsError {
    message: Cow::Borrowed("error:07880109:common libcrypto routines::interrupted or cancelled"),
    code: Some(Cow::Borrowed("ERR_OSSL_CRYPTO_INTERRUPTED_OR_CANCELLED")),
    library: Some("common libcrypto routines"),
    reason: Some("interrupted or cancelled"),
    openssl_error_stack: Vec::new(),
};

#[derive(Debug)]
struct InvalidKey(ErrorStack);

impl fmt::Display for InvalidKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Invalid key: ")?;
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for InvalidKey {}

pub(crate) fn load_private_key(
    pem: &[u8],
    passphrase: Option<&[u8]>,
) -> Result<PKey<Private>, Box<dyn std::error::Error + Send + Sync>> {
    // An empty passphrase stops OpenSSL from prompting on the terminal.
    let passphrase = passphrase.unwrap_or_default();
    let mut too_long = false;
    let key =
        PKey::private_key_from_pem_callback(pem, |buf| match buf.get_mut(..passphrase.len()) {
            Some(buf) => {
                buf.copy_from_slice(passphrase);
                Ok(passphrase.len())
            },
            None => {
                too_long = true;
                Ok(0)
            },
        });
    if too_long {
        return Err(INTERRUPTED_OR_CANCELLED.into());
    }
    key.map_err(|stack| {
        if is_encrypted_pem_key(pem) {
            if let Some(error) = node_error(&stack) {
                return error.into();
            }
        }
        InvalidKey(stack).into()
    })
}

/// Node.js's `ThrowCryptoError`.
fn node_error(stack: &ErrorStack) -> Option<TlsError> {
    let (first, rest) = stack.errors().split_first()?;
    let reason = first.reason();
    Some(TlsError {
        message: error_string(first).into(),
        code: reason.map(|reason| node_code(first.library_code(), reason).into()),
        library: first.library(),
        reason,
        openssl_error_stack: rest.iter().rev().map(error_string).collect(),
    })
}

fn node_code(library: c_int, reason: &str) -> String {
    let library = node_library_prefix(library);
    let ossl = if library == "SSL_" { "" } else { "OSSL_" };
    let mut code = String::with_capacity(4 + ossl.len() + library.len() + reason.len());
    code.push_str("ERR_");
    code.push_str(ossl);
    code.push_str(library);
    code.extend(reason.chars().map(|c| match c {
        ' ' => '_',
        c => c.to_ascii_uppercase(),
    }));
    code
}

/// OpenSSL's `ERR_error_string_n`.
fn error_string(e: &Error) -> String {
    let mut s = String::with_capacity(64);
    s.push_str("error:");
    // `%08lX` of a code that fits in 32 bits
    let code = e.code().to_be_bytes();
    hex_simd::encode_append(&code[code.len() - 4..], &mut s, AsciiCase::Upper);
    s.push(':');
    match e.library() {
        Some(library) => s.push_str(library),
        None => {
            s.push_str("lib(");
            s.push_str(itoa::Buffer::new().format(e.library_code()));
            s.push(')');
        },
    }
    s.push_str("::");
    match e.reason() {
        Some(reason) => s.push_str(reason),
        None => {
            s.push_str("reason(");
            s.push_str(itoa::Buffer::new().format(e.reason_code()));
            s.push(')');
        },
    }
    s
}

/// Node.js's `OSSL_ERROR_CODES_MAP`.
fn node_library_prefix(library: c_int) -> &'static str {
    match library {
        2 => "SYS_",
        3 => "BN_",
        4 => "RSA_",
        5 => "DH_",
        6 => "EVP_",
        7 => "BUF_",
        8 => "OBJ_",
        9 => "PEM_",
        10 => "DSA_",
        11 => "X509_",
        13 => "ASN1_",
        14 => "CONF_",
        15 => "CRYPTO_",
        16 => "EC_",
        20 => "SSL_",
        32 => "BIO_",
        33 => "PKCS7_",
        34 => "X509V3_",
        35 => "PKCS12_",
        36 => "RAND_",
        37 => "DSO_",
        38 => "ENGINE_",
        39 => "OCSP_",
        40 => "UI_",
        41 => "COMP_",
        42 => "ECDSA_",
        43 => "ECDH_",
        44 => "OSSL_STORE_",
        45 => "FIPS_",
        46 => "CMS_",
        47 => "TS_",
        48 => "HMAC_",
        50 => "CT_",
        51 => "ASYNC_",
        52 => "KDF_",
        53 => "SM2_",
        128 => "USER_",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const PASSPHRASE: &[u8] = "pässphrase".as_bytes();
    const PKCS8: &str = "client-encrypted.key";
    const TRADITIONAL: &str = "client-encrypted-traditional.key";

    fn read(name: &str) -> Vec<u8> {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/llrt_test_tls/data");
        std::fs::read(data.join(name)).unwrap()
    }

    fn load_error(pem: &[u8], passphrase: Option<&[u8]>) -> TlsError {
        *load_private_key(pem, passphrase)
            .unwrap_err()
            .downcast::<TlsError>()
            .unwrap()
    }

    fn fields(e: &TlsError) -> [&str; 4] {
        [
            &e.message,
            e.code.as_deref().unwrap_or_default(),
            e.library.unwrap_or_default(),
            e.reason.unwrap_or_default(),
        ]
    }

    // OpenSSL 4 queues other errors under the same first one.
    fn is_openssl_3() -> bool {
        openssl::version::number() < 0x4000_0000
    }

    #[test]
    fn decrypts_encrypted_keys() {
        let expected = PKey::private_key_from_pem(&read("client.key")).unwrap();
        for name in [PKCS8, TRADITIONAL] {
            let key = load_private_key(&read(name), Some(PASSPHRASE)).unwrap();
            assert!(key.public_eq(&expected), "{name}");
        }
    }

    #[test]
    fn ignores_the_passphrase_of_an_unencrypted_key() {
        load_private_key(&read("client.key"), Some(b"unused")).unwrap();
    }

    #[test]
    fn bad_decrypt() {
        let wrong_passphrases = [
            None,
            Some(&b"wrong"[..]),
            Some("pässphrase\0junk".as_bytes()),
        ];
        for (name, stack) in [
            (
                PKCS8,
                &[
                    "error:11800074:PKCS12 routines::pkcs12 cipherfinal error",
                    "error:1C800064:Provider routines::bad decrypt",
                    "error:11800074:PKCS12 routines::pkcs12 cipherfinal error",
                ][..],
            ),
            (
                TRADITIONAL,
                &["error:04800065:PEM routines::bad decrypt"][..],
            ),
        ] {
            for passphrase in wrong_passphrases {
                let e = load_error(&read(name), passphrase);
                assert_eq!(
                    fields(&e),
                    [
                        "error:1C800064:Provider routines::bad decrypt",
                        "ERR_OSSL_BAD_DECRYPT",
                        "Provider routines",
                        "bad decrypt",
                    ],
                    "{name} {passphrase:?}"
                );
                if is_openssl_3() {
                    assert_eq!(e.openssl_error_stack, stack, "{name} {passphrase:?}");
                }
            }
        }
    }

    #[test]
    fn passphrase_longer_than_openssl_buffer() {
        let pem = read(PKCS8);
        assert_eq!(
            load_error(&pem, Some(&[b'x'; 1025])),
            INTERRUPTED_OR_CANCELLED
        );
        assert_eq!(
            load_error(&pem, Some(&[b'x'; 1024])).code.as_deref(),
            Some("ERR_OSSL_BAD_DECRYPT")
        );
    }

    #[test]
    fn malformed_encrypted_key() {
        let pem = b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIBvTBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQI\n-----END ENCRYPTED PRIVATE KEY-----\n";
        assert_eq!(
            fields(&load_error(pem, Some(PASSPHRASE))),
            [
                "error:1E08010C:DECODER routines::unsupported",
                "ERR_OSSL_UNSUPPORTED",
                "DECODER routines",
                "unsupported",
            ]
        );
    }

    #[test]
    fn unsupported_prf() {
        // hmacWithSHA256 changed to an unknown OID
        const HMAC_SHA256: &[u8] = &[0x06, 0x08, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x02, 0x09];
        let pem = String::from_utf8(read(PKCS8)).unwrap();
        let base64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        let mut der = openssl::base64::decode_block(&base64).unwrap();
        let at = der
            .windows(HMAC_SHA256.len())
            .position(|w| w == HMAC_SHA256)
            .unwrap();
        der[at + HMAC_SHA256.len() - 1] = 0x7f;
        let pem = format!(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\n{}\n-----END ENCRYPTED PRIVATE KEY-----\n",
            openssl::base64::encode_block(&der)
        );
        assert_eq!(
            fields(&load_error(pem.as_bytes(), Some(PASSPHRASE))),
            [
                "error:0300007D:digital envelope routines::unsupported prf",
                "ERR_OSSL_EVP_UNSUPPORTED_PRF",
                "digital envelope routines",
                "unsupported prf",
            ]
        );
    }

    #[test]
    fn invalid_unencrypted_key_keeps_its_message() {
        let e = load_private_key(b"not a key", None).unwrap_err();
        assert!(e.downcast_ref::<TlsError>().is_none());
        assert!(e.to_string().starts_with("Invalid key: "), "{e}");
    }
}
