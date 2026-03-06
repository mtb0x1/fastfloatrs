#![no_std]

/* This is a NOT a direct port of the fast_float library authored by Daniel Lemire
    but rather an approximation of a translation into rust (probably full of bugs)
*/
use core::str;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FfFormat: u32 {
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
    pub format: FfFormat,
    pub decimal_point: u8,
}

impl Default for FfcParseOptions {
    fn default() -> Self {
        Self {
            format: FfFormat::PRESET_GENERAL,
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
    if options.format.contains(FfFormat::SKIP_WHITE_SPACE) {
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
    } else if input[i] == b'+'
        && !options.format.contains(FfFormat::BASIC_JSON)
        && options.format.contains(FfFormat::ALLOW_LEADING_PLUS)
    {
        i += 1;
    }

    let prefix_len = i;
    let mut int_len = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
        int_len += 1;
    }

    let basic_json = options.format.contains(FfFormat::BASIC_JSON);
    if basic_json {
        if int_len == 0 || (int_len > 1 && input[prefix_len] == b'0') {
            return Err(ParseError::InvalidInput);
        }
    } else if int_len == 0 && (i == input.len() || input[i] != options.decimal_point) {
        if !options.format.contains(FfFormat::NO_INFNAN) {
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
        let is_sci = options.format.contains(FfFormat::SCIENTIFIC) && (c == b'e' || c == b'E');
        let is_fortran = options.format.contains(FfFormat::BASIC_FORTRAN)
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
                if !options.format.contains(FfFormat::FIXED) {
                    return Err(ParseError::InvalidInput);
                }
                i = exp_marker_idx;
                exp_marker_idx = usize::MAX;
            }
        } else if options.format.contains(FfFormat::SCIENTIFIC)
            && !options.format.contains(FfFormat::FIXED)
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
        } else if idx == exp_marker_idx && options.format.contains(FfFormat::BASIC_FORTRAN) {
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
            if val.is_infinite()
                || (val == T::ZERO && parsed_bytes.iter().any(|&b| b > b'0' && b <= b'9'))
            {
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
        let d7 = (chunk & 0xFF) - 0x30;

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

pub mod ffi {
    use core::ffi::c_char;

    use super::{parse_float, parse_int, FfFormat, FfcParseOptions, ParseError, FastInt};

    // --- Types & constants from api.h -----------------------------------------

    pub type FfcOutcome = u32;

    pub const FFC_OUTCOME_OK: FfcOutcome = 0;
    pub const FFC_OUTCOME_INVALID_INPUT: FfcOutcome = 1;
    pub const FFC_OUTCOME_OUT_OF_RANGE: FfcOutcome = 2;

    #[repr(C)]
    pub struct ffc_result {
        // Where parsing stopped
        pub ptr: *const c_char,
        // The outcome of the call
        pub outcome: FfcOutcome,
    }

    pub type FfcFormat = u64;

    pub const FFC_FORMAT_FLAG_SCIENTIFIC: FfcFormat = FfFormat::SCIENTIFIC.bits() as u64;
    pub const FFC_FORMAT_FLAG_FIXED: FfcFormat = FfFormat::FIXED.bits() as u64;
    pub const FFC_FORMAT_FLAG_HEX: FfcFormat = FfFormat::HEX.bits() as u64;
    pub const FFC_FORMAT_FLAG_NO_INFNAN: FfcFormat = FfFormat::NO_INFNAN.bits() as u64;
    pub const FFC_FORMAT_FLAG_BASIC_JSON: FfcFormat = FfFormat::BASIC_JSON.bits() as u64;
    pub const FFC_FORMAT_FLAG_BASIC_FORTRAN: FfcFormat = FfFormat::BASIC_FORTRAN.bits() as u64;
    pub const FFC_FORMAT_FLAG_ALLOW_LEADING_PLUS: FfcFormat = FfFormat::ALLOW_LEADING_PLUS.bits() as u64;
    pub const FFC_FORMAT_FLAG_SKIP_WHITE_SPACE: FfcFormat = FfFormat::SKIP_WHITE_SPACE.bits() as u64;

    pub const FFC_PRESET_GENERAL: FfcFormat =
        FFC_FORMAT_FLAG_FIXED | FFC_FORMAT_FLAG_SCIENTIFIC;

    pub const FFC_PRESET_JSON: FfcFormat =
        FFC_FORMAT_FLAG_BASIC_JSON | FFC_PRESET_GENERAL | FFC_FORMAT_FLAG_NO_INFNAN;

    pub const FFC_PRESET_JSON_OR_INFNAN: FfcFormat =
        FFC_FORMAT_FLAG_BASIC_JSON | FFC_PRESET_GENERAL;

    pub const FFC_PRESET_FORTRAN: FfcFormat =
        FFC_FORMAT_FLAG_BASIC_FORTRAN | FFC_PRESET_GENERAL;

    #[repr(C)]
    pub struct ffc_parse_options {
        pub format: FfcFormat,
        pub decimal_point: c_char,
    }

    // --- Internal helpers -----------------------------------------------------

    fn map_error(err: ParseError) -> FfcOutcome {
        match err {
            ParseError::InvalidInput => FFC_OUTCOME_INVALID_INPUT,
            ParseError::OutOfRange => FFC_OUTCOME_OUT_OF_RANGE,
        }
    }

    fn to_rust_options(opts: ffc_parse_options) -> FfcParseOptions {
        let format = FfFormat::from_bits_truncate(opts.format as u32);
        let dp = if opts.decimal_point as u8 == 0 {
            b'.'
        } else {
            opts.decimal_point as u8
        };
        FfcParseOptions { format, decimal_point: dp }
    }

    unsafe fn slice_from_char_ptr<'a>(start: *const c_char, end: *const c_char) -> &'a [u8] {
        let len = end.offset_from(start);
        if len <= 0 {
            return &[];
        }
        core::slice::from_raw_parts(start as *const u8, len as usize)
    }

    /// Helper to convert a Rust parsing result into an `ffc_result` while writing
    /// the output value (or its default on error) through the provided pointer.
    fn from_parse_result<T: Default + Copy>(
        res: Result<(T, &[u8]), ParseError>,
        out: *mut T,
        input_start: *const c_char,
    ) -> ffc_result {
        match res {
            Ok((value, remainder)) => {
                unsafe { *out = value };
                ffc_result {
                    ptr: remainder.as_ptr() as *const c_char,
                    outcome: FFC_OUTCOME_OK,
                }
            }
            Err(e) => {
                unsafe { *out = T::default() };
                ffc_result {
                    ptr: input_start,
                    outcome: map_error(e),
                }
            }
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_options_default() -> ffc_parse_options {
        ffc_parse_options {
            format: FFC_PRESET_GENERAL,
            decimal_point: b'.' as c_char,
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_double_simple(
        len: usize,
        input: *const c_char,
        outcome: *mut FfcOutcome,
    ) -> f64 {
        let mut out = 0.0f64;
        let res = unsafe { ffc_parse_double(len, input, &mut out) };
        if !outcome.is_null() {
            unsafe { *outcome = res.outcome };
        }
        out
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_double(
        len: usize,
        input: *const c_char,
        out: *mut f64,
    ) -> ffc_result {
        if input.is_null() || out.is_null() {
            return ffc_result { ptr: core::ptr::null(), outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        let slice = unsafe { core::slice::from_raw_parts(input as *const u8, len) };
        from_parse_result(parse_float::<f64>(slice, FfcParseOptions::default()), out, input)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_from_chars_double(
        start: *const c_char,
        end: *const c_char,
        out: *mut f64,
    ) -> ffc_result {
        ffc_from_chars_double_options(start, end, out, ffc_parse_options_default())
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_from_chars_double_options(
        start: *const c_char,
        end: *const c_char,
        out: *mut f64,
        options: ffc_parse_options,
    ) -> ffc_result {
        if start.is_null() || end.is_null() || out.is_null() {
            return ffc_result { ptr: core::ptr::null(), outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        let slice = unsafe { slice_from_char_ptr(start, end) };
        let rust_opts = to_rust_options(options);

        from_parse_result(parse_float::<f64>(slice, rust_opts), out, start)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_float_simple(
        len: usize,
        input: *const c_char,
        outcome: *mut FfcOutcome,
    ) -> f32 {
        let mut out = 0.0f32;
        let res = unsafe { ffc_parse_float(len, input, &mut out) };
        if !outcome.is_null() {
            unsafe { *outcome = res.outcome };
        }
        out
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_float(
        len: usize,
        input: *const c_char,
        out: *mut f32,
    ) -> ffc_result {
        if input.is_null() || out.is_null() {
            return ffc_result { ptr: core::ptr::null(), outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        let slice = unsafe { core::slice::from_raw_parts(input as *const u8, len) };
        from_parse_result(parse_float::<f32>(slice, FfcParseOptions::default()), out, input)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_from_chars_float(
        start: *const c_char,
        end: *const c_char,
        out: *mut f32,
    ) -> ffc_result {
        ffc_from_chars_float_options(start, end, out, ffc_parse_options_default())
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_from_chars_float_options(
        start: *const c_char,
        end: *const c_char,
        out: *mut f32,
        options: ffc_parse_options,
    ) -> ffc_result {
        if start.is_null() || end.is_null() || out.is_null() {
            return ffc_result { ptr: core::ptr::null(), outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        let slice = unsafe { slice_from_char_ptr(start, end) };
        let rust_opts = to_rust_options(options);

        from_parse_result(parse_float::<f32>(slice, rust_opts), out, start)
    }

    unsafe fn parse_int_with_base<T>(
        len: usize,
        input: *const c_char,
        base: i32,
        out: *mut T,
    ) -> ffc_result
    where
        T: FastInt,
        T: Default,
    {
        if input.is_null() || out.is_null() {
            return ffc_result { ptr: core::ptr::null(), outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        if base != 10 {
            unsafe { *out = core::mem::zeroed() };
            return ffc_result { ptr: input, outcome: FFC_OUTCOME_INVALID_INPUT };
        }

        let slice = unsafe { core::slice::from_raw_parts(input as *const u8, len) };
        from_parse_result(parse_int::<T>(slice), out, input)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_i64(
        len: usize,
        input: *const c_char,
        base: i32,
        out: *mut i64,
    ) -> ffc_result {
        parse_int_with_base::<i64>(len, input, base, out)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_u64(
        len: usize,
        input: *const c_char,
        base: i32,
        out: *mut u64,
    ) -> ffc_result {
        parse_int_with_base::<u64>(len, input, base, out)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_i32(
        len: usize,
        input: *const c_char,
        base: i32,
        out: *mut i32,
    ) -> ffc_result {
        parse_int_with_base::<i32>(len, input, base, out)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn ffc_parse_u32(
        len: usize,
        input: *const c_char,
        base: i32,
        out: *mut u32,
    ) -> ffc_result {
        parse_int_with_base::<u32>(len, input, base, out)
    }
}