// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use cbc::cipher::{block_padding::Pkcs7, BlockCipherDecrypt, BlockModeDecrypt, KeyInit, KeyIvInit};
use der::asn1::{AnyRef, ObjectIdentifier as Oid, OctetStringRef};
use der::{Choice, Decode, DecodeValue, Reader, Sequence, SliceReader, Tag, Tagged};
use hmac::{Hmac, SimpleHmac};

use crate::error::{
    TlsError, BAD_DECRYPT, CIPHER_PARAMETER_ERROR, DATA_IS_WRONG, DECODER_UNSUPPORTED,
    DECODE_ERROR, EVP_UNSUPPORTED, INTERRUPTED_OR_CANCELLED, INVALID_ITERATION_COUNT,
    PARAM_NEGATIVE_VALUE, UNKNOWN_PBE_ALGORITHM, UNSUPPORTED_CIPHER, UNSUPPORTED_KDF,
    UNSUPPORTED_KEYLENGTH, UNSUPPORTED_PRF, UNSUPPORTED_SALT_TYPE, WRONG_FINAL_BLOCK_LENGTH,
};
use crate::rustls_key::{not_supported, BoxError, MAX_PASSPHRASE_LEN};

#[derive(Sequence)]
struct AlgorithmIdentifier<'a> {
    oid: Oid,
    params: Option<AnyRef<'a>>,
}

#[derive(Sequence)]
struct EncryptedPrivateKeyInfo<'a> {
    alg: AlgorithmIdentifier<'a>,
    data: &'a OctetStringRef,
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

/// Decrypts a DER `EncryptedPrivateKeyInfo` to a PKCS#8 `PrivateKeyInfo`,
/// with OpenSSL's checks in OpenSSL's order.
pub(crate) fn decrypt(der: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, BoxError> {
    let (epki, _) =
        EncryptedPrivateKeyInfo::from_der_partial(der).map_err(|_| DECODER_UNSUPPORTED)?;
    if passphrase.len() > MAX_PASSPHRASE_LEN {
        return Err(INTERRUPTED_OR_CANCELLED.into());
    }
    match epki.alg.oid {
        oid::PBES2 => {},
        oid => {
            return Err(match child_arc(&oid, oid::PKCS12_PBE) {
                Some(3) => not_supported(&["PBE-SHA1-3DES"]),
                Some(4) => not_supported(&["PBE-SHA1-2DES"]),
                // RC4 and RC2, which only OpenSSL's legacy provider has
                Some(1 | 2 | 5 | 6) => EVP_UNSUPPORTED.into(),
                _ => match child_arc(&oid, oid::PKCS5) {
                    // PBES1
                    Some(1 | 3 | 4 | 6 | 10 | 11) => EVP_UNSUPPORTED.into(),
                    _ => UNKNOWN_PBE_ALGORITHM.into(),
                },
            });
        },
    }
    let Pbes2Params { kdf, enc } = decode_params(epki.alg.params)?;
    let scrypt = match kdf.oid {
        oid::PBKDF2 => false,
        oid::SCRYPT => true,
        _ => return Err(UNSUPPORTED_KDF.into()),
    };
    let (cipher, cipher_params) = cipher_identifier(enc)?;
    let key_len = aes_cbc_key_len(&cipher)?;
    let iv = cbc_iv(cipher_params)?;
    if scrypt {
        return Err(not_supported(&["scrypt"]));
    }
    let params: Pbkdf2Params = decode_params(kdf.params)?;
    if params.key_length.is_some_and(|len| len != key_len as i64) {
        return Err(UNSUPPORTED_KEYLENGTH.into());
    }
    let pbkdf2 = pbkdf2_with(&params.prf.map_or(oid::HMAC_SHA1, |prf| prf.oid))?;
    if params.salt.tag() != Tag::OctetString {
        return Err(UNSUPPORTED_SALT_TYPE.into());
    }
    // OpenSSL reads the count into a C int
    let iterations = match i32::try_from(params.iterations) {
        Ok(0) => return Err(INVALID_ITERATION_COUNT.into()),
        Ok(n @ 1..) => n as u32,
        _ => return Err(PARAM_NEGATIVE_VALUE.into()),
    };
    let mut key = [0; 32];
    let key = &mut key[..key_len];
    pbkdf2(passphrase, params.salt.value(), iterations, key)
        .expect("HMAC takes keys of any length");
    let plain = aes_cbc_decrypt(key, &iv, epki.data.as_bytes())?;
    Ok(private_key_info(plain)?)
}

fn child_arc(oid: &Oid, parent: Oid) -> Option<u8> {
    match oid.as_bytes().strip_prefix(parent.as_bytes()) {
        Some(&[arc]) if arc < 0x80 => Some(arc),
        _ => None,
    }
}

fn decode_params<'a, T>(params: Option<AnyRef<'a>>) -> Result<T, TlsError>
where
    T: Choice<'a> + DecodeValue<'a>,
{
    params
        .and_then(|params| params.decode_as().ok())
        .ok_or(DECODE_ERROR)
}

