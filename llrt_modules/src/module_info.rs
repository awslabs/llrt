// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::module::ModuleDef;

pub struct ModuleInfo<T: ModuleDef> {
    pub name: &'static str,
    pub module: T,
}
