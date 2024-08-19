use llrt_utils::primordials::{BasePrimordials, Primordial};
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{atom::PredefinedAtom, function::Opt, Class, Ctx, Object, Result};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct DOMException {
    message: String,
    name: String,
    stack: String,
    code: u8,
}

#[rquickjs::methods]
impl DOMException {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, message: Opt<String>, name: Opt<String>) -> Result<Self> {
        let primordials = BasePrimordials::get(&ctx)?;

        let new: Object = primordials
            .constructor_error
            .construct((message.clone(),))?;

        let message = message.0.unwrap_or(String::from(""));
        let name = name.0.unwrap_or(String::from("Error"));

        // https://webidl.spec.whatwg.org/#dfn-error-names-table
        let code = match name.as_str() {
            "IndexSizeError" => 1,
            "HierarchyRequestError" => 3,
            "WrongDocumentError" => 4,
            "InvalidCharacterError" => 5,
            "NoModificationAllowedError" => 7,
            "NotFoundError" => 8,
            "NotSupportedError" => 9,
            "InUseAttributeError" => 10,
            "InvalidStateError" => 11,
            "SyntaxError" => 12,
            "InvalidModificationError" => 13,
            "NamespaceError" => 14,
            "InvalidAccessError" => 15,
            "TypeMismatchError" => 17,
            "SecurityError" => 18,
            "NetworkError" => 19,
            "AbortError" => 20,
            "URLMismatchError" => 21,
            "QuotaExceededError" => 22,
            "TimeoutError" => 23,
            "InvalidNodeTypeError" => 24,
            "DataCloneError" => 25,
            _ => 0,
        };

        Ok(Self {
            message,
            name,
            code,
            stack: new.get::<_, String>(PredefinedAtom::Stack)?,
        })
    }

    #[qjs(get)]
    fn message(&self) -> String {
        self.message.clone()
    }

    #[qjs(get)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[qjs(get)]
    pub fn code(&self) -> u8 {
        self.code
    }

    #[qjs(get)]
    fn stack(&self) -> String {
        self.stack.clone()
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    pub fn to_string(&self) -> String {
        if self.message.is_empty() {
            return self.name.clone();
        }

        [self.name.as_str(), self.message.as_str()].join(": ")
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    Class::<DOMException>::define(&globals)?;

    let dom_ex_proto = Class::<DOMException>::prototype(ctx)?.unwrap();
    let error_prototype = &BasePrimordials::get(ctx)?.prototype_error;
    dom_ex_proto.set_prototype(Some(error_prototype))?;

    Ok(())
}