/// The cipher and its undecoded parameters. For key wrap, OpenSSL writes
/// parameters with an invalid tag.
fn cipher_identifier(enc: AnyRef<'_>) -> Result<(Oid, &[u8]), TlsError> {
    let mut reader = SliceReader::new(enc.value()).map_err(|_| DECODE_ERROR)?;
    match (enc.tag(), reader.decode::<Oid>()) {
        (Tag::Sequence, Ok(oid)) => {
            let params = reader.read_slice(reader.remaining_len());
            Ok((oid, params.map_err(|_| DECODE_ERROR)?))
        },
        _ => Err(DECODE_ERROR),
    }
}

/// The key length of an AES-CBC cipher, and OpenSSL's errors for the other
/// PBES2 ciphers.
fn aes_cbc_key_len(oid: &Oid) -> Result<usize, BoxError> {
    if let Some(arc) = child_arc(oid, oid::AES) {
        let bits = match arc / 20 {
            0 => "128",
            1 => "192",
            2 => "256",
            _ => return Err(EVP_UNSUPPORTED.into()),
        };
        let mode = match arc % 20 {
            2 => return Ok(16 + 8 * usize::from(arc / 20)),
            6 => return Err(CIPHER_PARAMETER_ERROR.into()),
            7 => return Err(UNSUPPORTED_CIPHER.into()),
            1 => "ECB",
            3 => "OFB",
            4 => "CFB",
            5 => "WRAP",
            8 => "WRAP-PAD",
            _ => return Err(EVP_UNSUPPORTED.into()),
        };
        return Err(not_supported(&["AES-", bits, "-", mode]));
    }
    Err(match *oid {
        oid::DES_EDE3_CBC => not_supported(&["DES-EDE3-CBC"]),
        oid::DES_EDE => not_supported(&["DES-EDE"]),
        oid::DES3_WRAP => not_supported(&["DES3-WRAP"]),
        oid if oid.starts_with(oid::CAMELLIA) || oid.starts_with(oid::CAMELLIA_NTT) => {
            not_supported(&["Camellia"])
        },
        oid if oid.starts_with(oid::ARIA) => not_supported(&["ARIA"]),
        oid if oid.starts_with(oid::SM4) => not_supported(&["SM4"]),
        // DES, RC2, RC5, … (legacy provider only), and unknown ciphers
        _ => EVP_UNSUPPORTED.into(),
    })
}

fn cbc_iv(params: &[u8]) -> Result<[u8; 16], TlsError> {
    // OpenSSL keeps a zero IV when the parameter is absent
    if params.is_empty() {
        return Ok([0; 16]);
    }
    match AnyRef::from_der(params) {
        Ok(iv) if iv.tag() == Tag::OctetString => {
            iv.value().try_into().map_err(|_| CIPHER_PARAMETER_ERROR)
        },
        _ => Err(DATA_IS_WRONG),
    }
}

type Pbkdf2 = fn(&[u8], &[u8], u32, &mut [u8]) -> Result<(), hmac::digest::InvalidLength>;

/// PBKDF2 with a PRF from OpenSSL's table.
fn pbkdf2_with(prf: &Oid) -> Result<Pbkdf2, BoxError> {
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
        oid::HMAC_SM3 => return Err(not_supported(&["HMAC-SM3"])),
        // In OpenSSL's table, without a digest in its default provider
        oid::HMAC_GOST_94 | oid::HMAC_GOST_2012_256 | oid::HMAC_GOST_2012_512 => {
            return Err(EVP_UNSUPPORTED.into())
        },
        _ => return Err(UNSUPPORTED_PRF.into()),
    })
}

fn aes_cbc_decrypt(key: &[u8], iv: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, TlsError> {
    fn decrypt<C: BlockCipherDecrypt + KeyInit>(
        key: &[u8],
        iv: &[u8],
        buf: &mut [u8],
    ) -> Option<usize> {
        let decryptor = cbc::Decryptor::<C>::new_from_slices(key, iv).ok()?;
        decryptor.decrypt_padded::<Pkcs7>(buf).ok().map(<[u8]>::len)
    }
    if data.is_empty() || !data.len().is_multiple_of(16) {
        return Err(WRONG_FINAL_BLOCK_LENGTH);
    }
    let mut buf = data.to_vec();
    let len = match key.len() {
        16 => decrypt::<aes::Aes128>(key, iv, &mut buf),
        24 => decrypt::<aes::Aes192>(key, iv, &mut buf),
        _ => decrypt::<aes::Aes256>(key, iv, &mut buf),
    };
    buf.truncate(len.ok_or(BAD_DECRYPT)?);
    Ok(buf)
}

/// A wrong passphrase leaves valid padding about once in 256 tries. OpenSSL
/// then fails to decode the key.
fn private_key_info(mut plain: Vec<u8>) -> Result<Vec<u8>, TlsError> {
    let trailing = match AnyRef::from_der_partial(&plain) {
        Ok((any, rest)) if any.tag() == Tag::Sequence => rest.len(),
        _ => return Err(DECODER_UNSUPPORTED),
    };
    plain.truncate(plain.len() - trailing);
    Ok(plain)
}
