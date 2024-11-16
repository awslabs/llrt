// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(rust_nightly, feature(array_chunks))]
use std::borrow::Cow;

use hex_simd::AsciiCase;

#[derive(Clone)]
pub enum Encoder {
    Hex,
    Base64,
    Windows1252,
    Utf8,
    Utf16le,
    Utf16be,
}

const ENCODING_MAP: phf::Map<&'static str, Encoder> = phf::phf_map! {
    "hex" => Encoder::Hex,
    "base64" => Encoder::Base64,
    "utf-8" => Encoder::Utf8,
    "utf8" => Encoder::Utf8,
    "unicode-1-1-utf8" => Encoder::Utf8,
    "utf-16le" => Encoder::Utf16le,
    "utf16le" => Encoder::Utf16le,
    "utf-16" => Encoder::Utf16le,
    "utf16" => Encoder::Utf16le,
    "utf-16be" => Encoder::Utf16be,
    "utf16be" => Encoder::Utf16be,
    "windows-1252" => Encoder::Windows1252,
    "ansi_x3.4-1968" => Encoder::Windows1252,
    "ascii" => Encoder::Windows1252,
    "cp1252" => Encoder::Windows1252,
    "cp819" => Encoder::Windows1252,
    "csisolatin1" => Encoder::Windows1252,
    "ibm819" => Encoder::Windows1252,
    "iso-8859-1" => Encoder::Windows1252,
    "iso-ir-100" => Encoder::Windows1252,
    "iso8859-1" => Encoder::Windows1252,
    "iso88591" => Encoder::Windows1252,
    "iso_8859-1" => Encoder::Windows1252,
    "iso_8859-1:1987" => Encoder::Windows1252,
    "l1" => Encoder::Windows1252,
    "latin1" => Encoder::Windows1252,
    "us-ascii" => Encoder::Windows1252,
    "x-cp1252" => Encoder::Windows1252,
};

impl Encoder {
    pub fn from_optional_str(encoding: Option<&str>) -> Result<Self, String> {
        match encoding {
            Some(label) if !label.is_empty() => Self::from_str(label),
            _ => Ok(Self::Utf8),
        }
    }

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
            Self::Utf16le => bytes_to_utf16_string(bytes, Endian::Little, lossy),
            Self::Utf16be => bytes_to_utf16_string(bytes, Endian::Big, lossy),
        }
    }

    #[allow(dead_code)]
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => Ok(bytes_to_hex(bytes)),
            Self::Base64 => Ok(bytes_to_b64(bytes)),
            Self::Utf8 | Self::Windows1252 | Self::Utf16le | Self::Utf16be => Ok(bytes.to_vec()),
        }
    }

    pub fn decode<'a, T: Into<Cow<'a, [u8]>>>(&self, bytes: T) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => bytes_from_hex(&bytes.into()),
            Self::Base64 => bytes_from_b64(&bytes.into()),
            Self::Utf8 | Self::Windows1252 | Self::Utf16le | Self::Utf16be => {
                Ok(bytes.into().into())
            },
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
            Self::Utf16be => Ok(string
                .encode_utf16()
                .flat_map(|utf16| utf16.to_be_bytes())
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
            Self::Utf16be => "utf-16be",
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
    if lossy {
        Ok(String::from_utf8_lossy(bytes).to_string())
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy)]
pub enum Endian {
    Little,
    Big,
}

pub fn bytes_to_utf16_string(bytes: &[u8], endian: Endian, lossy: bool) -> Result<String, String> {
    if bytes.len() % 2 != 0 {
        return Err("Input byte slice length must be even".to_string());
    }

    #[cfg(rust_nightly)]
    let data16: Vec<u16> = match endian {
        Endian::Little => bytes
            .array_chunks::<2>()
            .map(|&chunk| u16::from_le_bytes(chunk))
            .collect(),
        Endian::Big => bytes
            .array_chunks::<2>()
            .map(|&chunk| u16::from_be_bytes(chunk))
            .collect(),
    };

    #[cfg(not(rust_nightly))]
    let data16: Vec<u16> = match endian {
        Endian::Little => bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect(),
        Endian::Big => bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect(),
    };

    if lossy {
        Ok(String::from_utf16_lossy(&data16))
    } else {
        String::from_utf16(&data16).map_err(|e| e.to_string())
    }
}
