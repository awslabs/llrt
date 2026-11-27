// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use cbc::cipher::{block_padding::Pkcs7, BlockCipherDecrypt, BlockModeDecrypt, KeyInit, KeyIvInit};
use der::asn1::{AnyRef, ObjectIdentifier as Oid};
use der::{Choice, Decode, DecodeValue, Reader, Sequence, SliceReader, Tag, Tagged};

use crate::Error;

#[derive(Sequence)]
pub(crate) struct AlgorithmIdentifier<'a> {
    pub oid: Oid,
    pub params: Option<AnyRef<'a>>,
}

#[derive(Sequence)]
struct Pbes2Params<'a> {
    kdf: AlgorithmIdentifier<'a>,
    enc: AnyRef<'a>,
}

#[derive(Sequence)]
struct Pbkdf2Params<'a> {
    salt: AnyRef<'a>,
    iterations: i64,
    key_length: Option<i64>,
    prf: Option<AlgorithmIdentifier<'a>>,
}

mod oid {
    use der::asn1::ObjectIdentifier as Oid;

    pub const PBES2: Oid = Oid::new_unwrap("1.2.840.113549.1.5.13");
    pub const PBKDF2: Oid = Oid::new_unwrap("1.2.840.113549.1.5.12");
    pub const SCRYPT: Oid = Oid::new_unwrap("1.3.6.1.4.1.11591.4.11");
    pub const PKCS5: Oid = Oid::new_unwrap("1.2.840.113549.1.5");
    pub const PKCS12_PBE: Oid = Oid::new_unwrap("1.2.840.113549.1.12.1");

    pub const HMAC_MD5: Oid = Oid::new_unwrap("1.2.840.113549.2.6");
    pub const HMAC_SHA1: Oid = Oid::new_unwrap("1.2.840.113549.2.7");
    pub const HMAC_SHA224: Oid = Oid::new_unwrap("1.2.840.113549.2.8");
    pub const HMAC_SHA256: Oid = Oid::new_unwrap("1.2.840.113549.2.9");
    pub const HMAC_SHA384: Oid = Oid::new_unwrap("1.2.840.113549.2.10");
    pub const HMAC_SHA512: Oid = Oid::new_unwrap("1.2.840.113549.2.11");
    pub const HMAC_SHA512_224: Oid = Oid::new_unwrap("1.2.840.113549.2.12");
    pub const HMAC_SHA512_256: Oid = Oid::new_unwrap("1.2.840.113549.2.13");
    pub const HMAC_MD5_IPSEC: Oid = Oid::new_unwrap("1.3.6.1.5.5.8.1.1");
    pub const HMAC_SHA1_IPSEC: Oid = Oid::new_unwrap("1.3.6.1.5.5.8.1.2");
    pub const HMAC_SHA3_224: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.13");
    pub const HMAC_SHA3_256: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.14");
    pub const HMAC_SHA3_384: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.15");
    pub const HMAC_SHA3_512: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.16");
    pub const HMAC_SM3: Oid = Oid::new_unwrap("1.2.156.10197.1.401.3.1");
    pub const HMAC_GOST_94: Oid = Oid::new_unwrap("1.2.643.2.2.10");
    pub const HMAC_GOST_2012_256: Oid = Oid::new_unwrap("1.2.643.7.1.1.4.1");
    pub const HMAC_GOST_2012_512: Oid = Oid::new_unwrap("1.2.643.7.1.1.4.2");

    pub const AES: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.1");
    pub const DES_EDE3_CBC: Oid = Oid::new_unwrap("1.2.840.113549.3.7");
    pub const DES_EDE: Oid = Oid::new_unwrap("1.3.14.3.2.17");
    pub const DES3_WRAP: Oid = Oid::new_unwrap("1.2.840.113549.1.9.16.3.6");
    pub const CAMELLIA: Oid = Oid::new_unwrap("1.2.392.200011.61.1.1");
    pub const CAMELLIA_NTT: Oid = Oid::new_unwrap("0.3.4401.5.3.1.9");
    pub const ARIA: Oid = Oid::new_unwrap("1.2.410.200046.1.1");
    pub const SM4: Oid = Oid::new_unwrap("1.2.156.10197.1.104");
}

