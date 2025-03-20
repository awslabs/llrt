// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_buffer::ArrayBufferView;
use llrt_encoding::Encoder;
use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, CString, Ctx, Exception, Result};

#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct StringDecoder {
    #[qjs(skip_trace)]
    encoder: Encoder,
    buffer: Vec<u8>,
    buffered_bytes: usize,
    missing_bytes: usize,
}

impl StringDecoder {
    fn make_string(&self, ctx: &Ctx<'_>, data: &[u8]) -> Result<String> {
        self.encoder
            .encode_to_string(data, true)
            .map_err(|_| Exception::throw_internal(ctx, "Encoding error"))
    }

    /// Try to decode the given buffer and store the incomplete bytes.
    /// The logic was adapted from the [Node implementation].
    ///
    /// [Node implementation]: https://github.com/nodejs/node/blob/ba06c5c509956dc413f91b755c1c93798bb700d4/src/string_decoder.cc#L66
    fn decode_data(&mut self, ctx: &Ctx<'_>, mut data: &[u8]) -> Result<String> {
        let mut result = String::new();

        if matches!(
            self.encoder,
            Encoder::Utf8 | Encoder::Utf16le | Encoder::Base64
        ) {
            // See if we want bytes to finish a character from the previous
            // chunk; if so, copy the new bytes to the missing bytes buffer
            // and create a string from it that is to be prepended to the main body.
            if self.missing_bytes > 0 {
                if matches!(self.encoder, Encoder::Utf8) {
                    // For UTF-8, we need special alignment treatment:
                    // If an incomplete character is found at a chunk boundary, we use
                    // its remainder and try to decode it.
                    let mut i = 0;
                    while i < data.len() && i < self.missing_bytes {
                        if (data[i] & 0xC0) != 0x80 {
                            // This byte is not a continuation byte even though it should have
                            // been one. We stop decoding of the incomplete character at this
                            // point (but still use the rest of the incomplete bytes from this
                            // chunk) and assume that the new, unexpected byte starts a new one.
                            self.missing_bytes = 0;
                            self.buffer.extend_from_slice(&data[..i]);
                            self.buffered_bytes += i;
                            data = &data[i..];
                            break;
                        }
                        i += 1;
                    }
                }

                let found_bytes = std::cmp::min(data.len(), self.missing_bytes);
                self.buffer.extend_from_slice(&data[..found_bytes]);

                data = &data[found_bytes..];

                self.missing_bytes -= found_bytes;
                self.buffered_bytes += found_bytes;
                if self.missing_bytes == 0 {
                    // We have enough bytes to decode the buffered character
                    result = self.make_string(ctx, &self.buffer)?;
                    self.buffer.clear();
                    self.buffered_bytes = 0;
                }
            }

            // It could be that trying to finish the previous chunk already
            // consumed all data that we received in this chunk.
            if data.is_empty() {
                return Ok(result);
            } else {
                // If not, that means is no character left to finish at this point.

                // See whether there is a character that we may have to cut off and
                // finish when receiving the next chunk.
                if matches!(self.encoder, Encoder::Utf8) && (data[data.len() - 1] & 0x80) != 0 {
                    let mut i = data.len() - 1;
                    loop {
                        self.buffered_bytes += 1;
                        if (data[i] & 0xC0) == 0x80 {
                            // This byte does not start a character (a "trailing" byte).
                            if self.buffered_bytes >= 4 || i == 0 {
                                // We either have more then 4 trailing bytes (which means
                                // the current character would not be inside the range for
                                // valid Unicode, and in particular cannot be represented
                                // through JavaScript's UTF-16-based approach to strings), or the
                                // current buffer does not contain the start of an UTF-8 character
                                // at all. Either way, this is invalid UTF8 and we can just
                                // let the engine's decoder handle it.
                                self.buffer.clear();
                                self.buffered_bytes = 0;
                                break;
                            }
                        } else {
                            // Found the first byte of a UTF-8 character. By looking at the
                            // upper bits we can tell how long the character *should* be.
                            if (data[i] & 0xE0) == 0xC0 {
                                self.missing_bytes = 2;
                            } else if (data[i] & 0xF0) == 0xE0 {
                                self.missing_bytes = 3;
                            } else if (data[i] & 0xF8) == 0xF0 {
                                self.missing_bytes = 4;
                            } else {
                                // This lead byte would indicate a character outside of the
                                // representable range.
                                self.buffered_bytes = 0;
                                break;
                            }

                            if self.buffered_bytes >= self.missing_bytes {
                                // Received more or exactly as many trailing bytes than the lead
                                // character would indicate. In the "==" case, we have valid
                                // data and don't need to slice anything off;
                                // in the ">" case, this is invalid UTF-8 anyway.
                                self.missing_bytes = 0;
                                self.buffered_bytes = 0;
                            }

                            self.missing_bytes -= self.buffered_bytes;
                            break;
                        }
                        i -= 1;
                    }
                } else if matches!(self.encoder, Encoder::Utf16le) {
                    // WARN: For UTF-16LE we deviate from the specification when an invalid
                    // high surrogate is found. The spec says we should keep it as is, but
                    // there no way to encode in UTF-8 (required to interface with quickjs).
                    // For now, we will replace it with a replacement character.
                    // See https://github.com/quickjs-ng/quickjs/issues/992
                    if (data.len() % 2) == 1 {
                        // We got half a codepoint, and need the second byte of it.
                        self.buffered_bytes = 1;
                        self.missing_bytes = 1;
                    } else if (data[data.len() - 1] & 0xFC) == 0xD8 {
                        // Half a split UTF-16 character.
                        self.buffered_bytes = 2;
                        self.missing_bytes = 2;
                    }
                } else if matches!(self.encoder, Encoder::Base64) {
                    self.buffered_bytes = data.len() % 3;
                    if self.buffered_bytes > 0 {
                        self.missing_bytes = 3 - self.buffered_bytes;
                    }
                }

                if self.buffered_bytes > 0 {
                    // Copy the requested number of buffered bytes from the end of the
                    // input into the incomplete character buffer.
                    self.buffer
                        .extend_from_slice(&data[data.len() - self.buffered_bytes..]);
                    data = &data[..data.len() - self.buffered_bytes];
                }

                if !data.is_empty() {
                    result.push_str(&self.make_string(ctx, data)?);
                }
            }

            Ok(result)
        } else {
            // For ASCII, HEX, and LATIN1, we can decode everything directly
            self.make_string(ctx, data)
        }
    }

