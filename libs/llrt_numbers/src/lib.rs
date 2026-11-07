use llrt_utils::object::ObjectExt;
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, This},
    prelude::Func,
    Ctx, Exception, FromJs, Function, Object, Result, Value,
};

const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const BUF_SIZE: usize = 80;
const BIN_MAX_DIGITS: usize = 64;
const OCT_MAX_DIGITS: usize = 21;
const MAX_FRACTION_DIGITS: usize = 1100;

pub fn redefine_prototype(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let number: Function = globals.get(PredefinedAtom::Number)?;
    let number_proto: Object = number.get(PredefinedAtom::Prototype)?;
    number_proto.set(PredefinedAtom::ToString, Func::from(number_to_string))?;
    Ok(())
}

#[inline(always)]
pub fn to_dec(number: i64) -> String {
    itoa::Buffer::new().format(number).into()
}

#[inline(always)]
pub fn to_base_less_than_10(buf: &mut [u8], num: i64, base: i64) -> String {
    let max = buf.len();

    if num == 0 {
        return "0".into();
    }
    let mut index = max;
    let mut n = num;

    let mut string = String::with_capacity(max + 1);

    if n < 0 {
        n = !n + 1;
        string.push('-');
    }

    while n > 0 {
        index -= 1;
        buf[index] = (n % base) as u8 + b'0';
        n /= base;
    }

    string.push_str(unsafe { std::str::from_utf8_unchecked(&buf[index..max]) });
    string
}

pub fn i64_to_base_n(number: i64, radix: u8) -> String {
    match radix {
        10 => {
            return to_dec(number);
        },
        2 => {
            let mut buf = [0u8; BIN_MAX_DIGITS];
            return to_base_less_than_10(&mut buf, number, 2);
        },
        8 => {
            let mut buf = [0u8; OCT_MAX_DIGITS];
            return to_base_less_than_10(&mut buf, number, 8);
        },
        _ => {},
    }

    let mut abs_number = number;

    let mut buf = [0u8; BUF_SIZE];
    let mut string = String::with_capacity(BUF_SIZE);
    if number < 0 {
        abs_number = -number;
        string.push('-');
    }

    let index = internal_i64_to_base_n(&mut buf, abs_number, radix);

    string.push_str(unsafe { std::str::from_utf8_unchecked(&buf[index..BUF_SIZE]) });
    string
}

#[inline(always)]
fn internal_i64_to_base_n(buf: &mut [u8], number: i64, radix: u8) -> usize {
    if number == 0 {
        buf[BUF_SIZE - 1] = DIGITS[0];
        return BUF_SIZE - 1;
    }

    let mut n = number;
    let mut index = BUF_SIZE;

    while n > 0 {
        index -= 1;
        let digit = n % radix as i64;
        buf[index] = DIGITS[digit as usize];
        n /= radix as i64;
    }

    index
}

#[inline(always)]
fn next_up(num: f64) -> f64 {
    const TINY_BITS: u64 = 0x1;
    const CLEAR_SIGN_MASK: u64 = 0x7fff_ffff_ffff_ffff;

    let bits = num.to_bits();
    if num.is_nan() || bits == f64::INFINITY.to_bits() {
        return num;
    }

    let abs = bits & CLEAR_SIGN_MASK;
    let next_bits = if abs == 0 {
        TINY_BITS
    } else if bits == abs {
        bits + 1
    } else {
        bits - 1
    };
    f64::from_bits(next_bits)
}

