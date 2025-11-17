// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use self::base::*;

#[allow(clippy::module_inception)]
mod base {
    pub use llrt_core::bytecode;
    #[cfg(not(feature = "lambda"))]
    pub use llrt_core::compiler;
    pub use llrt_core::environment;
    pub use llrt_core::libs;
    pub use llrt_core::modules;
    pub use llrt_core::vm;
}
pub use llrt_core::VERSION;

// rquickjs components
#[allow(unused_imports)]
pub use llrt_core::{
    async_with, atom::PredefinedAtom, context::EvalOptions, function::Rest, runtime_client,
    AsyncContext, CatchResultExt, Ctx, Error, Object, Promise,
};
