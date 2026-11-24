// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use cbc::cipher::{block_padding::Pkcs7, BlockCipherDecrypt, BlockModeDecrypt, KeyInit, KeyIvInit};
use der::asn1::{AnyRef, ObjectIdentifier as Oid};
use der::{Choice, Decode, DecodeValue, Reader, Sequence, SliceReader, Tag, Tagged};
use hmac::{Hmac, SimpleHmac};

#[derive(Sequence)]
pub struct AlgorithmIdentifier<'a> {
    pub oid: Oid,
    pub params: Option<AnyRef<'a>>,
}

/// Why decryption failed, with OpenSSL's distinctions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    UnknownScheme,
    /// A scheme, cipher or PRF that OpenSSL's default provider doesn't have.
    Unsupported,
    /// An algorithm OpenSSL supports and this crate doesn't, by OpenSSL's name.
    Unimplemented(&'static str),
    InvalidParameters,
    UnsupportedKdf,
    UnsupportedCipher,
    InvalidCipherParameters,
    InvalidIv,
    UnsupportedKeyLength,
    UnsupportedPrf,
    UnsupportedSaltType,
    ZeroIterations,
    IterationCountOutOfRange,
    WrongFinalBlockLength,
    BadDecrypt,
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
pub fn decrypt(
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

fn pbkdf2_with(prf: &Oid) -> Result<Pbkdf2, Error> {
    use pbkdf2::pbkdf2;
    Ok(match *prf {
        oid::HMAC_SHA1 | oid::HMAC_SHA1_IPSEC => pbkdf2::<Hmac<sha1::Sha1>>,
        oid::HMAC_SHA224 => pbkdf2::<Hmac<sha2::Sha224>>,
        oid::HMAC_SHA256 => pbkdf2::<Hmac<sha2::Sha256>>,
        oid::HMAC_SHA384 => pbkdf2::<Hmac<sha2::Sha384>>,
        oid::HMAC_SHA512 => pbkdf2::<Hmac<sha2::Sha512>>,
        oid::HMAC_SHA512_224 => pbkdf2::<Hmac<sha2::Sha512_224>>,
        oid::HMAC_SHA512_256 => pbkdf2::<Hmac<sha2::Sha512_256>>,
        oid::HMAC_MD5 | oid::HMAC_MD5_IPSEC => pbkdf2::<Hmac<md5::Md5>>,
        oid::HMAC_SHA3_224 => pbkdf2::<SimpleHmac<sha3::Sha3_224>>,
        oid::HMAC_SHA3_256 => pbkdf2::<SimpleHmac<sha3::Sha3_256>>,
        oid::HMAC_SHA3_384 => pbkdf2::<SimpleHmac<sha3::Sha3_384>>,
        oid::HMAC_SHA3_512 => pbkdf2::<SimpleHmac<sha3::Sha3_512>>,
        oid::HMAC_SM3 => return Err(Error::Unimplemented("HMAC-SM3")),
        // In OpenSSL's table, without a digest in its default provider
        oid::HMAC_GOST_94 | oid::HMAC_GOST_2012_256 | oid::HMAC_GOST_2012_512 => {
            return Err(Error::Unsupported)
        },
        _ => return Err(Error::UnsupportedPrf),
    })
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
