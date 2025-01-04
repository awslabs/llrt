use rquickjs::{
    atom::PredefinedAtom, function::Constructor, runtime::UserDataGuard, Ctx, Function, JsLifetime,
    Object, Result, Symbol,
};

use crate::class::CUSTOM_INSPECT_SYMBOL_DESCRIPTION;

#[derive(JsLifetime)]
pub struct BasePrimordials<'js> {
    // Constructors
    pub constructor_map: Constructor<'js>,
    pub constructor_set: Constructor<'js>,
    pub constructor_date: Constructor<'js>,
    pub constructor_error: Constructor<'js>,
    pub constructor_type_error: Constructor<'js>,
    pub constructor_range_error: Constructor<'js>,
    pub constructor_regexp: Constructor<'js>,
    pub constructor_uint8array: Constructor<'js>,
    pub constructor_array_buffer: Constructor<'js>,
    pub constructor_proxy: Constructor<'js>,
    pub constructor_object: Constructor<'js>,
    pub constructor_bool: Constructor<'js>,
    pub constructor_number: Constructor<'js>,
    pub constructor_string: Constructor<'js>,

    // Prototypes
    pub prototype_object: Object<'js>,
    pub prototype_date: Object<'js>,
    pub prototype_regexp: Object<'js>,
    pub prototype_set: Object<'js>,
    pub prototype_map: Object<'js>,
    pub prototype_error: Object<'js>,

    // Functions
    pub function_array_from: Function<'js>,
    pub function_array_buffer_is_view: Function<'js>,
    pub function_get_own_property_descriptor: Function<'js>,
    pub function_parse_int: Function<'js>,
    pub function_parse_float: Function<'js>,
    pub function_symbol_for: Function<'js>,

    // Symbols
    pub symbol_custom_inspect: Symbol<'js>,
}

pub trait Primordial<'js> {
    fn get<'a>(ctx: &'a Ctx<'js>) -> Result<UserDataGuard<'a, Self>>
    where
        Self: Sized + JsLifetime<'js>,
    {
        if let Some(primordials) = ctx.userdata::<Self>() {
            return Ok(primordials);
        }

        let primoridals = Self::new(ctx)?;

        _ = ctx.store_userdata(primoridals);
        Ok(ctx.userdata::<Self>().unwrap())
    }

    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized;
}

impl<'js> Primordial<'js> for BasePrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self> {
        let globals = ctx.globals();

        let constructor_object: Constructor = globals.get(PredefinedAtom::Object)?;
        let prototype_object: Object = constructor_object.get(PredefinedAtom::Prototype)?;

        let constructor_proxy: Constructor = globals.get(PredefinedAtom::Proxy)?;

        let function_get_own_property_descriptor: Function =
            constructor_object.get(PredefinedAtom::GetOwnPropertyDescriptor)?;

        let constructor_date: Constructor = globals.get(PredefinedAtom::Date)?;
        let prototype_date: Object = constructor_date.get(PredefinedAtom::Prototype)?;

        let constructor_map: Constructor = globals.get(PredefinedAtom::Map)?;
        let prototype_map: Object = constructor_map.get(PredefinedAtom::Prototype)?;

        let constructor_set: Constructor = globals.get(PredefinedAtom::Set)?;
        let prototype_set: Object = constructor_set.get(PredefinedAtom::Prototype)?;

        let constructor_regexp: Constructor = globals.get(PredefinedAtom::RegExp)?;
        let prototype_regexp: Object = constructor_regexp.get(PredefinedAtom::Prototype)?;

        let constructor_uint8array: Constructor = globals.get(PredefinedAtom::Uint8Array)?;
        let constructor_arraybuffer: Constructor = globals.get(PredefinedAtom::ArrayBuffer)?;

        let constructor_error: Constructor = globals.get(PredefinedAtom::Error)?;
        let constructor_type_error: Constructor = ctx.globals().get(PredefinedAtom::TypeError)?;
        let constructor_range_error: Constructor = ctx.globals().get(PredefinedAtom::RangeError)?;
        let prototype_error: Object = constructor_error.get(PredefinedAtom::Prototype)?;

        let constructor_array: Object = globals.get(PredefinedAtom::Array)?;
        let function_array_from: Function = constructor_array.get(PredefinedAtom::From)?;

        let constructor_array_buffer: Object = globals.get(PredefinedAtom::ArrayBuffer)?;
        let function_array_buffer_is_view: Function = constructor_array_buffer.get("isView")?;

        let constructor_bool: Constructor = globals.get(PredefinedAtom::Boolean)?;

        let constructor_number: Constructor = globals.get(PredefinedAtom::Number)?;
        let function_parse_float: Function = constructor_number.get("parseFloat")?;
        let function_parse_int: Function = constructor_number.get("parseInt")?;

        let constructor_string: Constructor = globals.get(PredefinedAtom::String)?;

        let constructor_symbol: Constructor = globals.get(PredefinedAtom::Symbol)?;
        let function_symbol_for: Function = constructor_symbol.get(PredefinedAtom::For)?;

        let symbol_custom_inspect: Symbol<'js> =
            function_symbol_for.call((CUSTOM_INSPECT_SYMBOL_DESCRIPTION,))?;

        Ok(Self {
            constructor_map,
            constructor_set,
            constructor_date,
            constructor_proxy,
            constructor_error,
            constructor_type_error,
            constructor_range_error,
            constructor_regexp,
            constructor_uint8array,
            constructor_array_buffer: constructor_arraybuffer,
            constructor_object,
            constructor_bool,
            constructor_number,
            constructor_string,
            prototype_object,
            prototype_date,
            prototype_regexp,
            prototype_set,
            prototype_map,
            prototype_error,
            function_array_from,
            function_array_buffer_is_view,
            function_get_own_property_descriptor,
            function_parse_float,
            function_parse_int,
            function_symbol_for,
            symbol_custom_inspect,
        })
    }
}
