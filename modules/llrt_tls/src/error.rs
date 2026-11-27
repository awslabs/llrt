// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsError {
    pub message: Cow<'static, str>,
    pub code: Option<Cow<'static, str>>,
    pub library: Option<&'static str>,
    pub reason: Option<&'static str>,
    pub openssl_error_stack: Vec<String>,
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TlsError {}

#[cfg(any(
    feature = "tls-openssl",
    all(
        feature = "_encrypted-keys",
        any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola")
    )
))]
macro_rules! openssl_errors {
    ($($name:ident = $id:literal $library:literal $reason:literal $code:literal;)*) => {$(
        pub(crate) const $name: TlsError = TlsError {
            message: Cow::Borrowed(concat!("error:", $id, ":", $library, "::", $reason)),
            code: Some(Cow::Borrowed($code)),
            library: Some($library),
            reason: Some($reason),
            openssl_error_stack: Vec::new(),
        };
    )*};
}

#[cfg(any(
    feature = "tls-openssl",
    all(
        feature = "_encrypted-keys",
        any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola")
    )
))]
openssl_errors! {
    INTERRUPTED_OR_CANCELLED = "07880109" "common libcrypto routines" "interrupted or cancelled" "ERR_OSSL_CRYPTO_INTERRUPTED_OR_CANCELLED";
}

// The errors Node.js throws for encrypted private keys, with OpenSSL 3.
#[cfg(all(
    feature = "_encrypted-keys",
    any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola")
))]
openssl_errors! {
    BAD_DECRYPT = "1C800064" "Provider routines" "bad decrypt" "ERR_OSSL_BAD_DECRYPT";
    WRONG_FINAL_BLOCK_LENGTH = "1C80006B" "Provider routines" "wrong final block length" "ERR_OSSL_WRONG_FINAL_BLOCK_LENGTH";
    INVALID_ITERATION_COUNT = "1C80007B" "Provider routines" "invalid iteration count" "ERR_OSSL_INVALID_ITERATION_COUNT";
    DECODER_UNSUPPORTED = "1E08010C" "DECODER routines" "unsupported" "ERR_OSSL_UNSUPPORTED";
    EVP_UNSUPPORTED = "0308010C" "digital envelope routines" "unsupported" "ERR_OSSL_EVP_UNSUPPORTED";
    UNSUPPORTED_CIPHER = "0300006B" "digital envelope routines" "unsupported cipher" "ERR_OSSL_EVP_UNSUPPORTED_CIPHER";
    DECODE_ERROR = "03000072" "digital envelope routines" "decode error" "ERR_OSSL_EVP_DECODE_ERROR";
    UNKNOWN_PBE_ALGORITHM = "03000079" "digital envelope routines" "unknown pbe algorithm" "ERR_OSSL_EVP_UNKNOWN_PBE_ALGORITHM";
    CIPHER_PARAMETER_ERROR = "0300007A" "digital envelope routines" "cipher parameter error" "ERR_OSSL_EVP_CIPHER_PARAMETER_ERROR";
    UNSUPPORTED_KEYLENGTH = "0300007B" "digital envelope routines" "unsupported keylength" "ERR_OSSL_EVP_UNSUPPORTED_KEYLENGTH";
    UNSUPPORTED_KDF = "0300007C" "digital envelope routines" "unsupported key derivation function" "ERR_OSSL_EVP_UNSUPPORTED_KEY_DERIVATION_FUNCTION";
    UNSUPPORTED_PRF = "0300007D" "digital envelope routines" "unsupported prf" "ERR_OSSL_EVP_UNSUPPORTED_PRF";
    UNSUPPORTED_SALT_TYPE = "0300007E" "digital envelope routines" "unsupported salt type" "ERR_OSSL_EVP_UNSUPPORTED_SALT_TYPE";
    PARAM_NEGATIVE_VALUE = "0780007D" "common libcrypto routines" "param unsigned integer negative value unsupported" "ERR_OSSL_CRYPTO_PARAM_UNSIGNED_INTEGER_NEGATIVE_VALUE_UNSUPPORTED";
    DATA_IS_WRONG = "0680006D" "asn1 encoding routines" "data is wrong" "ERR_OSSL_ASN1_DATA_IS_WRONG";
}
