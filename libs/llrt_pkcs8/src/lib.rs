// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use der::asn1::{AnyRef, OctetStringRef};
use der::{Decode, Sequence, Tag, Tagged};
use llrt_pbe::AlgorithmIdentifier;

#[derive(Sequence)]
pub struct EncryptedPrivateKeyInfo<'a> {
    algorithm: AlgorithmIdentifier<'a>,
    data: &'a OctetStringRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The DER doesn't decode, before or after decryption.
    Decode,
    Decrypt(llrt_pbe::Error),
}

impl<'a> EncryptedPrivateKeyInfo<'a> {
    /// Ignores data after the structure, as OpenSSL does.
    pub fn parse(der: &'a [u8]) -> Result<Self, Error> {
        Self::from_der_partial(der)
            .map(|(key, _)| key)
            .map_err(|_| Error::Decode)
    }

    /// Decrypts to a DER `PrivateKeyInfo`.
    pub fn decrypt(&self, passphrase: &[u8]) -> Result<Vec<u8>, Error> {
        let plain = llrt_pbe::decrypt(&self.algorithm, passphrase, self.data.as_bytes())
            .map_err(Error::Decrypt)?;
        private_key_info(plain)
    }
}

/// A wrong passphrase leaves valid padding about once in 256 tries, and OpenSSL can't decode it.
fn private_key_info(mut plain: Vec<u8>) -> Result<Vec<u8>, Error> {
    let trailing = match AnyRef::from_der_partial(&plain) {
        Ok((any, rest)) if any.tag() == Tag::Sequence => rest.len(),
        _ => return Err(Error::Decode),
    };
    plain.truncate(plain.len() - trailing);
    Ok(plain)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const PASSPHRASE: &[u8] = "pässphrase".as_bytes();

    fn read(name: &str) -> String {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../llrt_test_tls/data");
        std::fs::read_to_string(data.join(name)).unwrap()
    }

    fn der(pem: &str) -> Vec<u8> {
        llrt_pem::first_private_key_block(pem.as_bytes())
            .unwrap()
            .decode()
            .unwrap()
    }

    fn decrypt(der: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, Error> {
        EncryptedPrivateKeyInfo::parse(der)?.decrypt(passphrase)
    }

    #[test]
    fn decrypts_every_prf_and_aes_key_size() {
        let expected = der(&read("ec.key"));
        let fixtures = read("ec-encrypted.pem");
        let blocks: Vec<&str> = fixtures
            .split_inclusive("-----END ENCRYPTED PRIVATE KEY-----")
            .filter(|block| block.trim_start().starts_with("aes-"))
            .collect();
        assert_eq!(blocks.len(), 14);
        for block in blocks {
            assert_eq!(
                decrypt(&der(block), PASSPHRASE).unwrap(),
                expected,
                "{block}"
            );
            if block.contains("hmac-sha1") {
                // hmacWithSHA1 named explicitly, which OpenSSL never writes
                let mut der = der(block);
                let ipsec = [0x2b, 0x06, 0x01, 0x05, 0x05, 0x08, 0x01, 0x02];
                let at = der.windows(8).position(|w| w == ipsec).unwrap();
                der[at..at + 8].copy_from_slice(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x02, 0x07]);
                assert_eq!(decrypt(&der, PASSPHRASE).unwrap(), expected);
            }
        }
        let empty = der(&read("ec-encrypted-empty.key"));
        assert_eq!(decrypt(&empty, b"").unwrap(), expected);
        assert_eq!(
            decrypt(&empty, b"x"),
            Err(Error::Decrypt(llrt_pbe::Error::BadDecrypt))
        );
    }
}
