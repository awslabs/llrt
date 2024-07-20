// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::perf_hooks::new_performance;
use rquickjs::{Ctx, Result};

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    new_performance(ctx.clone())?;
    Ok(())
}