/// Decrypts `data` with a password-based encryption scheme, making
/// OpenSSL's checks in OpenSSL's order.
pub(crate) fn decrypt(
    scheme: &AlgorithmIdentifier<'_>,
    passphrase: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, Error> {
    match scheme.oid {
        oid::PBES2 => {},
        oid => {
            return Err(match child_arc(&oid, oid::PKCS12_PBE) {
                Some(3) => Error::Unimplemented("PBE-SHA1-3DES"),
                Some(4) => Error::Unimplemented("PBE-SHA1-2DES"),
                // RC4 and RC2, which only OpenSSL's legacy provider has
                Some(1 | 2 | 5 | 6) => Error::Unsupported,
                _ => match child_arc(&oid, oid::PKCS5) {
                    // PBES1
                    Some(1 | 3 | 4 | 6 | 10 | 11) => Error::Unsupported,
                    _ => Error::UnknownScheme,
                },
            });
        },
    }
    let Pbes2Params { kdf, enc } = decode_params(scheme.params)?;
    let scrypt = match kdf.oid {
        oid::PBKDF2 => false,
        oid::SCRYPT => true,
        _ => return Err(Error::UnsupportedKdf),
    };
    let (cipher, cipher_params) = cipher_identifier(enc)?;
    let key_len = aes_cbc_key_len(&cipher)?;
    let iv = cbc_iv(cipher_params)?;
    if scrypt {
        return Err(Error::Unimplemented("scrypt"));
    }
    let params: Pbkdf2Params = decode_params(kdf.params)?;
    if params.key_length.is_some_and(|len| len != key_len as i64) {
        return Err(Error::UnsupportedKeyLength);
    }
    let pbkdf2 = pbkdf2_with(&params.prf.map_or(oid::HMAC_SHA1, |prf| prf.oid))?;
    if params.salt.tag() != Tag::OctetString {
        return Err(Error::UnsupportedSaltType);
    }
    // OpenSSL reads the count into a C int
    let iterations = match i32::try_from(params.iterations) {
        Ok(0) => return Err(Error::ZeroIterations),
        Ok(n @ 1..) => n as u32,
        _ => return Err(Error::IterationCountOutOfRange),
    };
    let mut key = [0; 32];
    let key = &mut key[..key_len];
    pbkdf2(passphrase, params.salt.value(), iterations, key)
        .expect("HMAC takes keys of any length");
    aes_cbc_decrypt(key, &iv, data)
}

fn child_arc(oid: &Oid, parent: Oid) -> Option<u8> {
    match oid.as_bytes().strip_prefix(parent.as_bytes()) {
        Some(&[arc]) if arc < 0x80 => Some(arc),
        _ => None,
    }
}

fn decode_params<'a, T>(params: Option<AnyRef<'a>>) -> Result<T, Error>
where
    T: Choice<'a> + DecodeValue<'a>,
{
    params
        .and_then(|params| params.decode_as().ok())
        .ok_or(Error::InvalidParameters)
}

/// The cipher and its raw parameters, which OpenSSL writes with an invalid tag for key wrap.
fn cipher_identifier(enc: AnyRef<'_>) -> Result<(Oid, &[u8]), Error> {
    let mut reader = SliceReader::new(enc.value()).map_err(|_| Error::InvalidParameters)?;
    match (enc.tag(), reader.decode::<Oid>()) {
        (Tag::Sequence, Ok(oid)) => {
            let params = reader.read_slice(reader.remaining_len());
            Ok((oid, params.map_err(|_| Error::InvalidParameters)?))
        },
        _ => Err(Error::InvalidParameters),
    }
}

