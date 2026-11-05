// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use llrt_net_policy::{parse_list, set_allow_list, set_deny_list};

use crate::environment::{ENV_LLRT_NET_ALLOW, ENV_LLRT_NET_DENY};

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(env_value) = env::var(ENV_LLRT_NET_ALLOW) {
        set_allow_list(parse_list(
            ENV_LLRT_NET_ALLOW,
            env_value.split_whitespace(),
        )?);
    }

    if let Ok(env_value) = env::var(ENV_LLRT_NET_DENY) {
        set_deny_list(parse_list(ENV_LLRT_NET_DENY, env_value.split_whitespace())?);
    }

    Ok(())
}
