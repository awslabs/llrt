// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rustls::pki_types::{pem::PemObject, PrivateKeyDer};

use crate::client_auth::Invalid;
#[cfg(not(feature = "_encrypted-keys"))]
use crate::client_auth::{is_encrypted_pem_key, ENCRYPTED_KEY};

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn load_private_key(
    pem: &[u8],
    passphrase: Option<&[u8]>,
) -> Result<PrivateKeyDer<'static>, BoxError> {
    #[cfg(feature = "_encrypted-keys")]
    if let Some(block) = llrt_pbe::pem::first_private_key_block(pem)
        .filter(|block| block.proc_type || block.label == b"ENCRYPTED PRIVATE KEY")
    {
        return encrypted::load(&block, passphrase.unwrap_or_default());
    }
    #[cfg(not(feature = "_encrypted-keys"))]
    {
        _ = passphrase;
        if is_encrypted_pem_key(pem) {
            return Err(ENCRYPTED_KEY.into());
        }
    }
    PrivateKeyDer::from_pem_slice(pem).map_err(|e| Invalid("Invalid key: ", e).into())
}

#[cfg(feature = "_encrypted-keys")]
mod encrypted {
    use llrt_pbe::pem::PemBlock;
    use llrt_pbe::pkcs8::EncryptedPrivateKeyInfo;
    use llrt_pbe::Error;
    use rustls::pki_types::PrivateKeyDer;

    use super::BoxError;
    use crate::error::{
        TlsError, BAD_DECRYPT, CIPHER_PARAMETER_ERROR, DATA_IS_WRONG, DECODER_UNSUPPORTED,
        DECODE_ERROR, EVP_UNSUPPORTED, INTERRUPTED_OR_CANCELLED, INVALID_ITERATION_COUNT,
        PARAM_NEGATIVE_VALUE, UNKNOWN_PBE_ALGORITHM, UNSUPPORTED_CIPHER, UNSUPPORTED_KDF,
        UNSUPPORTED_KEYLENGTH, UNSUPPORTED_PRF, UNSUPPORTED_SALT_TYPE, WRONG_FINAL_BLOCK_LENGTH,
    };

    /// OpenSSL's `PEM_BUFSIZE`, the longest passphrase Node.js accepts.
    const MAX_PASSPHRASE_LEN: usize = 1024;

    pub(super) fn load(
        block: &PemBlock<'_>,
        passphrase: &[u8],
    ) -> Result<PrivateKeyDer<'static>, BoxError> {
        if !block.complete {
            return Err(DECODER_UNSUPPORTED.into());
        }
        if block.proc_type {
            check_length(passphrase)?;
            return Err(match block.dek_cipher {
                Some(cipher) => {
                    not_supported(&[&String::from_utf8_lossy(cipher), " in traditional PEM"])
                },
                None => not_supported(&["traditional PEM"]),
            });
        }
        let der = block.decode().ok_or(DECODER_UNSUPPORTED)?;
        let key = EncryptedPrivateKeyInfo::parse(&der).map_err(node_error)?;
        check_length(passphrase)?;
        let key = key.decrypt(passphrase).map_err(node_error)?;
        Ok(PrivateKeyDer::Pkcs8(key.into()))
    }

    fn check_length(passphrase: &[u8]) -> Result<(), TlsError> {
        match passphrase.len() {
            0..=MAX_PASSPHRASE_LEN => Ok(()),
            _ => Err(INTERRUPTED_OR_CANCELLED),
        }
    }

    pub(super) fn node_error(error: Error) -> BoxError {
        let error = match error {
            Error::Unimplemented(name) => return not_supported(&[name]),
            Error::InvalidKey => DECODER_UNSUPPORTED,
            Error::UnknownScheme => UNKNOWN_PBE_ALGORITHM,
            Error::Unsupported => EVP_UNSUPPORTED,
            Error::InvalidParameters => DECODE_ERROR,
            Error::UnsupportedKdf => UNSUPPORTED_KDF,
            Error::UnsupportedCipher => UNSUPPORTED_CIPHER,
            Error::InvalidCipherParameters => CIPHER_PARAMETER_ERROR,
            Error::InvalidIv => DATA_IS_WRONG,
            Error::UnsupportedKeyLength => UNSUPPORTED_KEYLENGTH,
            Error::UnsupportedPrf => UNSUPPORTED_PRF,
            Error::UnsupportedSaltType => UNSUPPORTED_SALT_TYPE,
            Error::ZeroIterations => INVALID_ITERATION_COUNT,
            Error::IterationCountOutOfRange => PARAM_NEGATIVE_VALUE,
            Error::WrongFinalBlockLength => WRONG_FINAL_BLOCK_LENGTH,
            Error::BadDecrypt => BAD_DECRYPT,
        };
        error.into()
    }

    fn not_supported(what: &[&str]) -> BoxError {
        const PREFIX: &str = "Invalid key: encrypted keys using ";
        const SUFFIX: &str = " are not supported";
        let len = PREFIX.len() + what.iter().map(|part| part.len()).sum::<usize>() + SUFFIX.len();
        let mut message = String::with_capacity(len);
        message.push_str(PREFIX);
        what.iter().for_each(|part| message.push_str(part));
        message.push_str(SUFFIX);
        message.into()
    }
}

