// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! OpenSSL crypto provider - uses OpenSSL for cryptographic operations.

use openssl::bn::BigNum;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::hash::{Hasher, MessageDigest};
use openssl::md::Md;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey};
use openssl::pkey_ctx::PkeyCtx;
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{Signer, Verifier};
use openssl::symm::{self, Cipher};

use crate::provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest};
use crate::sha_hash::ShaAlgorithm;
use crate::subtle::EllipticCurve;

pub struct OpenSslProvider;

pub enum OpenSslDigest {
    Md5(Hasher),
    Sha1(Hasher),
    Sha256(Hasher),
    Sha384(Hasher),
    Sha512(Hasher),
}

impl SimpleDigest for OpenSslDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            OpenSslDigest::Md5(h)
            | OpenSslDigest::Sha1(h)
            | OpenSslDigest::Sha256(h)
            | OpenSslDigest::Sha384(h)
            | OpenSslDigest::Sha512(h) => {
                let _ = h.update(data);
            },
        }
    }

    fn finalize(mut self) -> Vec<u8> {
        match self {
            OpenSslDigest::Md5(ref mut h)
            | OpenSslDigest::Sha1(ref mut h)
            | OpenSslDigest::Sha256(ref mut h)
            | OpenSslDigest::Sha384(ref mut h)
            | OpenSslDigest::Sha512(ref mut h) => {
                h.finish().map(|d| d.to_vec()).unwrap_or_default()
            },
        }
    }
}

pub struct OpenSslHmac {
    signer: Signer<'static>,
}

impl HmacProvider for OpenSslHmac {
    fn update(&mut self, data: &[u8]) {
        let _ = self.signer.update(data);
    }

    fn finalize(self) -> Vec<u8> {
        self.signer.sign_to_vec().unwrap_or_default()
    }
}

fn get_message_digest(alg: ShaAlgorithm) -> MessageDigest {
    match alg {
        ShaAlgorithm::MD5 => MessageDigest::md5(),
        ShaAlgorithm::SHA1 => MessageDigest::sha1(),
        ShaAlgorithm::SHA256 => MessageDigest::sha256(),
        ShaAlgorithm::SHA384 => MessageDigest::sha384(),
        ShaAlgorithm::SHA512 => MessageDigest::sha512(),
    }
}

fn get_md(alg: ShaAlgorithm) -> &'static openssl::md::MdRef {
    match alg {
        ShaAlgorithm::MD5 => Md::md5(),
        ShaAlgorithm::SHA1 => Md::sha1(),
        ShaAlgorithm::SHA256 => Md::sha256(),
        ShaAlgorithm::SHA384 => Md::sha384(),
        ShaAlgorithm::SHA512 => Md::sha512(),
    }
}

fn curve_to_nid(curve: EllipticCurve) -> Nid {
    match curve {
        EllipticCurve::P256 => Nid::X9_62_PRIME256V1,
        EllipticCurve::P384 => Nid::SECP384R1,
        EllipticCurve::P521 => Nid::SECP521R1,
    }
}

fn get_ec_group(curve: EllipticCurve) -> Result<EcGroup, CryptoError> {
    let nid = curve_to_nid(curve);
    EcGroup::from_curve_name(nid)
        .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))
}

impl CryptoProvider for OpenSslProvider {
    type Digest = OpenSslDigest;
    type Hmac = OpenSslHmac;

    fn digest(&self, algorithm: ShaAlgorithm) -> Self::Digest {
        let md = get_message_digest(algorithm);
        let hasher = Hasher::new(md).expect("Failed to create hasher");
        match algorithm {
            ShaAlgorithm::MD5 => OpenSslDigest::Md5(hasher),
            ShaAlgorithm::SHA1 => OpenSslDigest::Sha1(hasher),
            ShaAlgorithm::SHA256 => OpenSslDigest::Sha256(hasher),
            ShaAlgorithm::SHA384 => OpenSslDigest::Sha384(hasher),
            ShaAlgorithm::SHA512 => OpenSslDigest::Sha512(hasher),
        }
    }

    fn hmac(&self, algorithm: ShaAlgorithm, key: &[u8]) -> Self::Hmac {
        let md = get_message_digest(algorithm);
        let pkey = PKey::hmac(key).expect("Failed to create HMAC key");
        let signer = unsafe {
            std::mem::transmute::<Signer<'_>, Signer<'static>>(
                Signer::new(md, &pkey).expect("Failed to create signer"),
            )
        };
        OpenSslHmac { signer }
    }

