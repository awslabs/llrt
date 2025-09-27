// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//network
pub const ENV_LLRT_NET_ALLOW: &str = "LLRT_NET_ALLOW";
pub const ENV_LLRT_NET_DENY: &str = "LLRT_NET_DENY";
pub const ENV_LLRT_NET_POOL_IDLE_TIMEOUT: &str = "LLRT_NET_POOL_IDLE_TIMEOUT";
pub const ENV_LLRT_HTTP_VERSION: &str = "LLRT_HTTP_VERSION";
pub const ENV_LLRT_TLS_VERSION: &str = "LLRT_TLS_VERSION";
pub const ENV_LLRT_EXTRA_CA_CERTS: &str = "LLRT_EXTRA_CA_CERTS";

//log
pub const ENV_LLRT_LOG: &str = "LLRT_LOG";

//module
pub const ENV_LLRT_PLATFORM: &str = "LLRT_PLATFORM";

//llrt
pub const ENV_LLRT_PSEUDO_V8_STATS: &str = "LLRT_PSEUDO_V8_STATS";

//vm
pub const ENV_LLRT_GC_THRESHOLD_MB: &str = "LLRT_GC_THRESHOLD_MB";

//runtime client
pub const ENV_LLRT_SDK_CONNECTION_WARMUP: &str = "LLRT_SDK_CONNECTION_WARMUP";
