// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use der::asn1::{AnyRef, OctetStringRef};
use der::{Decode, Sequence, Tag, Tagged};

use crate::pbes2::{self, AlgorithmIdentifier};
use crate::Error;

#[derive(Sequence)]
pub struct EncryptedPrivateKeyInfo<'a> {
    pub(crate) algorithm: AlgorithmIdentifier<'a>,
    pub(crate) data: &'a OctetStringRef,
}

impl<'a> EncryptedPrivateKeyInfo<'a> {
    /// Ignores data after the structure, as OpenSSL does.
    pub fn parse(der: &'a [u8]) -> Result<Self, Error> {
        Self::from_der_partial(der)
            .map(|(key, _)| key)
            .map_err(|_| Error::InvalidKey)
    }

    /// Decrypts to a DER `PrivateKeyInfo`.
    pub fn decrypt(&self, passphrase: &[u8]) -> Result<Vec<u8>, Error> {
        private_key_info(pbes2::decrypt(
            &self.algorithm,
            passphrase,
            self.data.as_bytes(),
        )?)
    }
}

/// A wrong passphrase leaves valid padding about once in 256 tries, and OpenSSL can't decode it.
fn private_key_info(mut plain: Vec<u8>) -> Result<Vec<u8>, Error> {
    let trailing = match AnyRef::from_der_partial(&plain) {
        Ok((any, rest)) if any.tag() == Tag::Sequence => rest.len(),
        _ => return Err(Error::InvalidKey),
    };
    plain.truncate(plain.len() - trailing);
    Ok(plain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{der, read};

    fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        [&[tag, value.len() as u8][..], value].concat()
    }

    #[test]
    fn parses_encrypted_private_key_info() {
        let pbes2 = tlv(
            0x30,
            &tlv(
                0x06,
                &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x05, 0x0d],
            ),
        );
        let key = |fields: &[&[u8]]| tlv(0x30, &[&pbes2[..], &fields.concat()].concat());
        let data = tlv(0x04, &[1, 2]);
        assert!(EncryptedPrivateKeyInfo::parse(&key(&[&data])).is_ok());
        // OpenSSL ignores data after the structure
        assert!(EncryptedPrivateKeyInfo::parse(&[key(&[&data]), vec![0, 0]].concat()).is_ok());
        let truncated = der(&read("client-encrypted.key"))[..40].to_vec();
        let cases = [
            key(&[&tlv(0x03, &[0, 1])]),
            key(&[&data, &tlv(0x02, &[1])]),
            truncated,
            b"not DER".to_vec(),
        ];
        for der in cases {
            assert_eq!(
                EncryptedPrivateKeyInfo::parse(&der).err(),
                Some(Error::InvalidKey)
            );
        }
    }

    #[test]
    fn checks_the_decrypted_key() {
        let key = [0x30, 0x03, 0x02, 0x01, 0x00];
        assert_eq!(private_key_info(key.to_vec()), Ok(key.to_vec()));
        // OpenSSL ignores data after the key
        assert_eq!(
            private_key_info([&key[..], &[0]].concat()),
            Ok(key.to_vec())
        );
        for plain in [&b"not DER"[..], b"", &[0x02, 0x01, 0x00]] {
            assert_eq!(private_key_info(plain.to_vec()), Err(Error::InvalidKey));
        }
    }

    #[cfg(feature = "hmac-sha2")]
    #[test]
    fn decrypts() {
        let der = der(&read("ec-encrypted-empty.key"));
        let key = EncryptedPrivateKeyInfo::parse(&der).unwrap();
        assert_eq!(key.decrypt(b""), Ok(self::der(&read("ec.key"))));
        assert_eq!(key.decrypt(b"x"), Err(Error::BadDecrypt));
    }
}
