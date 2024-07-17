// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

// This is not the full module for now since we are not yet sure
// how to implement the TIME_ORIGIN Atomic in what are supposed to
// be independant modules.

pub fn get_platform() -> &'static str {
    let platform = env::consts::OS;
    match platform {
        "macos" => "darwin",
        "windows" => "win32",
        _ => platform,
    }
}
