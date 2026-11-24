// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rustls::pki_types::{pem::PemObject, PrivateKeyDer};

use crate::client_auth::Invalid;
#[cfg(not(feature = "encrypted-keys"))]
use crate::client_auth::{is_encrypted_pem_key, ENCRYPTED_KEY};

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn load_private_key(
    pem: &[u8],
    passphrase: Option<&[u8]>,
) -> Result<PrivateKeyDer<'static>, BoxError> {
    #[cfg(feature = "encrypted-keys")]
    if let Some(block) = llrt_pem::first_private_key_block(pem)
        .filter(|block| block.proc_type || block.label == b"ENCRYPTED PRIVATE KEY")
    {
        return encrypted::load(&block, passphrase.unwrap_or_default());
    }
    #[cfg(not(feature = "encrypted-keys"))]
    {
        _ = passphrase;
        if is_encrypted_pem_key(pem) {
            return Err(ENCRYPTED_KEY.into());
        }
    }
    PrivateKeyDer::from_pem_slice(pem).map_err(|e| Invalid("Invalid key: ", e).into())
}

#[cfg(feature = "encrypted-keys")]
mod encrypted {
    use llrt_pbe::Error as Pbe;
    use llrt_pem::PemBlock;
    use llrt_pkcs8::{EncryptedPrivateKeyInfo, Error as Pkcs8};
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