#[cfg(all(test, feature = "_encrypted-keys"))]
mod tests {
    use std::path::Path;

    use llrt_pbe::Error as Pbe;

    use super::encrypted::node_error;
    use super::*;
    use crate::error::*;
    use crate::TlsError;

    const PASSPHRASE: &[u8] = "pässphrase".as_bytes();
    const TOO_LONG: &[u8] = &[b'x'; 1025];

    fn read(name: &str) -> String {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/llrt_test_tls/data");
        std::fs::read_to_string(data.join(name)).unwrap()
    }

    /// The block after the line `label` in `ec-encrypted.pem`.
    fn fixture(label: &str) -> String {
        let text = read("ec-encrypted.pem");
        let pem: Vec<&str> = text
            .lines()
            .skip_while(|line| *line != label)
            .skip(1)
            .take_while(|line| !line.starts_with("-----END"))
            .collect();
        pem.join("\n") + "\n-----END ENCRYPTED PRIVATE KEY-----\n"
    }

    fn load(pem: &str, passphrase: Option<&[u8]>) -> Result<PrivateKeyDer<'static>, BoxError> {
        load_private_key(pem.as_bytes(), passphrase)
    }

    fn error(pem: &str, passphrase: Option<&[u8]>) -> TlsError {
        *load(pem, passphrase)
            .unwrap_err()
            .downcast::<TlsError>()
            .unwrap()
    }

    fn message(pem: &str, passphrase: Option<&[u8]>) -> String {
        let e = load(pem, passphrase).unwrap_err();
        assert!(e.downcast_ref::<TlsError>().is_none());
        e.to_string()
    }

    fn not_supported(what: &str) -> String {
        format!("Invalid key: encrypted keys using {what} are not supported")
    }

    #[test]
    fn maps_crate_errors_to_node_errors() {
        let cases = [
            (Pbe::InvalidKey, DECODER_UNSUPPORTED),
            (Pbe::UnknownScheme, UNKNOWN_PBE_ALGORITHM),
            (Pbe::Unsupported, EVP_UNSUPPORTED),
            (Pbe::InvalidParameters, DECODE_ERROR),
            (Pbe::UnsupportedKdf, UNSUPPORTED_KDF),
            (Pbe::UnsupportedCipher, UNSUPPORTED_CIPHER),
            (Pbe::InvalidCipherParameters, CIPHER_PARAMETER_ERROR),
            (Pbe::InvalidIv, DATA_IS_WRONG),
            (Pbe::UnsupportedKeyLength, UNSUPPORTED_KEYLENGTH),
            (Pbe::UnsupportedPrf, UNSUPPORTED_PRF),
            (Pbe::UnsupportedSaltType, UNSUPPORTED_SALT_TYPE),
            (Pbe::ZeroIterations, INVALID_ITERATION_COUNT),
            (Pbe::IterationCountOutOfRange, PARAM_NEGATIVE_VALUE),
            (Pbe::WrongFinalBlockLength, WRONG_FINAL_BLOCK_LENGTH),
            (Pbe::BadDecrypt, BAD_DECRYPT),
        ];
        for (error, expected) in cases {
            let e = node_error(error).downcast::<TlsError>().unwrap();
            assert_eq!(*e, expected, "{error:?}");
        }
        let e = node_error(Pbe::Unimplemented("HMAC-SM3"));
        assert!(e.downcast_ref::<TlsError>().is_none());
        assert_eq!(e.to_string(), not_supported("HMAC-SM3"));
    }

    #[test]
    fn passphrase_longer_than_openssl_buffer() {
        let two_byte_chars = "ä".repeat(513);
        for key in [
            read("client-encrypted.key"),
            read("client-encrypted-traditional.key"),
        ] {
            assert_eq!(error(&key, Some(TOO_LONG)), INTERRUPTED_OR_CANCELLED);
            assert_eq!(
                error(&key, Some(two_byte_chars.as_bytes())),
                INTERRUPTED_OR_CANCELLED
            );
        }
        let key = read("client-encrypted.key");
        for passphrase in [&[b'x'; 1024][..], &two_byte_chars.as_bytes()[2..]] {
            let e = load(&key, Some(passphrase)).unwrap_err();
            assert_ne!(e.downcast_ref(), Some(&INTERRUPTED_OR_CANCELLED));
        }
        // The structure is decoded before the passphrase is read
        let truncated = "-----BEGIN ENCRYPTED PRIVATE KEY-----\n\
                         MIIBvTBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQI\n\
                         -----END ENCRYPTED PRIVATE KEY-----\n";
        assert_eq!(error(truncated, Some(TOO_LONG)), DECODER_UNSUPPORTED);
        let legacy = read("ec-encrypted-legacy.pem");
        assert_eq!(error(&legacy, Some(TOO_LONG)), INTERRUPTED_OR_CANCELLED);
    }

    #[test]
    fn traditional_pem_and_incomplete_blocks() {
        let traditional = read("client-encrypted-traditional.key");
        assert_eq!(
            message(&traditional, Some(PASSPHRASE)),
            not_supported("AES-256-CBC in traditional PEM")
        );
        assert_eq!(
            message(&traditional.replace("DEK-Info", "X-Info"), Some(PASSPHRASE)),
            not_supported("traditional PEM")
        );
        let key = read("client-encrypted.key");
        let cases = [
            key.replace("-----END ENCRYPTED PRIVATE KEY-----", ""),
            key.replace("END ENCRYPTED PRIVATE KEY", "END PRIVATE KEY"),
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\n@@@@\n-----END ENCRYPTED PRIVATE KEY-----\n"
                .into(),
        ];
        for pem in cases {
            assert_eq!(error(&pem, Some(PASSPHRASE)), DECODER_UNSUPPORTED);
        }
    }

    #[test]
    fn loads_keys_by_digest() {
        let plain = load(&read("ec.key"), None).unwrap();
        let cases = [
            (
                cfg!(feature = "encrypted-keys-hmac-sha1"),
                "aes-256-cbc hmacWithSHA1",
                "HMAC-SHA1",
            ),
            (
                cfg!(feature = "encrypted-keys-hmac-sha2"),
                "aes-192-cbc hmacWithSHA256",
                "HMAC-SHA256",
            ),
            (
                cfg!(feature = "encrypted-keys-hmac-sha3"),
                "aes-192-cbc hmac-sha3-256",
                "HMAC-SHA3-256",
            ),
            (
                cfg!(feature = "encrypted-keys-hmac-md5"),
                "aes-128-cbc hmacWithMD5",
                "HMAC-MD5",
            ),
        ];
        for (on, label, name) in cases {
            let key = fixture(label);
            if on {
                assert_eq!(load(&key, Some(PASSPHRASE)).unwrap(), plain, "{label}");
                assert_eq!(error(&key, Some(b"wrong")), BAD_DECRYPT, "{label}");
            } else {
                assert_eq!(message(&key, Some(PASSPHRASE)), not_supported(name));
            }
        }
    }

    #[cfg(feature = "encrypted-keys-hmac-sha2")]
    #[test]
    fn no_passphrase_is_an_empty_one() {
        let plain = load(&read("ec.key"), None).unwrap();
        assert_eq!(load(&read("ec-encrypted-empty.key"), None).unwrap(), plain);
        assert_eq!(error(&read("client-encrypted.key"), None), BAD_DECRYPT);
    }

    #[test]
    fn uses_the_first_private_key_block() {
        let plain = load(&read("ec.key"), None).unwrap();
        let key = fixture("aes-192-cbc hmacWithSHA256");
        let plain_first = read("ec.key") + &key;
        assert_eq!(load(&plain_first, Some(b"wrong")).unwrap(), plain);
        let encrypted_first = key + &read("ec.key");
        load(&encrypted_first, Some(b"wrong")).unwrap_err();
    }

    #[test]
    fn unencrypted_keys_are_unchanged() {
        for passphrase in [None, Some(&b"unused"[..]), Some(TOO_LONG)] {
            load(&read("client.key"), passphrase).unwrap();
        }
        let e = load("not a key", None).unwrap_err();
        assert!(e.downcast_ref::<TlsError>().is_none());
        assert!(e.to_string().starts_with("Invalid key: "), "{e}");
    }
}

#[cfg(all(test, not(feature = "_encrypted-keys")))]
mod tests_without_encrypted_keys {
    use std::path::Path;

    use super::*;

    #[test]
    fn encrypted_keys_are_not_supported() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/llrt_test_tls/data");
        let read = |name| std::fs::read(data.join(name)).unwrap();
        for name in ["client-encrypted.key", "client-encrypted-traditional.key"] {
            let e = load_private_key(&read(name), Some("pässphrase".as_bytes())).unwrap_err();
            assert_eq!(
                e.to_string(),
                "Invalid key: encrypted keys are not supported"
            );
        }
        load_private_key(&read("client.key"), None).unwrap();
    }
}
