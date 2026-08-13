// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_exceptions::DOMException;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Ctx, Exception, FromJs, Result, Value};

use super::{algorithm_not_supported_error, normalize_algorithm_name, to_name_and_maybe_object};

#[derive(Debug)]
pub enum EncryptionAlgorithm {
    AesCbc {
        iv: Box<[u8]>,
    },
    AesCtr {
        counter: Box<[u8]>,
        length: u32,
    },
    AesGcm {
        iv: Box<[u8]>,
        tag_length: u8,
        additional_data: Option<Box<[u8]>>,
    },
    RsaOaep {
        label: Option<Box<[u8]>>,
    },
    AesKw,
}

impl<'js> FromJs<'js> for EncryptionAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let name = normalize_algorithm_name(&name);

        match name.as_str() {
            "AES-CBC" => {
                let obj = obj?;
                let iv = obj
                    .get_required::<_, ObjectBytes>("iv", "algorithm")?
                    .into_bytes(ctx)?
                    .into_boxed_slice();

                if iv.len() != 16 {
                    return Err(DOMException::operation_error(
                        ctx,
                        "invalid length of iv. Currently supported 16 bytes",
                    ));
                }

                Ok(EncryptionAlgorithm::AesCbc { iv })
            },
            "AES-CTR" => {
                let obj = obj?;
                let counter = obj
                    .get_required::<_, ObjectBytes>("counter", "algorithm")?
                    .into_bytes(ctx)?
                    .into_boxed_slice();

                let length = obj.get_required::<_, u32>("length", "algorithm")?;

                if !matches!(length, 32 | 64 | 128) {
                    return Err(DOMException::operation_error(
                        ctx,
                        "invalid counter length. Currently supported 32/64/128 bits",
                    ));
                }

                Ok(EncryptionAlgorithm::AesCtr { counter, length })
            },
            "AES-GCM" => {
                let obj = obj?;
                let iv = obj
                    .get_required::<_, ObjectBytes>("iv", "algorithm")?
                    .into_bytes(ctx)?
                    .into_boxed_slice();

                //FIXME only 12? 96 maybe recommended?
                if iv.len() != 12 {
                    return Err(Exception::throw_type(
                        ctx,
                        "invalid length of iv. Currently supported 12 bytes",
                    ));
                }

                let additional_data = obj
                    .get_optional::<_, ObjectBytes>("additionalData")?
                    .map(|v| v.into_bytes(ctx))
                    .transpose()?
                    .map(|vec| vec.into_boxed_slice());

                let tag_length = obj.get_optional::<_, u8>("tagLength")?.unwrap_or(128);

                //ensure tag length is supported using a match statement 32, 64, 96, 104, 112, 120, or 128
                if !matches!(tag_length, 32 | 64 | 96 | 104 | 112 | 120 | 128) {
                    return Err(DOMException::operation_error(ctx, "Invalid tagLength"));
                }

                Ok(EncryptionAlgorithm::AesGcm {
                    iv,
                    additional_data,
                    tag_length,
                })
            },
            "RSA-OAEP" => {
                let label = if let Ok(obj) = obj {
                    obj.get_optional::<_, ObjectBytes>("label")?
                        .map(|bytes| bytes.into_bytes(ctx))
                        .transpose()?
                        .map(|vec| vec.into_boxed_slice())
                } else {
                    None
                };

                Ok(EncryptionAlgorithm::RsaOaep { label })
            },
            "AES-KW" => Ok(EncryptionAlgorithm::AesKw),
            _ => algorithm_not_supported_error(ctx),
        }
    }
}