    fn node_error(error: Pkcs8) -> BoxError {
        let Pkcs8::Decrypt(error) = error else {
            return DECODER_UNSUPPORTED.into();
        };
        let error = match error {
            Pbe::Unimplemented(name) => return not_supported(&[name]),
            Pbe::UnknownScheme => UNKNOWN_PBE_ALGORITHM,
            Pbe::Unsupported => EVP_UNSUPPORTED,
            Pbe::InvalidParameters => DECODE_ERROR,
            Pbe::UnsupportedKdf => UNSUPPORTED_KDF,
            Pbe::UnsupportedCipher => UNSUPPORTED_CIPHER,
            Pbe::InvalidCipherParameters => CIPHER_PARAMETER_ERROR,
            Pbe::InvalidIv => DATA_IS_WRONG,
            Pbe::UnsupportedKeyLength => UNSUPPORTED_KEYLENGTH,
            Pbe::UnsupportedPrf => UNSUPPORTED_PRF,
            Pbe::UnsupportedSaltType => UNSUPPORTED_SALT_TYPE,
            Pbe::ZeroIterations => INVALID_ITERATION_COUNT,
            Pbe::IterationCountOutOfRange => PARAM_NEGATIVE_VALUE,
            Pbe::WrongFinalBlockLength => WRONG_FINAL_BLOCK_LENGTH,
            Pbe::BadDecrypt => BAD_DECRYPT,
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

#[cfg(all(test, feature = "encrypted-keys"))]
mod tests {
    use std::path::Path;

    use cbc::cipher::{block_padding::Pkcs7, BlockModeEncrypt, KeyIvInit};
    use der::asn1::ObjectIdentifier as Oid;
    use hmac::Hmac;

    use super::*;
    use crate::error::*;
    use crate::TlsError;

    const PASSPHRASE: &[u8] = "pässphrase".as_bytes();

    fn read(name: &str) -> String {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/llrt_test_tls/data");
        std::fs::read_to_string(data.join(name)).unwrap()
    }

    fn fixtures() -> Vec<(String, String)> {
        let text = read("ec-encrypted.pem");
        let mut lines = text.lines();
        let mut fixtures = Vec::new();
        while let Some(label) = lines.next() {
            let pem: Vec<&str> = lines
                .by_ref()
                .take_while(|l| !l.starts_with("-----END"))
                .collect();
            fixtures.push((
                label.to_string(),
                pem.join("\n") + "\n-----END ENCRYPTED PRIVATE KEY-----\n",
            ));
        }
        fixtures
    }

    fn fixture(label: &str) -> String {
        fixtures().into_iter().find(|(l, _)| l == label).unwrap().1
    }

    fn pem_der(pem: &str) -> Vec<u8> {
        let base64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        base64_simd::forgiving_decode_to_vec(base64.as_bytes()).unwrap()
    }

    fn pem(der: &[u8]) -> String {
        let base64 = base64_simd::STANDARD.encode_to_string(der);
        format!("-----BEGIN ENCRYPTED PRIVATE KEY-----\n{base64}\n-----END ENCRYPTED PRIVATE KEY-----\n")
    }

    fn decrypted(pem: &str, passphrase: Option<&[u8]>) -> Vec<u8> {
        match load_private_key(pem.as_bytes(), passphrase).unwrap() {
            PrivateKeyDer::Pkcs8(key) => key.secret_pkcs8_der().to_vec(),
            _ => panic!("not PKCS#8"),
        }
    }

    fn error(pem: &str, passphrase: Option<&[u8]>) -> TlsError {
        *load_private_key(pem.as_bytes(), passphrase)
            .unwrap_err()
            .downcast::<TlsError>()
            .unwrap()
    }

    fn not_supported_message(pem: &str) -> String {
        let e = load_private_key(pem.as_bytes(), Some(PASSPHRASE)).unwrap_err();
        assert!(e.downcast_ref::<TlsError>().is_none());
        e.to_string()
    }

    struct Node {
        tag: u8,
        value: Vec<u8>,
        children: Vec<Node>,
    }

    impl Node {
        fn parse(mut der: &[u8]) -> Vec<Node> {
            let mut nodes = Vec::new();
            while let [tag, len, rest @ ..] = der {
                let (len, rest) = match *len as usize {
                    len @ 0..0x80 => (len, rest),
                    n => {
                        let (len, rest) = rest.split_at(n & 0x7f);
                        (len.iter().fold(0, |acc, b| acc << 8 | *b as usize), rest)
                    },
                };
                let (value, tail) = rest.split_at(len);
                let children = if tag & 0x20 != 0 {
                    Node::parse(value)
                } else {
                    Vec::new()
                };
                nodes.push(Node {
                    tag: *tag,
                    value: value.to_vec(),
                    children,
                });
                der = tail;
            }
            nodes
        }

        fn encode(&self) -> Vec<u8> {
            let value = if self.tag & 0x20 != 0 {
                self.children.iter().flat_map(Node::encode).collect()
            } else {
                self.value.clone()
            };
            let mut der = vec![self.tag];
            match value.len() {
                len @ 0..0x80 => der.push(len as u8),
                len => {
                    let len = (len as u32).to_be_bytes();
                    let len = &len[len.iter().take_while(|b| **b == 0).count()..];
                    der.push(0x80 | len.len() as u8);
                    der.extend_from_slice(len);
                },
            }
            der.extend(value);
            der
        }

        fn primitive(tag: u8, value: &[u8]) -> Node {
            Node {
                tag,
                value: value.to_vec(),
                children: Vec::new(),
            }
        }

        fn oid(oid: &str) -> Node {
            Node::primitive(0x06, Oid::new_unwrap(oid).as_bytes())
        }

        fn sequence(children: Vec<Node>) -> Node {
            Node {
                tag: 0x30,
                value: Vec::new(),
                children,
            }
        }
    }

    const NULL: u8 = 0x05;
    const INTEGER: u8 = 0x02;
    const OCTET_STRING: u8 = 0x04;

    fn outer_oid(epki: &mut Node) -> &mut Node {
        &mut epki.children[0].children[0]
    }
    fn pbes2(epki: &mut Node) -> &mut Node {
        &mut epki.children[0].children[1]
    }
    fn kdf(epki: &mut Node) -> &mut Node {
        &mut pbes2(epki).children[0]
    }
    fn pbkdf2_params(epki: &mut Node) -> &mut Vec<Node> {
        &mut kdf(epki).children[1].children
    }
    fn prf_oid(epki: &mut Node) -> &mut Node {
        &mut pbkdf2_params(epki).last_mut().unwrap().children[0]
    }
    fn enc(epki: &mut Node) -> &mut Vec<Node> {
        &mut pbes2(epki).children[1].children
    }
    fn data(epki: &mut Node) -> &mut Vec<u8> {
        &mut epki.children[1].value
    }

    fn variant(edit: impl FnOnce(&mut Node)) -> String {
        let mut epki = Node::parse(&pem_der(&fixture("aes-192-cbc hmacWithSHA256"))).remove(0);
        edit(&mut epki);
        pem(&epki.encode())
    }

    // OpenSSL writes key wrap's parameters as an EOC tag in the long form
    fn wrap_variant(cipher: &str) -> String {
        const PLACEHOLDER: [u8; 4] = [0x04, 0x02, 0xaa, 0xbb];
        let mut der = pem_der(&variant(|epki| {
            *enc(epki) = vec![
                Node::oid(cipher),
                Node::primitive(OCTET_STRING, &PLACEHOLDER[2..]),
            ];
        }));
        let at = der.windows(4).position(|w| w == PLACEHOLDER).unwrap();
        der[at..at + 4].copy_from_slice(&[0x3f, 0x80, 0x00, 0x00]);
        pem(&der)
    }

    fn encrypt(epki: &mut Node, plain: &[u8], iv: &[u8]) {
        let params = pbkdf2_params(epki);
        let (salt, iterations) = (&params[0].value, &params[1].value);
        let iterations = iterations.iter().fold(0, |acc, b| acc << 8 | u32::from(*b));
        let mut key = [0; 24];
        pbkdf2::pbkdf2::<Hmac<sha2::Sha256>>(PASSPHRASE, salt, iterations, &mut key).unwrap();
        let mut buf = plain.to_vec();
        buf.resize(plain.len() + 16, 0);
        let len = cbc::Encryptor::<aes::Aes192>::new_from_slices(&key, iv)
            .unwrap()
            .encrypt_padded::<Pkcs7>(&mut buf, plain.len())
            .unwrap()
            .len();
        buf.truncate(len);
        *data(epki) = buf;
    }

    fn reencrypted(plain: &[u8]) -> String {
        variant(|epki| {
            let iv = enc(epki)[1].value.clone();
            encrypt(epki, plain, &iv);
        })
    }

    #[test]
    fn decrypts_openssl_variants() {
        let expected = pem_der(&read("ec.key"));
        let pem = variant(|epki| {
            pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[24]));
            pbkdf2_params(epki).last_mut().unwrap().children.truncate(1);
        });
        assert_eq!(decrypted(&pem, Some(PASSPHRASE)), expected);
        // OpenSSL keeps a zero IV when the parameter is absent
        let pem = variant(|epki| {
            encrypt(epki, &expected, &[0; 16]);
            enc(epki).truncate(1);
        });
        assert_eq!(decrypted(&pem, Some(PASSPHRASE)), expected);
        // OpenSSL ignores data after the key
        let pem = reencrypted(&[&expected[..], &[0]].concat());
        assert_eq!(decrypted(&pem, Some(PASSPHRASE)), expected);
    }

    #[test]
    fn decrypts_with_an_empty_passphrase() {
        let expected = pem_der(&read("ec.key"));
        assert_eq!(decrypted(&read("ec-encrypted-empty.key"), None), expected);
    }

    #[test]
    fn uses_the_first_private_key_block() {
        let expected = pem_der(&read("ec.key"));
        let key = fixture("aes-192-cbc hmacWithSHA256");
        let plain_first = read("ec.key") + &key;
        assert_eq!(decrypted(&plain_first, Some(b"wrong")), expected);
        let encrypted_first = key + &read("ec.key");
        assert_eq!(error(&encrypted_first, Some(b"wrong")), BAD_DECRYPT);
    }

    #[test]
    fn unencrypted_keys_are_unchanged() {
        for passphrase in [None, Some(&b"unused"[..]), Some(&[b'x'; 1025][..])] {
            load_private_key(read("client.key").as_bytes(), passphrase).unwrap();
        }
        let e = load_private_key(b"not a key", None).unwrap_err();
        assert!(e.downcast_ref::<TlsError>().is_none());
        assert!(e.to_string().starts_with("Invalid key: "), "{e}");
    }

    #[test]
    fn bad_decrypt() {
        let key = read("client-encrypted.key");
        let two_byte_chars = "ä".repeat(512);
        let wrong = [
            None,
            Some(&b"wrong"[..]),
            Some("pässphrase\0junk".as_bytes()),
            Some(&[b'x'; 1024][..]),
            Some(two_byte_chars.as_bytes()),
        ];
        for passphrase in wrong {
            assert_eq!(error(&key, passphrase), BAD_DECRYPT, "{passphrase:?}");
        }
    }

    #[test]
    fn passphrase_longer_than_openssl_buffer() {
        for key in [
            read("client-encrypted.key"),
            read("client-encrypted-traditional.key"),
        ] {
            assert_eq!(error(&key, Some(&[b'x'; 1025])), INTERRUPTED_OR_CANCELLED);
            assert_eq!(
                error(&key, Some("ä".repeat(513).as_bytes())),
                INTERRUPTED_OR_CANCELLED
            );
        }
        // The structure is decoded before the passphrase is read
        let truncated = pem(&pem_der(&read("client-encrypted.key"))[..40]);
        assert_eq!(error(&truncated, Some(&[b'x'; 1025])), DECODER_UNSUPPORTED);
        let legacy = variant(|epki| *outer_oid(epki) = Node::oid("1.2.840.113549.1.12.1.1"));
        assert_eq!(
            error(&legacy, Some(&[b'x'; 1025])),
            INTERRUPTED_OR_CANCELLED
        );
    }

    #[test]
    fn decoder_errors() {
        let key = fixture("aes-192-cbc hmacWithSHA256");
        let cases = [
            reencrypted(b"not DER"),
            reencrypted(b""),
            pem(&pem_der(&key)[..40]),
            pem(b"not DER"),
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\n@@@@\n-----END ENCRYPTED PRIVATE KEY-----\n"
                .into(),
            key.replace("-----END ENCRYPTED PRIVATE KEY-----\n", ""),
            key.replace("END ENCRYPTED PRIVATE KEY", "END PRIVATE KEY"),
            variant(|epki| epki.children[1].tag = 0x03),
            variant(|epki| epki.children.push(Node::primitive(INTEGER, &[1]))),
        ];
        for (i, pem) in cases.iter().enumerate() {
            assert_eq!(
                error(pem, Some(PASSPHRASE)),
                DECODER_UNSUPPORTED,
                "case {i}"
            );
        }
    }

    #[test]
    fn wrong_final_block_length() {
        let cases = [
            variant(|epki| _ = data(epki).pop()),
            variant(|epki| data(epki).push(0)),
            variant(|epki| data(epki).clear()),
        ];
        for pem in cases {
            assert_eq!(error(&pem, Some(PASSPHRASE)), WRONG_FINAL_BLOCK_LENGTH);
        }
    }

    #[test]
    fn unsupported_algorithms() {
        let cipher = |oid: &str| variant(|epki| enc(epki)[0] = Node::oid(oid));
        let cases = [
            (cipher("2.16.840.1.101.3.4.1.46"), CIPHER_PARAMETER_ERROR), // AES-256-GCM
            (cipher("2.16.840.1.101.3.4.1.6"), CIPHER_PARAMETER_ERROR),  // AES-128-GCM
            (cipher("2.16.840.1.101.3.4.1.47"), UNSUPPORTED_CIPHER),     // AES-256-CCM
            (cipher("2.16.840.1.101.3.4.1.49"), EVP_UNSUPPORTED),
            (cipher("1.3.14.3.2.7"), EVP_UNSUPPORTED), // DES-CBC
            (cipher("1.2.3.4"), EVP_UNSUPPORTED),
            (
                variant(|epki| *prf_oid(epki) = Node::oid("1.2.840.113549.2.127")),
                UNSUPPORTED_PRF,
            ),
            (
                variant(|epki| *prf_oid(epki) = Node::oid("1.2.643.2.2.10")),
                EVP_UNSUPPORTED,
            ), // GOST
            (
                variant(|epki| kdf(epki).children[0] = Node::oid("1.2.3.4")),
                UNSUPPORTED_KDF,
            ),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.3.4")),
                UNKNOWN_PBE_ALGORITHM,
            ),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.840.113549.1.5.14")),
                UNKNOWN_PBE_ALGORITHM,
            ),
        ];
        for (i, (pem, expected)) in cases.into_iter().enumerate() {
            assert_eq!(error(&pem, Some(PASSPHRASE)), expected, "case {i}");
        }
        let legacy = read("ec-encrypted-legacy.pem").replace("\r\n", "\n");
        let blocks: Vec<&str> = legacy
            .split_inclusive("-----END ENCRYPTED PRIVATE KEY-----\n")
            .collect();
        assert_eq!(blocks.len(), 3);
        for pem in blocks {
            assert_eq!(error(pem, Some(PASSPHRASE)), EVP_UNSUPPORTED, "{pem}");
            assert_eq!(error(pem, Some(b"wrong")), EVP_UNSUPPORTED, "{pem}");
        }
    }

    #[test]
    fn malformed_parameters() {
        let null = || Node::primitive(NULL, &[]);
        let cases = [
            (
                variant(|epki| *pbes2(epki) = Node::primitive(OCTET_STRING, &[1])),
                DECODE_ERROR,
            ),
            (
                variant(|epki| epki.children[0].children.truncate(1)),
                DECODE_ERROR,
            ),
            (variant(|epki| kdf(epki).children[1] = null()), DECODE_ERROR),
            (
                variant(|epki| pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[16]))),
                UNSUPPORTED_KEYLENGTH,
            ),
            (
                variant(|epki| pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[0xff]))),
                UNSUPPORTED_KEYLENGTH,
            ),
            (
                variant(|epki| pbkdf2_params(epki)[0] = Node::sequence(vec![Node::oid("1.2.3.4")])),
                UNSUPPORTED_SALT_TYPE,
            ),
            (
                variant(|epki| pbkdf2_params(epki)[1] = Node::primitive(INTEGER, &[0])),
                INVALID_ITERATION_COUNT,
            ),
            (
                variant(|epki| pbkdf2_params(epki)[1] = Node::primitive(INTEGER, &[0xff])),
                PARAM_NEGATIVE_VALUE,
            ),
            (
                variant(|epki| {
                    pbkdf2_params(epki)[1] = Node::primitive(INTEGER, &[0, 0x80, 0, 0, 0])
                }),
                PARAM_NEGATIVE_VALUE,
            ),
            (
                variant(|epki| enc(epki)[1] = Node::primitive(OCTET_STRING, &[0; 8])),
                CIPHER_PARAMETER_ERROR,
            ),
            (
                variant(|epki| enc(epki)[1] = Node::primitive(OCTET_STRING, &[0; 17])),
                CIPHER_PARAMETER_ERROR,
            ),
            (variant(|epki| enc(epki)[1] = null()), DATA_IS_WRONG),
            (wrap_variant("2.16.840.1.101.3.4.1.22"), DATA_IS_WRONG), // AES-192-CBC
            (
                variant(|epki| pbes2(epki).children[1] = Node::primitive(OCTET_STRING, &[1])),
                DECODE_ERROR,
            ),
        ];
        for (i, (pem, expected)) in cases.into_iter().enumerate() {
            assert_eq!(error(&pem, Some(PASSPHRASE)), expected, "case {i}");
        }
    }

    #[test]
    fn not_yet_supported() {
        let cipher = |oid: &str| variant(|epki| enc(epki)[0] = Node::oid(oid));
        let cases = [
            (cipher("1.2.840.113549.3.7"), "DES-EDE3-CBC"),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.840.113549.1.12.1.3")),
                "PBE-SHA1-3DES",
            ),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.840.113549.1.12.1.4")),
                "PBE-SHA1-2DES",
            ),
            (
                variant(|epki| kdf(epki).children[0] = Node::oid("1.3.6.1.4.1.11591.4.11")),
                "scrypt",
            ),
            (cipher("2.16.840.1.101.3.4.1.1"), "AES-128-ECB"),
            (cipher("2.16.840.1.101.3.4.1.23"), "AES-192-OFB"),
            (cipher("2.16.840.1.101.3.4.1.44"), "AES-256-CFB"),
            (cipher("2.16.840.1.101.3.4.1.45"), "AES-256-WRAP"),
            (cipher("2.16.840.1.101.3.4.1.8"), "AES-128-WRAP-PAD"),
            (
                fixture("id-aes128-wrap-pad hmacWithSHA256"),
                "AES-128-WRAP-PAD",
            ),
            (wrap_variant("2.16.840.1.101.3.4.1.25"), "AES-192-WRAP"),
            (fixture("des-ede hmacWithSHA256"), "DES-EDE"),
            (cipher("1.2.840.113549.1.9.16.3.6"), "DES3-WRAP"),
            (
                variant(|epki| *prf_oid(epki) = Node::oid("1.2.156.10197.1.401.3.1")),
                "HMAC-SM3",
            ),
            (cipher("1.2.392.200011.61.1.1.1.4"), "Camellia"),
            (cipher("0.3.4401.5.3.1.9.41"), "Camellia"),
            (cipher("1.2.410.200046.1.1.2"), "ARIA"),
            (cipher("1.2.156.10197.1.104.2"), "SM4"),
            (
                read("client-encrypted-traditional.key"),
                "AES-256-CBC in traditional PEM",
            ),
            (
                read("client-encrypted-traditional.key").replace("DEK-Info", "X-Info"),
                "traditional PEM",
            ),
        ];
        for (pem, what) in cases {
            assert_eq!(
                not_supported_message(&pem),
                format!("Invalid key: encrypted keys using {what} are not supported")
            );
        }
    }
}

#[cfg(all(test, not(feature = "encrypted-keys")))]
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