    fn ecdsa_sign(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        digest: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let group = get_ec_group(curve)?;
        let ec_key = EcKey::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let sig = EcdsaSig::sign(digest, &ec_key)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        let r = sig.r().to_vec();
        let s = sig.s().to_vec();
        let coord_len = (group.degree() as usize).div_ceil(8);
        let mut result = vec![0u8; coord_len * 2];
        result[coord_len - r.len()..coord_len].copy_from_slice(&r);
        result[coord_len * 2 - s.len()..].copy_from_slice(&s);
        Ok(result)
    }

    fn ecdsa_verify(
        &self,
        curve: EllipticCurve,
        public_key_sec1: &[u8],
        signature: &[u8],
        digest: &[u8],
    ) -> Result<bool, CryptoError> {
        let group = get_ec_group(curve)?;
        let ec_key = EcKey::public_key_from_der(public_key_sec1).or_else(|_| {
            let point = openssl::ec::EcPoint::from_bytes(
                &group,
                public_key_sec1,
                &mut openssl::bn::BigNumContext::new().unwrap(),
            )
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            EcKey::from_public_key(&group, &point)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
        })?;
        let coord_len = signature.len() / 2;
        let r = BigNum::from_slice(&signature[..coord_len])
            .map_err(|e| CryptoError::InvalidSignature(Some(e.to_string().into())))?;
        let s = BigNum::from_slice(&signature[coord_len..])
            .map_err(|e| CryptoError::InvalidSignature(Some(e.to_string().into())))?;
        let sig = EcdsaSig::from_private_components(r, s)
            .map_err(|e| CryptoError::InvalidSignature(Some(e.to_string().into())))?;
        Ok(sig.verify(digest, &ec_key).unwrap_or(false))
    }

