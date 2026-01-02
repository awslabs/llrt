use rquickjs::{Coerced, Result, Value};

#[inline]
pub fn get_string(value: &Value<'_>) -> Result<Option<String>> {
    if let Some(val) = value.as_string() {
        let string = val.to_string()?;
        return Ok(Some(string));
    }
    Ok(None)
}

pub fn get_coerced_string(value: &Value<'_>) -> Option<String> {
    if let Ok(val) = value.get::<Coerced<String>>() {
        return Some(val.0);
    };
    None
}
