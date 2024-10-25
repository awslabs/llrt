// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub(crate) mod zstd {
    use std::io::{BufReader, Read};

    use rquickjs::Result;
    use zstd::stream::read::Decoder as ZstdDecoder;

    pub fn decoder<R: Read>(r: R) -> Result<ZstdDecoder<'static, BufReader<R>>> {
        Ok(ZstdDecoder::new(r)?)
    }
}

pub(crate) mod gz {
    use std::io::Read;

    use flate2::read::GzDecoder;

    pub fn decoder<R: Read>(r: R) -> GzDecoder<R> {
        #[cfg(any(feature = "flate2-c", feature = "flate2-rust"))]
        {
            GzDecoder::new(r)
        }
        #[cfg(not(any(feature = "flate2-c", feature = "flate2-rust")))]
        {
            compile_error!("Either the `flate2-c` or `flate2-rust` feature must be enabled")
        }
    }
}

pub(crate) mod zlib {
    use std::io::Read;

    use flate2::read::ZlibDecoder;

    pub fn decoder<R: Read>(r: R) -> ZlibDecoder<R> {
        #[cfg(any(feature = "flate2-c", feature = "flate2-rust"))]
        {
            ZlibDecoder::new(r)
        }
        #[cfg(not(any(feature = "flate2-c", feature = "flate2-rust")))]
        {
            compile_error!("Either the `flate2-c` or `flate2-rust` feature must be enabled")
        }
    }
}

pub(crate) mod brotli {
    #[cfg(feature = "brotli-c")]
    use std::io::BufRead;
    #[cfg(feature = "brotli-rust")]
    use std::io::Read;

    #[cfg(feature = "brotli-rust")]
    use brotli::Decompressor as BrotliDecoder;
    #[cfg(feature = "brotli-c")]
    use brotlic::DecompressorReader as BrotliDecoder;

    #[cfg(feature = "brotli-c")]
    pub fn decoder<R: BufRead>(r: R) -> BrotliDecoder<R> {
        BrotliDecoder::new(r)
    }

    #[cfg(feature = "brotli-rust")]
    pub fn decoder<R: Read>(r: R) -> BrotliDecoder<R> {
        BrotliDecoder::new(r, 8_096)
    }

    #[cfg(not(any(feature = "brotli-c", feature = "brotli-rust")))]
    compile_error!("Either the `brotli-c` or `brotli-rust` feature must be enabled");
}