    fn ed25519_sign(&self, private_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let pkey = PKey::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut signer = Signer::new_without_digest(&pkey)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .sign_oneshot_to_vec(data)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))
    }

    fn ed25519_verify(
        &self,
        public_key_bytes: &[u8],
        signature: &[u8],
        data: &[u8],
    ) -> Result<bool, CryptoError> {
        let pkey = PKey::public_key_from_raw_bytes(public_key_bytes, Id::ED25519)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut verifier = Verifier::new_without_digest(&pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(verifier.verify_oneshot(signature, data).unwrap_or(false))
    }

    fn rsa_pss_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let md = get_message_digest(hash_alg);
        let mut signer = Signer::new(md, &pkey)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .set_rsa_padding(Padding::PKCS1_PSS)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .set_rsa_pss_saltlen(openssl::sign::RsaPssSaltlen::custom(salt_length as i32))
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .set_rsa_mgf1_md(md)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .update(digest)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .sign_to_vec()
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))
    }

    fn rsa_pss_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        let rsa = Rsa::public_key_from_der(public_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let md = get_message_digest(hash_alg);
        let mut verifier = Verifier::new(md, &pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .set_rsa_padding(Padding::PKCS1_PSS)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .set_rsa_pss_saltlen(openssl::sign::RsaPssSaltlen::custom(salt_length as i32))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .set_rsa_mgf1_md(md)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .update(digest)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(verifier.verify(signature).unwrap_or(false))
    }

    fn rsa_pkcs1v15_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let md = get_message_digest(hash_alg);
        let mut signer = Signer::new(md, &pkey)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .update(digest)
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))?;
        signer
            .sign_to_vec()
            .map_err(|e| CryptoError::SigningFailed(Some(e.to_string().into())))
    }

    fn rsa_pkcs1v15_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        let rsa = Rsa::public_key_from_der(public_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let md = get_message_digest(hash_alg);
        let mut verifier = Verifier::new(md, &pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        verifier
            .update(digest)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(verifier.verify(signature).unwrap_or(false))
    }

    fn rsa_oaep_encrypt(
        &self,
        public_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::public_key_from_der(public_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut ctx = PkeyCtx::new(&pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.encrypt_init()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_padding(Padding::PKCS1_OAEP)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_oaep_md(get_md(hash_alg))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_mgf1_md(get_md(hash_alg))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        if let Some(lbl) = label {
            ctx.set_rsa_oaep_label(lbl)
                .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        }
        let mut out = vec![0u8; pkey.size()];
        let len = ctx
            .encrypt(data, Some(&mut out))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        out.truncate(len);
        Ok(out)
    }

    fn rsa_oaep_decrypt(
        &self,
        private_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut ctx = PkeyCtx::new(&pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.decrypt_init()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_padding(Padding::PKCS1_OAEP)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_oaep_md(get_md(hash_alg))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        ctx.set_rsa_mgf1_md(get_md(hash_alg))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        if let Some(lbl) = label {
            ctx.set_rsa_oaep_label(lbl)
                .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        }
        let mut out = vec![0u8; pkey.size()];
        let len = ctx
            .decrypt(data, Some(&mut out))
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        out.truncate(len);
        Ok(out)
    }

    fn ecdh_derive_bits(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        public_key_sec1: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let group = get_ec_group(curve)?;
        let private_ec = EcKey::private_key_from_der(private_key_der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let private_pkey = PKey::from_ec_key(private_ec)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let public_ec = EcKey::public_key_from_der(public_key_sec1).or_else(|_| {
            let point = openssl::ec::EcPoint::from_bytes(
                &group,
                public_key_sec1,
                &mut openssl::bn::BigNumContext::new().unwrap(),
            )
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            EcKey::from_public_key(&group, &point)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
        })?;
        let public_pkey = PKey::from_ec_key(public_ec)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut deriver = Deriver::new(&private_pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        deriver
            .set_peer(&public_pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        deriver
            .derive_to_vec()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))
    }

    fn x25519_derive_bits(
        &self,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let private_pkey = PKey::private_key_from_raw_bytes(private_key, Id::X25519)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let public_pkey = PKey::public_key_from_raw_bytes(public_key, Id::X25519)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut deriver = Deriver::new(&private_pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        deriver
            .set_peer(&public_pkey)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        deriver
            .derive_to_vec()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))
    }

    fn aes_encrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        match mode {
            AesMode::Cbc => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_cbc(),
                    24 => Cipher::aes_192_cbc(),
                    32 => Cipher::aes_256_cbc(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                symm::encrypt(cipher, key, Some(iv), data)
                    .map_err(|e| CryptoError::EncryptionFailed(Some(e.to_string().into())))
            },
            AesMode::Ctr { .. } => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_ctr(),
                    24 => Cipher::aes_192_ctr(),
                    32 => Cipher::aes_256_ctr(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                symm::encrypt(cipher, key, Some(iv), data)
                    .map_err(|e| CryptoError::EncryptionFailed(Some(e.to_string().into())))
            },
            AesMode::Gcm { tag_length } => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_gcm(),
                    24 => Cipher::aes_192_gcm(),
                    32 => Cipher::aes_256_gcm(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                let tag_len = (tag_length / 8) as usize;
                let mut tag = vec![0u8; tag_len];
                let ciphertext = symm::encrypt_aead(
                    cipher,
                    key,
                    Some(iv),
                    additional_data.unwrap_or(&[]),
                    data,
                    &mut tag,
                )
                .map_err(|e| CryptoError::EncryptionFailed(Some(e.to_string().into())))?;
                let mut result = ciphertext;
                result.extend_from_slice(&tag);
                Ok(result)
            },
        }
    }

    fn aes_decrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        match mode {
            AesMode::Cbc => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_cbc(),
                    24 => Cipher::aes_192_cbc(),
                    32 => Cipher::aes_256_cbc(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                symm::decrypt(cipher, key, Some(iv), data)
                    .map_err(|e| CryptoError::DecryptionFailed(Some(e.to_string().into())))
            },
            AesMode::Ctr { .. } => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_ctr(),
                    24 => Cipher::aes_192_ctr(),
                    32 => Cipher::aes_256_ctr(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                symm::decrypt(cipher, key, Some(iv), data)
                    .map_err(|e| CryptoError::DecryptionFailed(Some(e.to_string().into())))
            },
            AesMode::Gcm { tag_length } => {
                let cipher = match key.len() {
                    16 => Cipher::aes_128_gcm(),
                    24 => Cipher::aes_192_gcm(),
                    32 => Cipher::aes_256_gcm(),
                    _ => {
                        return Err(CryptoError::InvalidKey(Some(
                            "Invalid AES key length".into(),
                        )))
                    },
                };
                let tag_len = (tag_length / 8) as usize;
                if data.len() < tag_len {
                    return Err(CryptoError::InvalidData(Some(
                        "Data too short for GCM tag".into(),
                    )));
                }
                let (ciphertext, tag) = data.split_at(data.len() - tag_len);
                symm::decrypt_aead(
                    cipher,
                    key,
                    Some(iv),
                    additional_data.unwrap_or(&[]),
                    ciphertext,
                    tag,
                )
                .map_err(|e| CryptoError::DecryptionFailed(Some(e.to_string().into())))
            },
        }
    }

    fn aes_kw_wrap(&self, kek: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        use openssl::aes::{wrap_key, AesKey};
        let aes_key = AesKey::new_encrypt(kek).map_err(|_| CryptoError::InvalidKey(None))?;
        let mut out = vec![0u8; key.len() + 8];
        wrap_key(&aes_key, None, &mut out, key).map_err(|_| CryptoError::OperationFailed(None))?;
        Ok(out)
    }

    fn aes_kw_unwrap(&self, kek: &[u8], wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        use openssl::aes::{unwrap_key, AesKey};
        let aes_key = AesKey::new_decrypt(kek).map_err(|_| CryptoError::InvalidKey(None))?;
        let mut out = vec![0u8; wrapped_key.len() - 8];
        unwrap_key(&aes_key, None, &mut out, wrapped_key)
            .map_err(|_| CryptoError::OperationFailed(None))?;
        Ok(out)
    }

    fn hkdf_derive_key(
        &self,
        key: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        use openssl::pkey_ctx::HkdfMode;
        let md = get_md(hash_alg);
        let mut ctx = PkeyCtx::new_id(Id::HKDF)
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        ctx.derive_init()
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        ctx.set_hkdf_md(md)
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        ctx.set_hkdf_mode(HkdfMode::EXTRACT_THEN_EXPAND)
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        ctx.set_hkdf_key(key)
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        if !salt.is_empty() {
            ctx.set_hkdf_salt(salt)
                .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        }
        if !info.is_empty() {
            ctx.add_hkdf_info(info)
                .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        }
        let mut out = vec![0u8; length];
        ctx.derive(Some(&mut out))
            .map_err(|e| CryptoError::DerivationFailed(Some(e.to_string().into())))?;
        Ok(out)
    }

    fn pbkdf2_derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let md = get_message_digest(hash_alg);
        let mut out = vec![0u8; length];
        openssl::pkcs5::pbkdf2_hmac(password, salt, iterations as usize, md, &mut out)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(out)
    }

    fn generate_aes_key(&self, length_bits: u16) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = (length_bits / 8) as usize;
        let mut key = vec![0u8; length_bytes];
        rand_bytes(&mut key)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(key)
    }

    fn generate_hmac_key(
        &self,
        hash_alg: ShaAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = if length_bits == 0 {
            match hash_alg {
                ShaAlgorithm::MD5 => 16,
                ShaAlgorithm::SHA1 => 20,
                ShaAlgorithm::SHA256 => 32,
                ShaAlgorithm::SHA384 => 48,
                ShaAlgorithm::SHA512 => 64,
            }
        } else {
            (length_bits / 8) as usize
        };
        let mut key = vec![0u8; length_bytes];
        rand_bytes(&mut key)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok(key)
    }

    fn generate_ec_key(&self, curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let group = get_ec_group(curve)?;
        let ec_key = EcKey::generate(&group)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let pkey = PKey::from_ec_key(ec_key.clone())
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        // Return PKCS#8 DER for private key (consistent with RustCrypto)
        let private_der = pkey
            .private_key_to_der()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        // Return SEC1 uncompressed point for public key (consistent with RustCrypto)
        let mut bn_ctx = openssl::bn::BigNumContext::new()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let public_sec1 = ec_key
            .public_key()
            .to_bytes(
                &group,
                openssl::ec::PointConversionForm::UNCOMPRESSED,
                &mut bn_ctx,
            )
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok((private_der, public_sec1))
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let pkey = PKey::generate_ed25519()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let private_der = pkey
            .private_key_to_der()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let public_raw = pkey
            .raw_public_key()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok((private_der, public_raw))
    }

    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let pkey = PKey::generate_x25519()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let private_raw = pkey
            .raw_private_key()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let public_raw = pkey
            .raw_public_key()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok((private_raw, public_raw))
    }

    fn generate_rsa_key(
        &self,
        modulus_length: u32,
        public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let exp = BigNum::from_slice(public_exponent)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let rsa = Rsa::generate_with_e(modulus_length, &exp)
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let private_der = rsa
            .private_key_to_der()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        let public_der = rsa
            .public_key_to_der()
            .map_err(|e| CryptoError::OperationFailed(Some(e.to_string().into())))?;
        Ok((private_der, public_der))
    }

    fn import_rsa_public_key_pkcs1(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        let rsa = Rsa::public_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let modulus_length = rsa.n().num_bits() as u32;
        let public_exponent = rsa.e().to_vec();
        let key_data = rsa
            .public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length,
            public_exponent,
            is_private: false,
        })
    }

    fn import_rsa_private_key_pkcs1(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        let rsa = Rsa::private_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let modulus_length = rsa.n().num_bits() as u32;
        let public_exponent = rsa.e().to_vec();
        let key_data = rsa
            .private_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length,
            public_exponent,
            is_private: true,
        })
    }

    fn import_rsa_public_key_spki(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        let pkey = PKey::public_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let rsa = pkey
            .rsa()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let modulus_length = rsa.n().num_bits() as u32;
        let public_exponent = rsa.e().to_vec();
        let key_data = rsa
            .public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length,
            public_exponent,
            is_private: false,
        })
    }

    fn import_rsa_private_key_pkcs8(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        let pkey = PKey::private_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let rsa = pkey
            .rsa()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let modulus_length = rsa.n().num_bits() as u32;
        let public_exponent = rsa.e().to_vec();
        let key_data = rsa
            .private_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length,
            public_exponent,
            is_private: true,
        })
    }

    fn export_rsa_public_key_pkcs1(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::public_key_from_der(key_data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        rsa.public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn export_rsa_public_key_spki(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::public_key_from_der(key_data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        pkey.public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn export_rsa_private_key_pkcs8(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let rsa = Rsa::private_key_from_der(key_data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey =
            PKey::from_rsa(rsa).map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        pkey.private_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn import_ec_public_key_sec1(
        &self,
        data: &[u8],
        curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        let nid = curve_to_nid(curve);
        let group = EcGroup::from_curve_name(nid)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut ctx = openssl::bn::BigNumContext::new()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let point = openssl::ec::EcPoint::from_bytes(&group, data, &mut ctx)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let ec_key = EcKey::from_public_key(&group, &point)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey = PKey::from_ec_key(ec_key)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::EcImportResult {
            key_data: pkey
                .public_key_to_der()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
            is_private: false,
        })
    }

    fn import_ec_public_key_spki(&self, der: &[u8]) -> Result<super::EcImportResult, CryptoError> {
        let pkey = PKey::public_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::EcImportResult {
            key_data: pkey
                .public_key_to_der()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
            is_private: false,
        })
    }

    fn import_ec_private_key_pkcs8(
        &self,
        der: &[u8],
    ) -> Result<super::EcImportResult, CryptoError> {
        let pkey = PKey::private_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::EcImportResult {
            key_data: pkey
                .private_key_to_der()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
            is_private: true,
        })
    }

    fn import_ec_private_key_sec1(
        &self,
        data: &[u8],
        curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        let nid = curve_to_nid(curve);
        let group = EcGroup::from_curve_name(nid)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let bn = BigNum::from_slice(data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let ec_key = EcKey::from_private_components(&group, &bn, group.generator())
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pkey = PKey::from_ec_key(ec_key)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::EcImportResult {
            key_data: pkey
                .private_key_to_der()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
            is_private: true,
        })
    }

    fn export_ec_public_key_sec1(
        &self,
        key_data: &[u8],
        _curve: EllipticCurve,
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut ctx = openssl::bn::BigNumContext::new()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        if is_private {
            let ec_key = PKey::private_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?
                .ec_key()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            ec_key
                .public_key()
                .to_bytes(
                    ec_key.group(),
                    openssl::ec::PointConversionForm::UNCOMPRESSED,
                    &mut ctx,
                )
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
        } else {
            let ec_key = PKey::public_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?
                .ec_key()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            ec_key
                .public_key()
                .to_bytes(
                    ec_key.group(),
                    openssl::ec::PointConversionForm::UNCOMPRESSED,
                    &mut ctx,
                )
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
        }
    }

    fn export_ec_public_key_spki(
        &self,
        key_data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        let pkey = PKey::public_key_from_der(key_data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        pkey.public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn export_ec_private_key_pkcs8(
        &self,
        key_data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        let pkey = PKey::private_key_from_der(key_data)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        pkey.private_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn import_okp_public_key_raw(
        &self,
        data: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        if data.len() != 32 {
            return Err(CryptoError::InvalidKey(None));
        }
        Ok(super::OkpImportResult {
            key_data: data.to_vec(),
            is_private: false,
        })
    }

    fn import_okp_public_key_spki(
        &self,
        der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        let pkey = PKey::public_key_from_der(der)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let raw = pkey
            .raw_public_key()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        Ok(super::OkpImportResult {
            key_data: raw,
            is_private: false,
        })
    }

    fn import_okp_private_key_pkcs8(
        &self,
        der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        Ok(super::OkpImportResult {
            key_data: der.to_vec(),
            is_private: true,
        })
    }

    fn export_okp_public_key_raw(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        if is_private {
            let pkey = PKey::private_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            pkey.raw_public_key()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
        } else {
            Ok(key_data.to_vec())
        }
    }

    fn export_okp_public_key_spki(
        &self,
        key_data: &[u8],
        _oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // key_data is raw public key, need to wrap in SPKI
        let pkey = PKey::public_key_from_raw_bytes(key_data, Id::ED25519)
            .or_else(|_| PKey::public_key_from_raw_bytes(key_data, Id::X25519))
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        pkey.public_key_to_der()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))
    }

    fn export_okp_private_key_pkcs8(
        &self,
        key_data: &[u8],
        _oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // key_data is already PKCS8
        Ok(key_data.to_vec())
    }

    fn import_rsa_jwk(
        &self,
        jwk: super::RsaJwkImport<'_>,
    ) -> Result<super::RsaImportResult, CryptoError> {
        let n = BigNum::from_slice(jwk.n)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let e = BigNum::from_slice(jwk.e)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let modulus_length = n.num_bits() as u32;
        let pub_exp_bytes = jwk.e.to_vec();

        if let (
            Some(d_bytes),
            Some(p_bytes),
            Some(q_bytes),
            Some(dp_bytes),
            Some(dq_bytes),
            Some(qi_bytes),
        ) = (jwk.d, jwk.p, jwk.q, jwk.dp, jwk.dq, jwk.qi)
        {
            let d = BigNum::from_slice(d_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let p = BigNum::from_slice(p_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let q = BigNum::from_slice(q_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let dp = BigNum::from_slice(dp_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let dq = BigNum::from_slice(dq_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let qi = BigNum::from_slice(qi_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;

            let rsa = Rsa::from_private_components(n, e, d, p, q, dp, dq, qi)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let pkey = PKey::from_rsa(rsa)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::RsaImportResult {
                key_data: pkey
                    .private_key_to_der()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
                modulus_length,
                public_exponent: pub_exp_bytes,
                is_private: true,
            })
        } else {
            let rsa = Rsa::from_public_components(n, e)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let pkey = PKey::from_rsa(rsa)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::RsaImportResult {
                key_data: pkey
                    .public_key_to_der()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
                modulus_length,
                public_exponent: pub_exp_bytes,
                is_private: false,
            })
        }
    }

    fn export_rsa_jwk(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<super::RsaJwkExport, CryptoError> {
        if is_private {
            let pkey = PKey::private_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let rsa = pkey
                .rsa()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::RsaJwkExport {
                n: rsa.n().to_vec(),
                e: rsa.e().to_vec(),
                d: Some(rsa.d().to_vec()),
                p: rsa.p().map(|v| v.to_vec()),
                q: rsa.q().map(|v| v.to_vec()),
                dp: rsa.dmp1().map(|v| v.to_vec()),
                dq: rsa.dmq1().map(|v| v.to_vec()),
                qi: rsa.iqmp().map(|v| v.to_vec()),
            })
        } else {
            let pkey = PKey::public_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let rsa = pkey
                .rsa()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::RsaJwkExport {
                n: rsa.n().to_vec(),
                e: rsa.e().to_vec(),
                d: None,
                p: None,
                q: None,
                dp: None,
                dq: None,
                qi: None,
            })
        }
    }

    fn import_ec_jwk(
        &self,
        jwk: super::EcJwkImport<'_>,
        curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        let nid = curve_to_nid(curve);
        let group = EcGroup::from_curve_name(nid)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let x = BigNum::from_slice(jwk.x)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let y = BigNum::from_slice(jwk.y)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let pub_key = EcKey::from_public_key_affine_coordinates(&group, &x, &y)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;

        if let Some(d_bytes) = jwk.d {
            let d = BigNum::from_slice(d_bytes)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let priv_key = EcKey::from_private_components(&group, &d, pub_key.public_key())
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let pkey = PKey::from_ec_key(priv_key)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::EcImportResult {
                key_data: pkey
                    .private_key_to_der()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
                is_private: true,
            })
        } else {
            // Return SEC1 uncompressed point for public key (consistent with generate_ec_key)
            let mut ctx = openssl::bn::BigNumContext::new()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let sec1 = pub_key
                .public_key()
                .to_bytes(
                    &group,
                    openssl::ec::PointConversionForm::UNCOMPRESSED,
                    &mut ctx,
                )
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::EcImportResult {
                key_data: sec1,
                is_private: false,
            })
        }
    }

    fn export_ec_jwk(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
        is_private: bool,
    ) -> Result<super::EcJwkExport, CryptoError> {
        let nid = curve_to_nid(curve);
        let group = EcGroup::from_curve_name(nid)
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
        let mut ctx = openssl::bn::BigNumContext::new()
            .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;

        if is_private {
            let pkey = PKey::private_key_from_der(key_data)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let ec_key = pkey
                .ec_key()
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let mut x =
                BigNum::new().map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            let mut y =
                BigNum::new().map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            ec_key
                .public_key()
                .affine_coordinates(&group, &mut x, &mut y, &mut ctx)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            Ok(super::EcJwkExport {
                x: x.to_vec(),
                y: y.to_vec(),
                d: Some(ec_key.private_key().to_vec()),
            })
        } else {
            // key_data is SEC1 uncompressed point (0x04 || x || y)
            let coord_len = match curve {
                EllipticCurve::P256 => 32,
                EllipticCurve::P384 => 48,
                EllipticCurve::P521 => 66,
            };
            if key_data.len() != 1 + 2 * coord_len || key_data[0] != 0x04 {
                return Err(CryptoError::InvalidKey(None));
            }
            let x = key_data[1..1 + coord_len].to_vec();
            let y = key_data[1 + coord_len..].to_vec();
            Ok(super::EcJwkExport { x, y, d: None })
        }
    }

    fn import_okp_jwk(
        &self,
        jwk: super::OkpJwkImport<'_>,
        is_ed25519: bool,
    ) -> Result<super::OkpImportResult, CryptoError> {
        let id = if is_ed25519 { Id::ED25519 } else { Id::X25519 };
        if let Some(d) = jwk.d {
            // Private key
            let pkey = PKey::private_key_from_raw_bytes(d, id)
                .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
            if is_ed25519 {
                // Ed25519: return PKCS8 DER
                Ok(super::OkpImportResult {
                    key_data: pkey
                        .private_key_to_der()
                        .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?,
                    is_private: true,
                })
            } else {
                // X25519: return raw bytes
                Ok(super::OkpImportResult {
                    key_data: d.to_vec(),
                    is_private: true,
                })
            }
        } else {
            // Public key - store raw bytes
            Ok(super::OkpImportResult {
                key_data: jwk.x.to_vec(),
                is_private: false,
            })
        }
    }

    fn export_okp_jwk(
        &self,
        key_data: &[u8],
        is_private: bool,
        is_ed25519: bool,
    ) -> Result<super::OkpJwkExport, CryptoError> {
        if is_private {
            if is_ed25519 {
                // Ed25519: key_data is PKCS8 DER
                let pkey = PKey::private_key_from_der(key_data)
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
                let d = pkey
                    .raw_private_key()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
                let x = pkey
                    .raw_public_key()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
                Ok(super::OkpJwkExport { x, d: Some(d) })
            } else {
                // X25519: key_data is raw 32-byte secret
                let pkey = PKey::private_key_from_raw_bytes(key_data, Id::X25519)
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
                let x = pkey
                    .raw_public_key()
                    .map_err(|e| CryptoError::InvalidKey(Some(e.to_string().into())))?;
                Ok(super::OkpJwkExport {
                    x,
                    d: Some(key_data.to_vec()),
                })
            }
        } else {
            // Public key - key_data is raw bytes
            Ok(super::OkpJwkExport {
                x: key_data.to_vec(),
                d: None,
            })
        }
    }
}
