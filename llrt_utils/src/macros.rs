// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[macro_export]
macro_rules! iterable_enum {
    ($visibility:vis, $name:ident, $($member:tt),*) => {
        #[derive(Copy, Clone)]
        $visibility enum $name {$($member),*}
        impl $name {
            pub fn iterate() -> Vec<$name> {
                vec![$($name::$member,)*]
            }
        }
    };
    ($name:ident, $($member:tt),*) => {
        iterable_enum!(, $name, $($member),*)
    };
}