    fn flush(&mut self, ctx: &Ctx<'_>) -> Result<String> {
        if matches!(self.encoder, Encoder::Utf16le) && self.buffered_bytes % 2 == 1 {
            // Ignore a single trailing byte, like the JS decoder does.
            self.missing_bytes -= 1;
            self.buffered_bytes -= 1;
        }

        if self.buffered_bytes == 0 {
            return Ok(String::new());
        }

        let res = self.make_string(ctx, &self.buffer);

        self.missing_bytes = 0;
        self.buffered_bytes = 0;
        self.buffer.clear();

        res
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl StringDecoder {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, encoding: Opt<CString<'_>>) -> Result<Self> {
        let encoding = encoding.0.as_ref().map(|e| e.as_str()).unwrap_or("utf-8");

        let encoder = Encoder::from_str(encoding).map_err(|_| {
            let msg = ["Unknown encoding: ", encoding].concat();
            Exception::throw_type(&ctx, &msg)
        })?;

        Ok(Self {
            encoder,
            buffer: Vec::new(),
            buffered_bytes: 0,
            missing_bytes: 0,
        })
    }

    #[qjs(get)]
    pub fn encoding(&self) -> &str {
        self.encoder.as_label()
    }

    pub fn end(&mut self, ctx: Ctx<'_>, buffer: Opt<ArrayBufferView<'_>>) -> Result<String> {
        let output = if let Some(data) = buffer.0.as_ref().and_then(|b| b.as_bytes()) {
            Some(self.decode_data(&ctx, data)?)
        } else {
            None
        };

        let flush = self.flush(&ctx)?;
        Ok(output
            .map(|mut o| {
                o.push_str(&flush);
                o
            })
            .unwrap_or(flush))
    }

    pub fn write(&mut self, ctx: Ctx<'_>, buffer: ArrayBufferView<'_>) -> Result<String> {
        let data = buffer
            .as_bytes()
            .or_throw_msg(&ctx, "Buffer has already been used")?;
        self.decode_data(&ctx, data)
    }
}