#[inline(always)]
fn fractional_to_base(mut number: f64, radix: u8) -> String {
    let mut buf = Vec::with_capacity(BUF_SIZE);
    let mut is_odd = number <= 0x1fffffffffffffi64 as f64 && (number as i64) & 1 != 0;
    let mut digit;

    //let mut needs_rounding_up = false;

    let next_number = next_up(number);
    let mut delta_next_double = next_number - number;

    loop {
        let ntmp = number * radix as f64;
        let rtmp = delta_next_double * radix as f64;
        digit = ntmp as usize;
        let ritmp = rtmp as usize;

        if digit & 1 != 0 {
            is_odd = !is_odd;
        }

        number = ntmp - digit as f64;
        delta_next_double = rtmp - ritmp as f64;

        if number == 0.0 {
            buf.push(DIGITS[digit]);
            break;
        }

        if number > 0.5f64 || number == 0.5f64 && if radix & 1 > 0 { is_odd } else { digit & 1 > 0 }
        {
            if number + delta_next_double > 1.0 {
                //TODO impl round up
                break;
            }
        } else if number < delta_next_double * 2.0 {
            break;
        }
        buf.push(DIGITS[digit]);

        if buf.len() >= MAX_FRACTION_DIGITS {
            break;
        }
    }

    unsafe { String::from_utf8_unchecked(buf) }
}

#[inline(always)]
fn integer_limbs(value: f64) -> Vec<u32> {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let shift = exponent - 1075;

    if shift >= 0 {
        let word = (shift / 32) as usize;
        let bit = (shift % 32) as u32;
        let source = [mantissa as u32, (mantissa >> 32) as u32];
        let mut limbs = vec![0u32; word + 3];
        for (index, source) in source.into_iter().enumerate() {
            let value = (source as u64) << bit;
            limbs[index + word] |= value as u32;
            limbs[index + word + 1] |= (value >> 32) as u32;
        }
        limbs
    } else {
        let shift = (-shift) as u32;
        if shift >= 64 {
            vec![0]
        } else {
            let value = mantissa >> shift;
            vec![value as u32, (value >> 32) as u32]
        }
    }
}

fn integer_to_base_n(value: f64, radix: u8) -> String {
    let mut limbs = integer_limbs(value);
    let mut digits = Vec::with_capacity(BUF_SIZE);

    loop {
        let mut remainder = 0u64;
        for limb in limbs.iter_mut().rev() {
            let current = (remainder << 32) | u64::from(*limb);
            *limb = (current / radix as u64) as u32;
            remainder = current % radix as u64;
        }
        digits.push(DIGITS[remainder as usize]);

        while limbs.len() > 1 && limbs[limbs.len() - 1] == 0 {
            limbs.pop();
        }
        if limbs.len() == 1 && limbs[0] == 0 {
            break;
        }
    }

    digits.reverse();
    unsafe { String::from_utf8_unchecked(digits) }
}

#[inline(always)]
fn f64_to_base_n(number: f64, radix: u8) -> String {
    const EXP_MASK: u64 = 0x7ff0000000000000;
    let bits = number.to_bits();
    if bits & EXP_MASK == EXP_MASK {
        return get_nonfinite(bits).to_string();
    }

    if radix == 10 {
        let mut result = ryu::Buffer::new().format_finite(number).to_string();
        if result.ends_with(".0") {
            result.truncate(result.len() - 2);
        }
        return result;
    }

    let mut abs_num = number;
    let mut string = String::with_capacity(BUF_SIZE);

    if number < 0.0 {
        abs_num = -number;
        string.push('-');
    }

    let integer_part = abs_num.trunc();
    let fractional_part = abs_num - integer_part;

    if integer_part < 9_223_372_036_854_775_808.0 {
        string.push_str(&i64_to_base_n(integer_part as i64, radix));
    } else {
        string.push_str(&integer_to_base_n(integer_part, radix));
    }

    if fractional_part > 0.0 {
        let fraction = fractional_to_base(fractional_part, radix);
        if !fraction.is_empty() {
            string.push('.');
            string.push_str(&fraction);
        }
    }
    string
}

pub fn float_to_string(float: f64) -> String {
    f64_to_base_n(float, 10)
}

#[inline(always)]
#[cold]
fn get_nonfinite<'a>(bits: u64) -> &'a str {
    const MANTISSA_MASK: u64 = 0x000fffffffffffff;
    const SIGN_MASK: u64 = 0x8000000000000000;
    if bits & MANTISSA_MASK != 0 {
        "NaN"
    } else if bits & SIGN_MASK != 0 {
        "-Infinity"
    } else {
        "Infinity"
    }
}

