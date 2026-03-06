#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::os::raw::c_char;

    use fastfloatrs::ffi::{
        FfcOutcome, ffc_parse_options, ffc_parse_options_default, ffc_result,
        FFC_OUTCOME_OK,
    };
    #[allow(unsafe_code)]
    unsafe extern "C" {
        unsafe fn ffc_parse_double(len: usize, input: *const c_char, out: *mut f64) -> ffc_result;
        unsafe fn ffc_parse_double_simple(
            len: usize, 
            input: *const c_char,
            outcome: *mut FfcOutcome,
        ) -> f64;

        unsafe fn ffc_parse_float(len: usize, input: *const c_char, out: *mut f32) -> ffc_result;

        unsafe fn ffc_parse_i32(
            len: usize,
            input: *const c_char,
            base: i32,
            out: *mut i32,
        ) -> ffc_result;
    }

    #[test]
    fn ffi_parse_double_basic() {
        let s = CString::new("123.456e-2").unwrap();
        let mut out = 0.0f64;

        let res = unsafe { ffc_parse_double(10, s.as_ptr(), &mut out) };

        assert_eq!(res.outcome, FFC_OUTCOME_OK);
        assert!((out - 1.23456).abs() < 1e-12);
    }

    #[test]
    fn ffi_parse_double_simple_sets_outcome() {
        let s = CString::new("not-a-number").unwrap();
        let mut outcome: FfcOutcome = 0;

        let _ = unsafe { ffc_parse_double_simple(11, s.as_ptr(), &mut outcome) };

        assert_ne!(outcome, FFC_OUTCOME_OK);
    }

    #[test]
    fn ffi_parse_float_basic() {
        let s = CString::new("1.5").unwrap();
        let mut out = 0.0f32;

        let res = unsafe { ffc_parse_float(3, s.as_ptr(), &mut out) };

        assert_eq!(res.outcome, FFC_OUTCOME_OK);
        assert!((out - 1.5).abs() < 1e-6);
    }

    #[test]
    fn ffi_parse_i32_base10_and_remainder() {
        let s = CString::new("42xyz").unwrap();
        let mut out = 0i32;

        let res = unsafe { ffc_parse_i32(5, s.as_ptr(), 10, &mut out) };

        assert_eq!(res.outcome, FFC_OUTCOME_OK);
        assert_eq!(out, 42);
        let first_rem = unsafe { *res.ptr } as u8;
        assert_eq!(first_rem, b'x');
    }

    #[test]
    fn ffi_parse_i32_non_decimal_rejected() {
        let s = CString::new("ff").unwrap();
        let mut out = 0i32;

        let res = unsafe { ffc_parse_i32(2, s.as_ptr(), 16, &mut out) };

        assert_ne!(res.outcome, FFC_OUTCOME_OK);
    }

    #[test]
    fn ffi_parse_options_default_sane() {
        let opts: ffc_parse_options = unsafe { ffc_parse_options_default() };
        assert_eq!(opts.decimal_point as u8, b'.');
    }
}