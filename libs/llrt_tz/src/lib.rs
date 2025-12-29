// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Compact timezone library for LLRT.
//!
//! This library provides timezone offset calculations with significantly reduced memory
//! footprint compared to chrono-tz (~165KB vs ~1.1MB), while maintaining 100% accuracy
//! for all timezones from 1970-2024.
//!
//! # Architecture
//!
//! The library uses a two-tier architecture:
//!
//! 1. **Compact DST Rules (~15KB)** - Always in memory. For each timezone, stores:
//!    - Standard UTC offset (e.g., -300 minutes for EST)
//!    - DST transition rules (e.g., "2nd Sunday of March at 2:00 AM")
//!    - A `rules_valid_from` timestamp indicating when these rules became effective
//!
//! 2. **Compressed Historical Data (~150KB)** - Zstd-compressed, loaded lazily on first
//!    access to historical dates. Contains hourly offset transitions from 1970 to present.
//!
//! # When Historical Data is Loaded
//!
//! Historical data is loaded only when querying a timestamp before the timezone's
//! `rules_valid_from` date. For example:
//! - Querying "America/New_York" for 2024 → Uses compact rules (no decompression)
//! - Querying "America/New_York" for 2006 → Loads historical data (US changed DST rules in 2007)
//!
//! Once loaded, historical data is cached for subsequent queries.
//!
//! # Performance
//!
//! - **Recent dates**: O(1) calculation using DST rules
//! - **Historical dates**: O(log n) binary search after one-time decompression
//! - **Memory**: ~15KB baseline, +~150KB if historical data is accessed
//!
//! # Special Cases
//!
//! Morocco (`Africa/Casablanca`, `Africa/El_Aaiun`) suspends DST during Ramadan, which
//! follows the Islamic lunar calendar. Since this cannot be encoded in fixed DST rules,
//! these timezones always use historical data lookup for accurate offsets.
//!
//! # Example
//!
//! ```
//! use llrt_tz::Tz;
//!
//! // Parse a timezone
//! let tz: Tz = "America/New_York".parse().unwrap();
//!
//! // Get offset at a specific Unix timestamp (in minutes from UTC)
//! let offset = tz.offset_at_timestamp(1704067200); // 2024-01-01 00:00:00 UTC
//! assert_eq!(offset, -300); // EST = UTC-5:00 = -300 minutes
//! ```

mod compact;
mod historical;
mod tz_wrapper;

pub use compact::{get_offset, list_timezones, lookup_timezone, Timezone, TZ_NAMES, TZ_VARIANTS};
pub use tz_wrapper::{Tz, TzOffset};

/// The UTC timezone.
pub const UTC: Tz = Tz::Utc;

#[cfg(test)]
mod tests;
