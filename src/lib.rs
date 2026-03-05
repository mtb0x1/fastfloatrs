#![no_std]

use core::str;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FfcFormat: u32 {
        const SCIENTIFIC         = 1 << 0;
        const FIXED              = 1 << 2;
        const HEX                = 1 << 3;
        const NO_INFNAN          = 1 << 4;
        const BASIC_JSON         = 1 << 5;
        const BASIC_FORTRAN      = 1 << 6;
        const ALLOW_LEADING_PLUS = 1 << 7;
        const SKIP_WHITE_SPACE   = 1 << 8;

        const PRESET_GENERAL = Self::FIXED.bits() | Self::SCIENTIFIC.bits();
        const PRESET_JSON = Self::BASIC_JSON.bits() | Self::PRESET_GENERAL.bits() | Self::NO_INFNAN.bits();
        const PRESET_JSON_OR_INFNAN = Self::BASIC_JSON.bits() | Self::PRESET_GENERAL.bits();
        const PRESET_FORTRAN = Self::BASIC_FORTRAN.bits() | Self::PRESET_GENERAL.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    InvalidInput,
    OutOfRange,
}

#[derive(Debug, Clone, Copy)]
pub struct FfcParseOptions {
    pub format: FfcFormat,
    pub decimal_point: u8,
}

impl Default for FfcParseOptions {
    fn default() -> Self {
        Self {
            format: FfcFormat::PRESET_GENERAL,
            decimal_point: b'.',
        }
    }
}

pub trait FastFloat: Sized + core::str::FromStr + Copy + PartialEq {
    const NAN: Self;
    const INFINITY: Self;
    const NEG_INFINITY: Self;
    const ZERO: Self;
    fn is_infinite(self) -> bool;
}

impl FastFloat for f32 {
    const NAN: Self = f32::NAN;
    const INFINITY: Self = f32::INFINITY;
    const NEG_INFINITY: Self = f32::NEG_INFINITY;
    const ZERO: Self = 0.0;
    fn is_infinite(self) -> bool {
        self.is_infinite()
    }
}

impl FastFloat for f64 {
    const NAN: Self = f64::NAN;
    const INFINITY: Self = f64::INFINITY;
    const NEG_INFINITY: Self = f64::NEG_INFINITY;
    const ZERO: Self = 0.0;
    fn is_infinite(self) -> bool {
        self.is_infinite()
    }
}

pub fn parse_float<T: FastFloat>(
    mut input: &[u8],
    options: FfcParseOptions,
) -> Result<(T, &[u8]), ParseError> {
    if options.format.contains(FfcFormat::SKIP_WHITE_SPACE) {
        while let Some(&b) = input.first() {
            if b.is_ascii_whitespace() {
                input = &input[1..];
            } else {
                break;
            }
        }
    }

    if input.is_empty() {
        return Err(ParseError::InvalidInput);
    }

    let mut i = 0;
    let mut neg = false;
    if input[i] == b'-' {
        neg = true;
        i += 1;
    } else if input[i] == b'+' {
        if !options.format.contains(FfcFormat::BASIC_JSON)
            && options.format.contains(FfcFormat::ALLOW_LEADING_PLUS)
        {
            i += 1;
        }
    }

    let prefix_len = i;
    let mut int_len = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
        int_len += 1;
    }

    let basic_json = options.format.contains(FfcFormat::BASIC_JSON);
    if basic_json {
        if int_len == 0 || (int_len > 1 && input[prefix_len] == b'0') {
            return Err(ParseError::InvalidInput);
        }
    } else if int_len == 0 && (i == input.len() || input[i] != options.decimal_point) {
        if !options.format.contains(FfcFormat::NO_INFNAN) {
            let sub = &input[prefix_len..];
            let is_match =
                |s: &[u8], p: &[u8]| s.len() >= p.len() && s[..p.len()].eq_ignore_ascii_case(p);

            if is_match(sub, b"nan") {
                let mut ptr = &sub[3..];
                if ptr.first() == Some(&b'(') {
                    let mut end_idx = 1;
                    while end_idx < ptr.len() {
                        let c = ptr[end_idx];
                        if c == b')' {
                            ptr = &ptr[end_idx + 1..];
                            break;
                        } else if c.is_ascii_alphanumeric() || c == b'_' {
                            end_idx += 1;
                        } else {
                            break;
                        }
                    }
                }
                return Ok((T::NAN, ptr));
            } else if is_match(sub, b"infinity") {
                return Ok((if neg { T::NEG_INFINITY } else { T::INFINITY }, &sub[8..]));
            } else if is_match(sub, b"inf") {
                return Ok((if neg { T::NEG_INFINITY } else { T::INFINITY }, &sub[3..]));
            }
        }
        return Err(ParseError::InvalidInput);
    }

    let mut fraction_len = 0;
    if i < input.len() && input[i] == options.decimal_point {
        i += 1;
        while i < input.len() && input[i].is_ascii_digit() {
            i += 1;
            fraction_len += 1;
        }
        if basic_json && fraction_len == 0 {
            return Err(ParseError::InvalidInput);
        }
    }

    if int_len == 0 && fraction_len == 0 {
        return Err(ParseError::InvalidInput);
    }

    let mut exp_marker_idx = usize::MAX;
    if i < input.len() {
        let c = input[i];
        let is_sci = options.format.contains(FfcFormat::SCIENTIFIC) && (c == b'e' || c == b'E');
        let is_fortran = options.format.contains(FfcFormat::BASIC_FORTRAN)
            && (c == b'+' || c == b'-' || c == b'd' || c == b'D');

        if is_sci || is_fortran {
            exp_marker_idx = i;
            if c == b'e' || c == b'E' || c == b'd' || c == b'D' {
                i += 1;
            }
            if i < input.len() && (input[i] == b'-' || input[i] == b'+') {
                i += 1;
            }

            let exp_start = i;
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
            }

            if i == exp_start {
                if !options.format.contains(FfcFormat::FIXED) {
                    return Err(ParseError::InvalidInput);
                }
                i = exp_marker_idx;
                exp_marker_idx = usize::MAX;
            }
        } else if options.format.contains(FfcFormat::SCIENTIFIC)
            && !options.format.contains(FfcFormat::FIXED)
        {
            return Err(ParseError::InvalidInput);
        }
    }

    let parsed_bytes = &input[..i];
    let remaining = &input[i..];

    let mut buf = [0u8; 128];
    if parsed_bytes.len() > buf.len() {
        return Err(ParseError::OutOfRange);
    }

    for (idx, &b) in parsed_bytes.iter().enumerate() {
        if b == options.decimal_point {
            buf[idx] = b'.';
        } else if idx == exp_marker_idx && options.format.contains(FfcFormat::BASIC_FORTRAN) {
            if b == b'+' || b == b'-' || b == b'd' || b == b'D' {
                buf[idx] = b'e';
            } else {
                buf[idx] = b;
            }
        } else {
            buf[idx] = b;
        }
    }

    // SAFE: We verified above that `parsed_bytes` only contains ASCII digits, signs, standard letters.
    let s = unsafe { str::from_utf8_unchecked(&buf[..parsed_bytes.len()]) };

    match s.parse::<T>() {
        Ok(val) => {
            if val.is_infinite() {
                Err(ParseError::OutOfRange)
            } else if val == T::ZERO && parsed_bytes.iter().any(|&b| b > b'0' && b <= b'9') {
                Err(ParseError::OutOfRange)
            } else {
                Ok((val, remaining))
            }
        }
        Err(_) => Err(ParseError::OutOfRange),
    }
}

