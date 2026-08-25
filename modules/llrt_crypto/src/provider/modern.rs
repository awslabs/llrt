// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use chacha20poly1305::{
    aead::{Aead, Payload},
    ChaCha20Poly1305, KeyInit, Nonce,
};

use super::CryptoError;

pub(crate) fn chacha20_poly1305_encrypt(
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey(None))?;
    let nonce = Nonce::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: data,
                aad: additional_data.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed(None))
}

pub(crate) fn chacha20_poly1305_decrypt(
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey(None))?;
    let nonce = Nonce::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: data,
                aad: additional_data.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed(None))
}
