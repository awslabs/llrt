// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Lazy-loaded historical timezone transition data.
//!
//! Historical data is stored compressed and only decompressed when needed.
//! Each timezone's data is decompressed independently to minimize memory usage.
//!
//! # Error Handling
//!
//! All functions return `Option` and gracefully handle errors:
//! - Corrupted or invalid data returns `None`
//! - Zstd decompression failures return `None`
//! - Cache lock failures return `None`
//!
//! When `None` is returned, callers fall back to the timezone's current DST rules,
//! which is a reasonable approximation for most use cases.

use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use zstd::dict::DecoderDictionary;

/// A historical timezone transition.
#[derive(Debug, Clone, Copy)]
pub struct Transition {
    /// Unix timestamp when this transition occurs
    pub timestamp: i64,
    /// UTC offset in minutes after this transition
    pub offset: i16,
}

/// Cache of decompressed historical data.
static HISTORICAL_CACHE: RwLock<Option<HashMap<&'static str, Vec<Transition>>>> = RwLock::new(None);

/// Include the compressed historical data blob.
/// Format: [magic(4)][tz_count(2)][dict_len(4)][dictionary][index][compressed_data...]
/// Index: [(tz_id: u16, data_offset: u32, data_len: u16)...]
static HISTORICAL_BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tz_historical.bin"));

/// Lazily parsed dictionary for decompression.
static DECODER_DICT: Lazy<Option<DecoderDictionary<'static>>> = Lazy::new(|| {
    if HISTORICAL_BLOB.len() < 10 {
        return None;
    }

    let dict_len = u32::from_le_bytes([
        HISTORICAL_BLOB[6],
        HISTORICAL_BLOB[7],
        HISTORICAL_BLOB[8],
        HISTORICAL_BLOB[9],
    ]) as usize;

    if 10 + dict_len > HISTORICAL_BLOB.len() {
        return None;
    }

    let dict_bytes = &HISTORICAL_BLOB[10..10 + dict_len];
    Some(DecoderDictionary::copy(dict_bytes))
});

/// Get the historical offset for a timezone at a given timestamp.
/// Returns None if no historical data is available.
pub fn get_historical_offset(tz_name: &str, timestamp_secs: i64) -> Option<i16> {
    // Check cache first
    {
        let cache = HISTORICAL_CACHE.read().ok()?;
        if let Some(ref map) = *cache {
            if let Some(transitions) = map.get(tz_name) {
                return lookup_offset_in_transitions(transitions, timestamp_secs);
            }
        }
    }

    // Cache miss - load and decompress this timezone's historical data
    let transitions = load_historical_data(tz_name)?;
    let result = lookup_offset_in_transitions(&transitions, timestamp_secs);

    // Store in cache
    {
        let mut cache = HISTORICAL_CACHE.write().ok()?;
        let map = cache.get_or_insert_with(HashMap::new);
        // Use a static string if possible, otherwise skip caching
        if let Some(static_name) = get_static_tz_name(tz_name) {
            map.insert(static_name, transitions);
        }
    }

    result
}

/// Look up offset in sorted transitions using binary search.
fn lookup_offset_in_transitions(transitions: &[Transition], timestamp_secs: i64) -> Option<i16> {
    if transitions.is_empty() {
        return None;
    }

    // Binary search for the transition that applies at this timestamp
    let idx = transitions
        .binary_search_by(|t| t.timestamp.cmp(&timestamp_secs))
        .unwrap_or_else(|i| i.saturating_sub(1));

    transitions.get(idx).map(|t| t.offset)
}

/// Load and decompress historical data for a specific timezone.
fn load_historical_data(tz_name: &str) -> Option<Vec<Transition>> {
    if HISTORICAL_BLOB.len() < 10 {
        return None;
    }

    // Parse header
    let magic = u32::from_le_bytes([
        HISTORICAL_BLOB[0],
        HISTORICAL_BLOB[1],
        HISTORICAL_BLOB[2],
        HISTORICAL_BLOB[3],
    ]);

    if magic != 0x5A544C4C {
        // "LLTZ" in little-endian
        return None;
    }

    let tz_count = u16::from_le_bytes([HISTORICAL_BLOB[4], HISTORICAL_BLOB[5]]) as usize;
    let dict_len = u32::from_le_bytes([
        HISTORICAL_BLOB[6],
        HISTORICAL_BLOB[7],
        HISTORICAL_BLOB[8],
        HISTORICAL_BLOB[9],
    ]) as usize;

    // Parse index to find this timezone
    // Index starts after header (10 bytes) + dictionary
    let index_start = 10 + dict_len;
    let index_entry_size = 8; // tz_id (2) + data_offset (4) + data_len (2)

    // Find the timezone in the index
    let tz_id = find_tz_id(tz_name)?;

    if tz_id >= tz_count {
        return None;
    }

    let entry_offset = index_start + tz_id * index_entry_size;
    if entry_offset + index_entry_size > HISTORICAL_BLOB.len() {
        return None;
    }

    let data_offset = u32::from_le_bytes([
        HISTORICAL_BLOB[entry_offset + 2],
        HISTORICAL_BLOB[entry_offset + 3],
        HISTORICAL_BLOB[entry_offset + 4],
        HISTORICAL_BLOB[entry_offset + 5],
    ]) as usize;

    let data_len = u16::from_le_bytes([
        HISTORICAL_BLOB[entry_offset + 6],
        HISTORICAL_BLOB[entry_offset + 7],
    ]) as usize;

    if data_len == 0 {
        return Some(Vec::new());
    }

    if data_offset + data_len > HISTORICAL_BLOB.len() {
        return None;
    }

    // Get the prepared dictionary (lazily initialized)
    let dict = DECODER_DICT.as_ref()?;

    // Create decompressor with prepared dictionary and decompress
    let compressed = &HISTORICAL_BLOB[data_offset..data_offset + data_len];
    let mut decompressor = match zstd::bulk::Decompressor::with_prepared_dictionary(dict) {
        Ok(d) => d,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "llrt_tz: failed to create decompressor for timezone '{}': {:?}",
                tz_name, _e
            );
            return None;
        },
    };

    let decompressed = match decompressor.decompress(compressed, 1024 * 1024) {
        Ok(data) => data,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "llrt_tz: failed to decompress historical data for timezone '{}': {:?}",
                tz_name, _e
            );
            return None;
        },
    };

    // Parse transitions
    parse_transitions(&decompressed)
}

/// Parse transitions from decompressed data.
/// Format: [(timestamp: i64, offset: i16)...]
fn parse_transitions(data: &[u8]) -> Option<Vec<Transition>> {
    let entry_size = 10; // i64 + i16
    let count = data.len() / entry_size;
    let mut transitions = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * entry_size;
        if offset + entry_size > data.len() {
            break;
        }

        let timestamp = i64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        let tz_offset = i16::from_le_bytes([data[offset + 8], data[offset + 9]]);

        transitions.push(Transition {
            timestamp,
            offset: tz_offset,
        });
    }

    Some(transitions)
}

/// Find the timezone ID from the timezone names table.
fn find_tz_id(name: &str) -> Option<usize> {
    crate::compact::TZ_VARIANTS
        .binary_search_by(|tz| tz.name.cmp(name))
        .ok()
}

/// Get a static reference to a timezone name if it exists.
fn get_static_tz_name(name: &str) -> Option<&'static str> {
    crate::compact::lookup_timezone(name).map(|tz| tz.name)
}