// Basic SWAR Integer Parsing trait (SIMD Within A Register)
pub trait FastInt: Sized + Copy + core::cmp::PartialOrd {
    const IS_SIGNED: bool;
    fn from_u64(val: u64, neg: bool) -> Option<Self>;
}

macro_rules! impl_fast_int {
    ($t:ty, $is_signed:expr) => {
        impl FastInt for $t {
            const IS_SIGNED: bool = $is_signed;
            #[inline]
            fn from_u64(val: u64, neg: bool) -> Option<Self> {
                let max_val = if $is_signed {
                    if neg {
                        (<$t>::MIN as i64).unsigned_abs() as u64
                    } else {
                        <$t>::MAX as u64
                    }
                } else {
                    <$t>::MAX as u64
                };
                if val > max_val {
                    return None;
                }
                Some(if neg {
                    val.wrapping_neg() as $t
                } else {
                    val as $t
                })
            }
        }
    };
}

impl_fast_int!(i32, true);
impl_fast_int!(u32, false);
impl_fast_int!(i64, true);
impl_fast_int!(u64, false);

pub fn parse_int<T: FastInt>(mut input: &[u8]) -> Result<(T, &[u8]), ParseError> {
    if input.is_empty() {
        return Err(ParseError::InvalidInput);
    }

    let mut neg = false;
    if input[0] == b'-' {
        if !T::IS_SIGNED {
            return Err(ParseError::InvalidInput);
        }
        neg = true;
        input = &input[1..];
    } else if input[0] == b'+' {
        input = &input[1..];
    }

    if input.is_empty() || !input[0].is_ascii_digit() {
        return Err(ParseError::InvalidInput);
    }

    let mut val: u64 = 0;

    // SWAR fast path for 8 digits chunk
    while input.len() >= 8 {
        let chunk = u64::from_be_bytes([
            input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7],
        ]);

        // Check if all 8 bytes are ASCII digits
        if (chunk.wrapping_add(0x4646464646464646) | chunk.wrapping_sub(0x3030303030303030))
            & 0x8080808080808080
            != 0
        {
            // Fallback to byte-by-byte
            break;
        }

        // Convert 8 ASCII digits to a u32 value using proper digit extraction
        // Each byte contains a digit, we need to compute:
        // d0*10^7 + d1*10^6 + d2*10^5 + d3*10^4 + d4*10^3 + d5*10^2 + d6*10 + d7
        let d0 = ((chunk >> 56) & 0xFF) - 0x30;
        let d1 = ((chunk >> 48) & 0xFF) - 0x30;
        let d2 = ((chunk >> 40) & 0xFF) - 0x30;
        let d3 = ((chunk >> 32) & 0xFF) - 0x30;
        let d4 = ((chunk >> 24) & 0xFF) - 0x30;
        let d5 = ((chunk >> 16) & 0xFF) - 0x30;
        let d6 = ((chunk >> 8) & 0xFF) - 0x30;
        let d7 = ((chunk >> 0) & 0xFF) - 0x30;

        let parsed_chunk = d0 * 10_000_000
            + d1 * 1_000_000
            + d2 * 100_000
            + d3 * 10_000
            + d4 * 1_000
            + d5 * 100
            + d6 * 10
            + d7;

        if let Some(new_val) = val
            .checked_mul(100_000_000)
            .and_then(|v| v.checked_add(parsed_chunk))
        {
            val = new_val;
            input = &input[8..];
        } else {
            return Err(ParseError::OutOfRange);
        }
    }

    // Trailing bytes (or if < 8 digits total)
    for &b in input.iter() {
        if b.is_ascii_digit() {
            let digit = (b - b'0') as u64;
            val = val
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
                .ok_or(ParseError::OutOfRange)?;
            input = &input[1..];
        } else {
            break;
        }
    }

    T::from_u64(val, neg)
        .map(|v| (v, input))
        .ok_or(ParseError::OutOfRange)
}
