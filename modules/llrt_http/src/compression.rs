// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(any(feature = "flate2-c", feature = "flate2-rust")))]
compile_error!("Either the `flate2-c` or `flate2-rust` feature must be enabled");

#[cfg(not(any(feature = "brotli-c", feature = "brotli-rust")))]
compile_error!("Either the `brotli-c` or `brotli-rust` feature must be enabled");

pub(crate) mod zstd {
    use std::io::{BufReader, Read};

    use rquickjs::Result;
    use zstd::stream::read::Decoder as ZstdDecoder;

    pub fn decoder<R: Read>(r: R) -> Result<ZstdDecoder<'static, BufReader<R>>> {
        Ok(ZstdDecoder::new(r)?)
    }
}

#[cfg(any(feature = "flate2-c", feature = "flate2-rust"))]
pub(crate) mod gz {
    use std::io::Read;

    use flate2::read::GzDecoder;

    pub fn decoder<R: Read>(r: R) -> GzDecoder<R> {
        GzDecoder::new(r)
    }
}

#[cfg(any(feature = "flate2-c", feature = "flate2-rust"))]
pub(crate) mod zlib {
    use std::io::Read;

    use flate2::read::ZlibDecoder;

    pub fn decoder<R: Read>(r: R) -> ZlibDecoder<R> {
        ZlibDecoder::new(r)
    }
}

#[cfg(feature = "brotli-c")]
pub(crate) mod brotli {
    use std::io::BufRead;

    use brotlic::DecompressorReader as BrotliDecoder;

    pub fn decoder<R: BufRead>(r: R) -> BrotliDecoder<R> {
        BrotliDecoder::new(r)
    }
}

#[cfg(all(not(feature = "brotli-c"), feature = "brotli-rust"))]
pub(crate) mod brotli {
    use std::io::Read;

    use brotli::Decompressor as BrotliDecoder;

    pub fn decoder<R: Read>(r: R) -> BrotliDecoder<R> {
        BrotliDecoder::new(r, 8_096)
    }
}