fn aes_cbc_key_len(oid: &Oid) -> Result<usize, Error> {
    if let Some(arc) = child_arc(oid, oid::AES) {
        let name = match arc {
            2 | 22 | 42 => return Ok(16 + 8 * usize::from(arc / 20)),
            6 | 26 | 46 => return Err(Error::InvalidCipherParameters),
            7 | 27 | 47 => return Err(Error::UnsupportedCipher),
            1 => "AES-128-ECB",
            3 => "AES-128-OFB",
            4 => "AES-128-CFB",
            5 => "AES-128-WRAP",
            8 => "AES-128-WRAP-PAD",
            21 => "AES-192-ECB",
            23 => "AES-192-OFB",
            24 => "AES-192-CFB",
            25 => "AES-192-WRAP",
            28 => "AES-192-WRAP-PAD",
            41 => "AES-256-ECB",
            43 => "AES-256-OFB",
            44 => "AES-256-CFB",
            45 => "AES-256-WRAP",
            48 => "AES-256-WRAP-PAD",
            _ => return Err(Error::Unsupported),
        };
        return Err(Error::Unimplemented(name));
    }
    Err(match *oid {
        oid::DES_EDE3_CBC => Error::Unimplemented("DES-EDE3-CBC"),
        oid::DES_EDE => Error::Unimplemented("DES-EDE"),
        oid::DES3_WRAP => Error::Unimplemented("DES3-WRAP"),
        oid if oid.starts_with(oid::CAMELLIA) || oid.starts_with(oid::CAMELLIA_NTT) => {
            Error::Unimplemented("Camellia")
        },
        oid if oid.starts_with(oid::ARIA) => Error::Unimplemented("ARIA"),
        oid if oid.starts_with(oid::SM4) => Error::Unimplemented("SM4"),
        // DES, RC2, RC5, … (legacy provider only), and unknown ciphers
        _ => Error::Unsupported,
    })
}

fn cbc_iv(params: &[u8]) -> Result<[u8; 16], Error> {
    // OpenSSL keeps a zero IV when the parameter is absent
    if params.is_empty() {
        return Ok([0; 16]);
    }
    match AnyRef::from_der(params) {
        Ok(iv) if iv.tag() == Tag::OctetString => iv
            .value()
            .try_into()
            .map_err(|_| Error::InvalidCipherParameters),
        _ => Err(Error::InvalidIv),
    }
}

type Pbkdf2 = fn(&[u8], &[u8], u32, &mut [u8]) -> Result<(), hmac::digest::InvalidLength>;

macro_rules! pbkdf2_if {
    ($feature:literal, $mac:ty) => {{
        #[cfg(feature = $feature)]
        let pbkdf2: Option<Pbkdf2> = Some(pbkdf2::pbkdf2::<$mac>);
        #[cfg(not(feature = $feature))]
        let pbkdf2: Option<Pbkdf2> = None;
        pbkdf2
    }};
}

fn pbkdf2_with(prf: &Oid) -> Result<Pbkdf2, Error> {
    let (name, pbkdf2) = match *prf {
        oid::HMAC_SHA1 | oid::HMAC_SHA1_IPSEC => {
            ("HMAC-SHA1", pbkdf2_if!("hmac-sha1", hmac::Hmac<sha1::Sha1>))
        },
        oid::HMAC_SHA224 => (
            "HMAC-SHA224",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha224>),
        ),
        oid::HMAC_SHA256 => (
            "HMAC-SHA256",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha256>),
        ),
        oid::HMAC_SHA384 => (
            "HMAC-SHA384",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha384>),
        ),
        oid::HMAC_SHA512 => (
            "HMAC-SHA512",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha512>),
        ),
        oid::HMAC_SHA512_224 => (
            "HMAC-SHA512-224",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha512_224>),
        ),
        oid::HMAC_SHA512_256 => (
            "HMAC-SHA512-256",
            pbkdf2_if!("hmac-sha2", hmac::Hmac<sha2::Sha512_256>),
        ),
        oid::HMAC_SHA3_224 => (
            "HMAC-SHA3-224",
            pbkdf2_if!("hmac-sha3", hmac::SimpleHmac<sha3::Sha3_224>),
        ),
        oid::HMAC_SHA3_256 => (
            "HMAC-SHA3-256",
            pbkdf2_if!("hmac-sha3", hmac::SimpleHmac<sha3::Sha3_256>),
        ),
        oid::HMAC_SHA3_384 => (
            "HMAC-SHA3-384",
            pbkdf2_if!("hmac-sha3", hmac::SimpleHmac<sha3::Sha3_384>),
        ),
        oid::HMAC_SHA3_512 => (
            "HMAC-SHA3-512",
            pbkdf2_if!("hmac-sha3", hmac::SimpleHmac<sha3::Sha3_512>),
        ),
        oid::HMAC_MD5 | oid::HMAC_MD5_IPSEC => {
            ("HMAC-MD5", pbkdf2_if!("hmac-md5", hmac::Hmac<md5::Md5>))
        },
        oid::HMAC_SM3 => ("HMAC-SM3", None),
        // In OpenSSL's table, without a digest in its default provider
        oid::HMAC_GOST_94 | oid::HMAC_GOST_2012_256 | oid::HMAC_GOST_2012_512 => {
            return Err(Error::Unsupported)
        },
        _ => return Err(Error::UnsupportedPrf),
    };
    pbkdf2.ok_or(Error::Unimplemented(name))
}

