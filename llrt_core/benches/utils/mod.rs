// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_encoding::*;
use rand::RngExt;

const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_bytes(len: usize) -> Vec<u8> {
    let mut rng = rand::rng();

    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..ASCII.len());
            ASCII[idx]
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn random_ascii(len: usize) -> String {
    let bytes = random_bytes(len);
    String::from_utf8(bytes).unwrap()
}

#[allow(dead_code)]
pub(crate) fn random_base64(len: usize) -> String {
    let bytes = random_bytes(len);
    bytes_to_b64_string(&bytes)
}

#[allow(dead_code)]
pub(crate) fn random_hex(len: usize) -> String {
    let bytes = random_bytes(len);
    bytes_to_hex_string(&bytes)
}
