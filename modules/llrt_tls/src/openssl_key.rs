// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::borrow::Cow;
use std::ffi::c_int;

use openssl::error::{Error, ErrorStack};
use openssl::pkey::{PKey, Private};

use crate::client_auth::is_encrypted_pem_key;
use crate::TlsError;

const INTERRUPTED_OR_CANCELLED: TlsError = TlsError {
    message: Cow::Borrowed("error:07880109:common libcrypto routines::interrupted or cancelled"),
    code: Some(Cow::Borrowed("ERR_OSSL_CRYPTO_INTERRUPTED_OR_CANCELLED")),
    library: Some(Cow::Borrowed("common libcrypto routines")),
    reason: Some(Cow::Borrowed("interrupted or cancelled")),
    openssl_error_stack: Vec::new(),
};

/// Loads a PEM private key as Node.js does. OpenSSL gets the passphrase
/// through a callback that copies every byte, an absent passphrase is empty,
/// and one longer than OpenSSL's buffer fails.
pub(crate) fn load_private_key(
    pem: &[u8],
    passphrase: Option<&[u8]>,
) -> Result<PKey<Private>, Box<dyn std::error::Error + Send + Sync>> {
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
    key.map_err(|stack| match node_error(&stack) {
        Some(e) if is_encrypted_pem_key(pem) => e.into(),
        _ => format!("Invalid key: {stack}").into(),
    })
}

/// Node.js's `ThrowCryptoError`: the earliest error gives `message`,
/// `library`, `reason` and `code`, and the others, newest first, give
/// `opensslErrorStack`.
fn node_error(stack: &ErrorStack) -> Option<TlsError> {
    let (first, rest) = stack.errors().split_first()?;
    let code = first.reason().map(|reason| {
        let library = node_library_prefix(first.library_code());
        let ossl = if library == "SSL_" { "" } else { "OSSL_" };
        format!(
            "ERR_{ossl}{library}{}",
            reason.to_uppercase().replace(' ', "_")
        )
    });
    Some(TlsError {
        message: error_string(first).into(),
        code: code.map(Into::into),
        library: first.library().map(|s| s.to_string().into()),
        reason: first.reason().map(|s| s.to_string().into()),
        openssl_error_stack: rest.iter().rev().map(error_string).collect(),
    })
}

/// OpenSSL's `ERR_error_string_n`.
fn error_string(e: &Error) -> String {
    let library = e
        .library()
        .map_or_else(|| format!("lib({})", e.library_code()), str::to_string);
    let reason = e
        .reason()
        .map_or_else(|| format!("reason({})", e.reason_code()), str::to_string);
    format!("error:{:08X}:{library}::{reason}", e.code())
}

/// The libraries that Node.js names in error codes (`OSSL_ERROR_CODES_MAP`).
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
            e.library.as_deref().unwrap_or_default(),
            e.reason.as_deref().unwrap_or_default(),
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
