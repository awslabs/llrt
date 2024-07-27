// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hex_simd::AsciiCase;

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Clone)]
pub enum Encoder {
    Hex,
    Base64,
    Windows1252,
    Utf8,
    Utf16le,
}

static ENCODING_MAP: Lazy<HashMap<&'static str, Encoder>> = Lazy::new(|| {
    let mut map = HashMap::with_capacity(24);
    // Encoder::Hex
    map.insert("hex", Encoder::Hex);
    // Encoder::Base64
    map.insert("base64", Encoder::Base64);
    // Encoder::Utf8
    map.insert("utf-8", Encoder::Utf8);
    map.insert("utf8", Encoder::Utf8);
    map.insert("unicode-1-1-utf8", Encoder::Utf8);
    // Encoder::Utf16le
    map.insert("utf-16le", Encoder::Utf16le);
    map.insert("utf-16", Encoder::Utf16le);
    // Encoder::Windows1252
    map.insert("windows-1252", Encoder::Windows1252);
    map.insert("ansi_x3.4-1968", Encoder::Windows1252);
    map.insert("ascii", Encoder::Windows1252);
    map.insert("cp1252", Encoder::Windows1252);
    map.insert("cp819", Encoder::Windows1252);
    map.insert("csisolatin1", Encoder::Windows1252);
    map.insert("ibm819", Encoder::Windows1252);
    map.insert("iso-8859-1", Encoder::Windows1252);
    map.insert("iso-ir-100", Encoder::Windows1252);
    map.insert("iso8859-1", Encoder::Windows1252);
    map.insert("iso88591", Encoder::Windows1252);
    map.insert("iso_8859-1", Encoder::Windows1252);
    map.insert("iso_8859-1:1987", Encoder::Windows1252);
    map.insert("l1", Encoder::Windows1252);
    map.insert("latin1", Encoder::Windows1252);
    map.insert("us-ascii", Encoder::Windows1252);
    map.insert("x-cp1252", Encoder::Windows1252);
    map
});

impl Encoder {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(encoding: &str) -> Result<Self, String> {
        ENCODING_MAP
            .get(encoding.to_ascii_lowercase().as_str())
            .cloned()
            .ok_or_else(|| ["The \"", encoding, "\" encoding is not supported"].concat())
    }

    pub fn encode_to_string(&self, bytes: &[u8], lossy: bool) -> Result<String, String> {
        match self {
            Self::Hex => Ok(bytes_to_hex_string(bytes)),
            Self::Base64 => Ok(bytes_to_b64_string(bytes)),
            Self::Utf8 | Self::Windows1252 => bytes_to_string(bytes, lossy),
            Self::Utf16le => bytes_to_utf16le_string(bytes, lossy),
        }
    }

    #[allow(dead_code)]
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => Ok(bytes_to_hex(bytes)),
            Self::Base64 => Ok(bytes_to_b64(bytes)),
            Self::Utf8 | Self::Windows1252 | Self::Utf16le => Ok(bytes.to_vec()),
        }
    }

    pub fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => bytes_from_hex(&bytes),
            Self::Base64 => bytes_from_b64(&bytes),
            Self::Utf8 | Self::Windows1252 | Self::Utf16le => Ok(bytes),
        }
    }

    pub fn decode_from_string(&self, string: String) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => bytes_from_hex(string.as_bytes()),
            Self::Base64 => bytes_from_b64(string.as_bytes()),
            Self::Utf8 | Self::Windows1252 => Ok(string.into_bytes()),
            Self::Utf16le => Ok(string
                .encode_utf16()
                .flat_map(|utf16| utf16.to_le_bytes())
                .collect::<Vec<u8>>()),
        }
    }

    pub fn as_label(&self) -> &str {
        match self {
            Self::Hex => "hex",
            Self::Base64 => "base64",
            Self::Windows1252 => "windows-1252",
            Self::Utf8 => "utf-8",
            Self::Utf16le => "utf-16le",
        }
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> Vec<u8> {
    hex_simd::encode_type(bytes, AsciiCase::Lower)
}

pub fn bytes_from_hex(hex_bytes: &[u8]) -> Result<Vec<u8>, String> {
    hex_simd::decode_to_vec(hex_bytes).map_err(|err| err.to_string())
}

pub fn bytes_to_b64_string(bytes: &[u8]) -> String {
    base64_simd::STANDARD.encode_to_string(bytes)
}

pub fn bytes_from_b64(bytes: &[u8]) -> Result<Vec<u8>, String> {
    base64_simd::forgiving_decode_to_vec(bytes).map_err(|e| e.to_string())
}

pub fn bytes_to_b64(bytes: &[u8]) -> Vec<u8> {
    base64_simd::STANDARD.encode_type(bytes)
}

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    hex_simd::encode_to_string(bytes, AsciiCase::Lower)
}

pub fn bytes_to_string(bytes: &[u8], lossy: bool) -> Result<String, String> {
    match lossy {
        true => Ok(String::from_utf8_lossy(bytes).to_string()),
        false => String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string()),
    }
}

#[cfg(not(rust_nightly))]
pub fn bytes_to_utf16le_string(bytes: &[u8], lossy: bool) -> Result<String, String> {
    let data16 = bytes
        .chunks(2)
        .map(|e| e.try_into().map(u16::from_le_bytes))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    match lossy {
        true => Ok(String::from_utf16_lossy(&data16)),
        false => String::from_utf16(&data16).map_err(|e| e.to_string()),
    }
}

#[cfg(rust_nightly)]
pub fn bytes_to_utf16le_string(bytes: &[u8], lossy: bool) -> Result<String, String> {
    let data16 = bytes
        .array_chunks()
        .cloned()
        .map(|e| e.try_into().map(u16::from_le_bytes))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    match lossy {
        true => Ok(String::from_utf16_lossy(&data16)),
        false => String::from_utf16(&data16).map_err(|e| e.to_string()),
    }
}
