// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{env, error::Error, fs, io, result::Result as StdResult};

const AWS_SDK_VERSION_FILE: &str = "../../bundle/js/.aws-sdk-version";

fn main() -> StdResult<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={AWS_SDK_VERSION_FILE}");

    if env::var_os("CARGO_FEATURE_NO_SDK").is_some() {
        return Ok(());
    }

    let version = match fs::read_to_string(AWS_SDK_VERSION_FILE) {
        Ok(version) => version,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let version = version.trim();
    if version.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "AWS SDK version is empty").into());
    }

    println!("cargo:rustc-env=LLRT_AWS_SDK_VERSION={version}");
    Ok(())
}