fn aes_cbc_decrypt(key: &[u8], iv: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, Error> {
    fn decrypt<C: BlockCipherDecrypt + KeyInit>(
        key: &[u8],
        iv: &[u8],
        buf: &mut [u8],
    ) -> Option<usize> {
        let decryptor = cbc::Decryptor::<C>::new_from_slices(key, iv).ok()?;
        decryptor.decrypt_padded::<Pkcs7>(buf).ok().map(<[u8]>::len)
    }
    if data.is_empty() || !data.len().is_multiple_of(16) {
        return Err(Error::WrongFinalBlockLength);
    }
    let mut buf = data.to_vec();
    let len = match key.len() {
        16 => decrypt::<aes::Aes128>(key, iv, &mut buf),
        24 => decrypt::<aes::Aes192>(key, iv, &mut buf),
        _ => decrypt::<aes::Aes256>(key, iv, &mut buf),
    };
    buf.truncate(len.ok_or(Error::BadDecrypt)?);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{der, read};
    use crate::pkcs8::EncryptedPrivateKeyInfo;

    const PASSPHRASE: &[u8] = "pässphrase".as_bytes();

    fn fixtures() -> Vec<(String, Vec<u8>)> {
        let text = read("ec-encrypted.pem");
        let mut lines = text.lines();
        let mut fixtures = Vec::new();
        while let Some(label) = lines.next() {
            let pem: Vec<&str> = lines
                .by_ref()
                .take_while(|l| !l.starts_with("-----END"))
                .collect();
            fixtures.push((label.to_string(), der(&pem.join("\n"))));
        }
        fixtures
    }

    fn fixture(label: &str) -> Vec<u8> {
        fixtures().into_iter().find(|(l, _)| l == label).unwrap().1
    }

    fn decrypt_key(der: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, Error> {
        let key = EncryptedPrivateKeyInfo::parse(der).unwrap();
        decrypt(&key.algorithm, passphrase, key.data.as_bytes())
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

        #[cfg(feature = "hmac-sha2")]
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
    #[cfg(feature = "hmac-sha2")]
    fn data(epki: &mut Node) -> &mut Vec<u8> {
        &mut epki.children[1].value
    }

    fn variant(edit: impl FnOnce(&mut Node)) -> Vec<u8> {
        let mut epki = Node::parse(&fixture("aes-192-cbc hmacWithSHA256")).remove(0);
        edit(&mut epki);
        epki.encode()
    }

    // OpenSSL writes key wrap's parameters as an EOC tag in the long form
    fn wrap_variant(cipher: &str) -> Vec<u8> {
        const PLACEHOLDER: [u8; 4] = [0x04, 0x02, 0xaa, 0xbb];
        let mut der = variant(|epki| {
            *enc(epki) = vec![
                Node::oid(cipher),
                Node::primitive(OCTET_STRING, &PLACEHOLDER[2..]),
            ];
        });
        let at = der.windows(4).position(|w| w == PLACEHOLDER).unwrap();
        der[at..at + 4].copy_from_slice(&[0x3f, 0x80, 0x00, 0x00]);
        der
    }

    #[cfg(feature = "hmac-sha2")]
    fn encrypt(epki: &mut Node, plain: &[u8], iv: &[u8]) {
        use cbc::cipher::BlockModeEncrypt;
        let params = pbkdf2_params(epki);
        let (salt, iterations) = (&params[0].value, &params[1].value);
        let iterations = iterations.iter().fold(0, |acc, b| acc << 8 | u32::from(*b));
        let mut key = [0; 24];
        pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha256>>(PASSPHRASE, salt, iterations, &mut key).unwrap();
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

    /// Whether the PRF in a fixture's label is on, and its name.
    fn prf(label: &str) -> (bool, &'static str) {
        let sha1 = cfg!(feature = "hmac-sha1");
        let sha2 = cfg!(feature = "hmac-sha2");
        let sha3 = cfg!(feature = "hmac-sha3");
        match label.split(' ').nth(1).unwrap() {
            "hmacWithSHA1" | "hmac-sha1" => (sha1, "HMAC-SHA1"),
            "hmacWithMD5" | "hmac-md5" => (cfg!(feature = "hmac-md5"), "HMAC-MD5"),
            "hmacWithSHA224" => (sha2, "HMAC-SHA224"),
            "hmacWithSHA256" => (sha2, "HMAC-SHA256"),
            "hmacWithSHA384" => (sha2, "HMAC-SHA384"),
            "hmacWithSHA512" => (sha2, "HMAC-SHA512"),
            "hmacWithSHA512-224" => (sha2, "HMAC-SHA512-224"),
            "hmacWithSHA512-256" => (sha2, "HMAC-SHA512-256"),
            "hmac-sha3-224" => (sha3, "HMAC-SHA3-224"),
            "hmac-sha3-256" => (sha3, "HMAC-SHA3-256"),
            "hmac-sha3-384" => (sha3, "HMAC-SHA3-384"),
            "hmac-sha3-512" => (sha3, "HMAC-SHA3-512"),
            prf => panic!("{prf}"),
        }
    }

    #[test]
    fn decrypts_every_prf_and_aes_key_size() {
        let expected = der(&read("ec.key"));
        let fixtures: Vec<_> = fixtures()
            .into_iter()
            .filter(|(label, _)| label.starts_with("aes-"))
            .collect();
        assert_eq!(fixtures.len(), 14);
        for (label, der) in fixtures {
            let mut ders = vec![der];
            if label.ends_with("hmac-sha1") {
                // hmacWithSHA1 named explicitly, which OpenSSL never writes
                let mut epki = Node::parse(&ders[0]).remove(0);
                *prf_oid(&mut epki) = Node::oid("1.2.840.113549.2.7");
                ders.push(epki.encode());
            }
            let result = match prf(&label) {
                (true, _) => Ok(expected.clone()),
                (false, name) => Err(Error::Unimplemented(name)),
            };
            for der in ders {
                assert_eq!(decrypt_key(&der, PASSPHRASE), result, "{label}");
            }
        }
    }

    #[cfg(feature = "hmac-sha2")]
    #[test]
    fn decrypts_openssl_variants() {
        let expected = der(&read("ec.key"));
        let der = variant(|epki| {
            pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[24]));
            pbkdf2_params(epki).last_mut().unwrap().children.truncate(1);
        });
        assert_eq!(decrypt_key(&der, PASSPHRASE), Ok(expected.clone()));
        // OpenSSL keeps a zero IV when the parameter is absent
        let der = variant(|epki| {
            encrypt(epki, &expected, &[0; 16]);
            enc(epki).truncate(1);
        });
        assert_eq!(decrypt_key(&der, PASSPHRASE), Ok(expected));
    }

    #[cfg(feature = "hmac-sha2")]
    #[test]
    fn bad_decrypt() {
        let der = der(&read("client-encrypted.key"));
        for passphrase in [&b""[..], b"wrong", "pässphrase\0junk".as_bytes()] {
            assert_eq!(decrypt_key(&der, passphrase), Err(Error::BadDecrypt));
        }
    }

    #[cfg(feature = "hmac-sha2")]
    #[test]
    fn wrong_final_block_length() {
        let cases = [
            variant(|epki| _ = data(epki).pop()),
            variant(|epki| data(epki).push(0)),
            variant(|epki| data(epki).clear()),
        ];
        for der in cases {
            assert_eq!(
                decrypt_key(&der, PASSPHRASE),
                Err(Error::WrongFinalBlockLength)
            );
        }
    }

    #[test]
    fn unsupported_algorithms() {
        let cipher = |oid: &str| variant(|epki| enc(epki)[0] = Node::oid(oid));
        let cases = [
            (
                cipher("2.16.840.1.101.3.4.1.46"),
                Error::InvalidCipherParameters,
            ), // AES-256-GCM
            (
                cipher("2.16.840.1.101.3.4.1.6"),
                Error::InvalidCipherParameters,
            ), // AES-128-GCM
            (cipher("2.16.840.1.101.3.4.1.47"), Error::UnsupportedCipher), // AES-256-CCM
            (cipher("2.16.840.1.101.3.4.1.49"), Error::Unsupported),
            (cipher("1.3.14.3.2.7"), Error::Unsupported), // DES-CBC
            (cipher("1.2.3.4"), Error::Unsupported),
            (
                variant(|epki| *prf_oid(epki) = Node::oid("1.2.840.113549.2.127")),
                Error::UnsupportedPrf,
            ),
            (
                variant(|epki| *prf_oid(epki) = Node::oid("1.2.643.2.2.10")),
                Error::Unsupported,
            ), // GOST
            (
                variant(|epki| kdf(epki).children[0] = Node::oid("1.2.3.4")),
                Error::UnsupportedKdf,
            ),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.3.4")),
                Error::UnknownScheme,
            ),
            (
                variant(|epki| *outer_oid(epki) = Node::oid("1.2.840.113549.1.5.14")),
                Error::UnknownScheme,
            ),
        ];
        for (i, (der, expected)) in cases.into_iter().enumerate() {
            assert_eq!(decrypt_key(&der, PASSPHRASE), Err(expected), "case {i}");
        }
        let legacy = read("ec-encrypted-legacy.pem").replace("\r\n", "\n");
        let blocks: Vec<&str> = legacy
            .split_inclusive("-----END ENCRYPTED PRIVATE KEY-----\n")
            .collect();
        assert_eq!(blocks.len(), 3);
        for pem in blocks {
            let der = der(pem);
            for passphrase in [PASSPHRASE, b"wrong"] {
                assert_eq!(
                    decrypt_key(&der, passphrase),
                    Err(Error::Unsupported),
                    "{pem}"
                );
            }
        }
    }

    #[test]
    fn malformed_parameters() {
        let null = || Node::primitive(NULL, &[]);
        let cases = [
            (
                variant(|epki| *pbes2(epki) = Node::primitive(OCTET_STRING, &[1])),
                Error::InvalidParameters,
            ),
            (
                variant(|epki| epki.children[0].children.truncate(1)),
                Error::InvalidParameters,
            ),
            (
                variant(|epki| kdf(epki).children[1] = null()),
                Error::InvalidParameters,
            ),
            (
                variant(|epki| pbes2(epki).children[1] = Node::primitive(OCTET_STRING, &[1])),
                Error::InvalidParameters,
            ),
            (
                variant(|epki| pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[16]))),
                Error::UnsupportedKeyLength,
            ),
            (
                variant(|epki| pbkdf2_params(epki).insert(2, Node::primitive(INTEGER, &[0xff]))),
                Error::UnsupportedKeyLength,
            ),
            (
                variant(|epki| enc(epki)[1] = Node::primitive(OCTET_STRING, &[0; 8])),
                Error::InvalidCipherParameters,
            ),
            (
                variant(|epki| enc(epki)[1] = Node::primitive(OCTET_STRING, &[0; 17])),
                Error::InvalidCipherParameters,
            ),
            (variant(|epki| enc(epki)[1] = null()), Error::InvalidIv),
            (wrap_variant("2.16.840.1.101.3.4.1.22"), Error::InvalidIv), // AES-192-CBC
        ];
        for (i, (der, expected)) in cases.into_iter().enumerate() {
            assert_eq!(decrypt_key(&der, PASSPHRASE), Err(expected), "case {i}");
        }
    }

    #[cfg(feature = "hmac-sha2")]
    #[test]
    fn malformed_pbkdf2_parameters() {
        let iterations =
            |value: &[u8]| variant(|epki| pbkdf2_params(epki)[1] = Node::primitive(INTEGER, value));
        let cases = [
            (
                variant(|epki| pbkdf2_params(epki)[0] = Node::sequence(vec![Node::oid("1.2.3.4")])),
                Error::UnsupportedSaltType,
            ),
            (iterations(&[0]), Error::ZeroIterations),
            (iterations(&[0xff]), Error::IterationCountOutOfRange),
            (
                iterations(&[0, 0x80, 0, 0, 0]),
                Error::IterationCountOutOfRange,
            ),
        ];
        for (i, (der, expected)) in cases.into_iter().enumerate() {
            assert_eq!(decrypt_key(&der, PASSPHRASE), Err(expected), "case {i}");
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
        ];
        for (der, name) in cases {
            assert_eq!(
                decrypt_key(&der, PASSPHRASE),
                Err(Error::Unimplemented(name))
            );
        }
    }
}
