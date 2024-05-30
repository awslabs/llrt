// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    function::{Opt, This},
    Ctx, Exception, Result, Value,
};
use std::{fmt::Write, result::Result as StdResult};

const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const BUF_SIZE: usize = 81;

macro_rules! write_formatted {
    ($format:expr, $number:expr) => {{
        let digits = ($number as f64).log10() as usize + 2;
        let mut string = String::with_capacity(digits);
        write!(string, $format, $number).unwrap();
        string
    }};
}

fn i64_to_base_n(number: i64, radix: u8) -> String {
    match radix {
        2 => return write_formatted!("{:b}", number),
        8 => return write_formatted!("{:o}", number),
        10 => return write_formatted!("{}", number),
        16 => return write_formatted!("{:x}", number),
        _ => {},
    }

    let mut positive_number = number;
    let mut index = 0;
    let mut buf = [0_u8; BUF_SIZE];
    if number < 0 {
        positive_number = -number;
        index = 1;
        buf[0] = b'-';
    }

    let index = internal_i64_to_base_n(&mut buf, index, positive_number, radix);
    String::from_utf8_lossy(&buf[..index]).into_owned()
}

#[inline(always)]
fn internal_i64_to_base_n(
    buf: &mut [u8; BUF_SIZE],
    start_index: usize,
    number: i64,
    radix: u8,
) -> usize {
    let mut n = number;
    let mut end_index = BUF_SIZE - 1;
    let mut index = start_index;

    while n > 0 {
        let digit = n % radix as i64;
        buf[end_index] = DIGITS[digit as usize];
        n /= radix as i64;
        end_index -= 1;
        index += 1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            buf.as_ptr().add(end_index + 1),
            buf.as_mut_ptr().add(start_index),
            index,
        )
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
fn fractional_to_base(
    buf: &mut [u8; BUF_SIZE],
    mut index: usize,
    mut number: f64,
    radix: u8,
) -> usize {
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

        if number > 0.5f64 || number == 0.5f64 && if radix & 1 > 0 { is_odd } else { digit & 1 > 0 }
        {
            if number + delta_next_double > 1.0 {
                //TODO impl round up
                break;
            }
        } else if number < delta_next_double * 2.0 {
            break;
        }
        buf[index] = DIGITS[digit];

        index += 1;
    }

    // let last_index = index;
    // while number > 0.0 {
    //     let tmp = number * radix as f64;
    //     let itmp = tmp as usize;
    //     buf[index] = DIGITS[itmp];
    //     number = tmp - itmp as f64;
    //     index += 1;
    //     if index - last_index > BUF_SIZE - last_index - 1 {
    //         break;
    //     }
    // }
    index
}

#[inline(always)]
fn f64_to_base_n(number: f64, radix: u8) -> String {
    let mut positive_num = number;
    let mut index = 0;
    let mut buf = [0_u8; BUF_SIZE];
    if number < 0.0 {
        positive_num = -number;
        index = 1;
        buf[0] = b'-';
    }

    let integer_part = positive_num.trunc();
    let fractional_part = positive_num - integer_part;
    let integer_part = positive_num as i64;

    index = internal_i64_to_base_n(&mut buf, index, integer_part, radix);
    let dot_index = index;
    index = fractional_to_base(&mut buf, index + 1, fractional_part, radix);
    if index - 1 > dot_index {
        buf[dot_index] = b'.';
    }

    String::from(unsafe { std::str::from_utf8_unchecked(&buf[..index]) })
}

pub fn number_to_string(ctx: Ctx, this: This<Value>, radix: Opt<u8>) -> Result<String> {
    if let Some(int) = this.as_int() {
        if let Some(radix) = radix.0 {
            check_radix(&ctx, radix)?;
            return Ok(i64_to_base_n(int as i64, radix));
        }
        return Ok(write_formatted!("{}", int));
    }
    if let Some(float) = this.as_float() {
        if let Some(radix) = radix.0 {
            check_radix(&ctx, radix)?;
            return Ok(f64_to_base_n(float, radix));
        }

        let mut buffer = ryu::Buffer::new();
        return float_to_string(&mut buffer, float).map(|f| f.into());
    }
    Ok("".into())
}

pub fn float_to_string(buffer: &mut ryu::Buffer, float: f64) -> Result<&str> {
    let str = match float_to_str(buffer, float) {
        Ok(value) => value,
        Err(value) => return Ok(value),
    };
    let len = str.len();
    if unsafe { str.get_unchecked(str.len() - 2..) } == ".0" {
        let bytes = str.as_bytes();

        return Ok(unsafe { std::str::from_utf8_unchecked(&bytes[..len - 2]) });
    }
    Ok(str)
}

/// Returns a string representation of the float value.
///
/// Returns error with a `str` if value is non-finite
#[inline(always)]
pub fn float_to_str(buf: &mut ryu::Buffer, float: f64) -> StdResult<&str, &str> {
    const EXP_MASK: u64 = 0x7ff0000000000000;
    let bits = float.to_bits();
    if bits & EXP_MASK == EXP_MASK {
        return Err(get_nonfinite(bits));
    }

    let str = buf.format_finite(float);
    Ok(str)
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
        return Err(Exception::throw_message(
            ctx,
            "radix must be between 2 and 36",
        ));
    }
    Ok(())
}
