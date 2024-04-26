// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hex_simd::AsciiCase;

macro_rules! encoder_enum {
    (
        $(#[$attr:meta])*
        pub enum $enum_name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        $(#[$attr])*
        pub enum $enum_name {
            $($variant),*
        }

        impl $enum_name {
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(encoding: &str) -> Result<Self, String> {

                let encoding:String = encoding.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_ascii_uppercase()
                        } else {
                            c.to_ascii_lowercase()
                        }
                    })
                    .filter(|&c| c != '-' && c != '_')
                    .collect();

                match encoding.as_str() {
                    $(
                        stringify!($variant) => Ok(Self::$variant),
                    )*
                    _ => Err(format!("Unsupported encoding: {}", encoding)),
                }
            }
        }
    };
}

encoder_enum! {
    pub enum Encoder {
        Hex,
        Base64,
        Utf8,
        Iso88591,
    }
}

impl Encoder {
    pub fn encode_to_string(&self, bytes: &[u8]) -> Result<String, String> {
        match self {
            Self::Hex => Ok(bytes_to_hex_string(bytes)),
            Self::Base64 => Ok(bytes_to_b64_string(bytes)),
            Self::Utf8 | Self::Iso88591 => Ok(bytes_to_string(bytes)),
        }
    }

    #[allow(dead_code)]
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => Ok(bytes_to_hex(bytes)),
            Self::Base64 => Ok(bytes_to_b64(bytes)),
            Self::Utf8 | Self::Iso88591 => Ok(bytes.to_vec()),
        }
    }

    pub fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => bytes_from_hex(&bytes),
            Self::Base64 => bytes_from_b64(&bytes),
            Self::Utf8 | Self::Iso88591 => Ok(bytes),
        }
    }

    #[allow(dead_code)]
    pub fn decode_from_string(&self, string: String) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex => bytes_from_hex(string.as_bytes()),
            Self::Base64 => bytes_from_b64(string.as_bytes()),
            Self::Utf8 | Self::Iso88591 => Ok(string.into_bytes()),
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

pub fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}
