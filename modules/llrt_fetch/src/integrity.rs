// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Subresource Integrity (SRI) parsing and verification, per
//! <https://www.w3.org/TR/SRI/>.
//!
//! `fetch(url, { integrity: "sha256-..." })` causes the fetch machinery to
//! hash the response body and compare against one of the listed hashes. A
//! mismatch rejects the fetch promise with a `TypeError`.

use llrt_encoding::bytes_from_b64;
use sha2::{Digest, Sha256, Sha384, Sha512};

#[derive(Clone, Copy, Debug)]
pub enum SriAlgorithm {
    Sha256,
    Sha384,
    Sha512,
}

impl SriAlgorithm {
    /// Relative strength rank per SRI spec. Larger is stronger.
    fn strength(self) -> u8 {
        match self {
            SriAlgorithm::Sha256 => 1,
            SriAlgorithm::Sha384 => 2,
            SriAlgorithm::Sha512 => 3,
        }
    }

    fn hash(self, data: &[u8]) -> Vec<u8> {
        match self {
            SriAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
            SriAlgorithm::Sha384 => Sha384::digest(data).to_vec(),
            SriAlgorithm::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

#[derive(Debug)]
pub struct SriEntry {
    pub algorithm: SriAlgorithm,
    pub expected: Vec<u8>,
}

/// Parse an integrity metadata string into its (algorithm, hash) entries.
/// Returns an empty list for empty / whitespace-only input (per SRI spec
/// this is "no integrity metadata" — the response is always accepted).
/// Unrecognized or malformed entries are silently skipped; the caller
/// treats an all-empty parse the same as no integrity.
pub fn parse_integrity(input: &str) -> Vec<SriEntry> {
    let mut out = Vec::new();
    for token in input.split_ascii_whitespace() {
        // Each token: `<algo>-<base64hash>` with optional `?opt=value`
        // query params we ignore.
        let (algo_hash, _opts) = match token.split_once('?') {
            Some((a, o)) => (a, Some(o)),
            None => (token, None),
        };
        let Some((algo, hash_b64)) = algo_hash.split_once('-') else {
            continue;
        };
        let algorithm = match algo {
            "sha256" => SriAlgorithm::Sha256,
            "sha384" => SriAlgorithm::Sha384,
            "sha512" => SriAlgorithm::Sha512,
            _ => continue,
        };
        // Accept both standard base64 (`+/`) and base64url (`-_`). Convert
        // url-safe chars to standard before decoding.
        let normalized: String = hash_b64
            .chars()
            .map(|c| match c {
                '-' => '+',
                '_' => '/',
                c => c,
            })
            .collect();
        let Ok(expected) = bytes_from_b64(normalized.as_bytes()) else {
            continue;
        };
        out.push(SriEntry {
            algorithm,
            expected,
        });
    }
    out
}

/// Returns `true` if `data` matches at least one of the SRI entries after
/// selecting the strongest algorithm (sha512 > sha384 > sha256) per the
/// "strongest metadata" rule of the SRI spec. Entries of other algorithms
/// are ignored once a stronger one is present. If `entries` is empty,
/// returns `true` (no integrity metadata = accept).
pub fn verify(entries: &[SriEntry], data: &[u8]) -> bool {
    if entries.is_empty() {
        return true;
    }
    // Pick the strongest algorithm that has at least one entry.
    let strongest = entries
        .iter()
        .map(|e| e.algorithm.strength())
        .max()
        .unwrap();
    for entry in entries {
        if entry.algorithm.strength() != strongest {
            continue;
        }
        let actual = entry.algorithm.hash(data);
        if actual == entry.expected {
            return true;
        }
    }
    false
}
