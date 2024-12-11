// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[macro_export]
macro_rules! count_members {
    () => (0);
    ($head:tt $(,$tail:tt)*) => (1 + count_members!($($tail),*));
}

#[macro_export]
macro_rules! iterable_enum {
    ($name:ident, $($variant:ident),*) => {
        impl $name {
            const VARIANTS: &'static [$name] = &[$($name::$variant,)*];
            pub fn iter() -> std::slice::Iter<'static, $name> {
                Self::VARIANTS.iter()
            }

            #[allow(dead_code)]
            fn _ensure_all_variants(s: Self) {
                match s {
                    $($name::$variant => {},)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! str_enum {
    ($name:ident, $($variant:ident => $str:expr),*) => {
        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $str,)*
                }
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl TryFrom<&str> for $name {
            type Error = String;
            fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
                match s.to_ascii_uppercase().as_str() {
                    $($str => Ok($name::$variant),)*
                    _ => Err(["'", s, "' not available"].concat())
                }
            }
        }
    };
}