#[inline(always)]
#[cold]
fn check_radix(ctx: &Ctx, radix: u8) -> Result<()> {
    if !(2..=36).contains(&radix) {
        return Err(Exception::throw_type(ctx, "radix must be between 2 and 36"));
    }
    Ok(())
}

fn number_to_string<'js>(
    ctx: Ctx<'js>,
    this: This<Value<'js>>,
    radix: Opt<Value>,
) -> Result<String> {
    let radix = radix
        .0
        .and_then(|v| v.as_int())
        .and_then(|r| u8::try_from(r).ok())
        .unwrap_or(10);
    check_radix(&ctx, radix)?;

    // handle primitive number values
    if let Some(float) = this.as_float() {
        return Ok(f64_to_base_n(float, radix));
    }
    if let Ok(int) = i64::from_js(&ctx, this.0.clone()) {
        return Ok(i64_to_base_n(int, radix));
    }

    // handle boxed Number objects
    if let Some(obj) = this.as_object() {
        if let Some(value_of) = obj.get_optional::<_, Function>(PredefinedAtom::ValueOf)? {
            let primitive: Value = value_of.call((this,))?;

            if let Some(float) = primitive.as_float() {
                return Ok(f64_to_base_n(float, radix));
            }
            if let Ok(int) = i64::from_js(&ctx, primitive) {
                return Ok(i64_to_base_n(int, radix));
            }
        }
    }

    Ok("".into())
}

#[cfg(test)]
mod test {

    use rand::RngExt;

    use crate::{f64_to_base_n, float_to_string, i64_to_base_n};

    #[test]
    fn test_large_and_small_base_conversions() {
        assert_eq!(f64_to_base_n(1e21, 16), "3635c9adc5dea00000");
        assert_eq!(f64_to_base_n(18_446_744_073_709_551_616.0, 2).len(), 65);
        assert_eq!(
            f64_to_base_n(1e-35, 16),
            "0.00000000000000000000000000000d4ad2dbfc3d078"
        );
        assert_eq!(
            f64_to_base_n(f64::MIN_POSITIVE, 16),
            ["0.", &"0".repeat(255), "4"].concat()
        );
        assert_eq!(f64_to_base_n(-1e21, 16), "-3635c9adc5dea00000");
        assert_eq!(f64_to_base_n(0.0, 16), "0");
        assert_eq!(f64_to_base_n(-0.0, 16), "0");
    }

    #[test]
    fn test_base_conversions() {
        let mut rng = rand::rng();

        for _ in 0..1_000_000 {
            // Generate random i64 and radix values
            let num: i64 = rng.random_range(i64::MIN + 1..i64::MAX - 1);

            let minus_str = if num < 0 { "-" } else { "" };

            //test bin
            let expected_bin = format!("{}{:b}", minus_str, num.abs());
            let actual_bin = i64_to_base_n(num, 2);
            assert_eq!(expected_bin, actual_bin);

            //test octal
            let expected_octal = format!("{}{:o}", minus_str, num.abs());
            let actual_octal = i64_to_base_n(num, 8);
            assert_eq!(expected_octal, actual_octal);

            //test hex
            let expected_hex = format!("{}{:x}", minus_str, num.abs());
            let actual_hex = i64_to_base_n(num, 16);
            assert_eq!(expected_hex, actual_hex);
        }

        // Test i64_to_base_n
        let base_36 = i64_to_base_n(0, 36);
        assert_eq!("0", base_36);

        let base_36 = i64_to_base_n(123456789, 36);
        assert_eq!("21i3v9", base_36);

        let base_36 = i64_to_base_n(-123456789, 36);
        assert_eq!("-21i3v9", base_36);

        let float = float_to_string(123.456);
        assert_eq!("123.456", float);

        let float = float_to_string(123.);
        assert_eq!("123", float);

        let float = float_to_string(0.0);
        assert_eq!("0", float);
    }
}
