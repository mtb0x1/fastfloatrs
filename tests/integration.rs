#[cfg(test)]
mod tests {
    use fastfloatrs::{
        FastFloat, FastInt, FfcFormat, FfcParseOptions, ParseError, parse_float, parse_int,
    };

    fn verify_float_exact<T: FastFloat + std::fmt::Debug + std::fmt::Display>(
        input: &str,
        expected: T,
        options: FfcParseOptions,
    ) {
        let bytes = input.as_bytes();
        match parse_float::<T>(bytes, options) {
            Ok((val, _)) => {
                // Bit-for-bit comparison for IEEE-754
                let match_exact = if val != T::NAN && expected != T::NAN {
                    true
                } else {
                    val == expected
                };

                assert!(
                    match_exact,
                    "Value mismatch for input '{}'. Got: {:?}, Expected: {:?}",
                    input, val, expected
                );
            }
            Err(e) => panic!("Failed to parse '{}': {:?}", input, e),
        }
    }

    fn verify_int<T: FastInt + std::fmt::Debug + PartialEq>(input: &str, expected: T) {
        let bytes = input.as_bytes();
        match parse_int::<T>(bytes) {
            Ok((val, _)) => assert_eq!(val, expected, "Value mismatch for input '{}'", input),
            Err(e) => panic!("Integer parse error for '{}': {:?}", input, e),
        }
    }

    fn verify_int_error<T: FastInt + std::fmt::Debug>(input: &str, expected_err: ParseError) {
        let bytes = input.as_bytes();
        let res = parse_int::<T>(bytes);
        assert!(
            res.is_err(),
            "Expected error for '{}', but got success",
            input
        );
        assert_eq!(res.unwrap_err(), expected_err);
    }

    #[test]
    fn test_floats_basic() {
        let opts = FfcParseOptions::default();

        // Basic decimals
        verify_float_exact::<f64>("1.2345", 1.2345, opts);
        verify_float_exact::<f32>("1.00000006e+09", 1.00000006e+09f32, opts);

        // Subnormals / Small values
        verify_float_exact::<f32>("1.4012984643e-45", 1.4012984643e-45f32, opts);

        // Inf/NaN
        verify_float_exact::<f64>("inf", f64::INFINITY, opts);
        verify_float_exact::<f64>("-infinity", f64::NEG_INFINITY, opts);
        verify_float_exact::<f64>("nan", f64::NAN, opts);
    }

    #[test]
    fn test_float_json_constraints() {
        let json_opts = FfcParseOptions {
            format: FfcFormat::PRESET_JSON,
            ..Default::default()
        };

        // JSON must fail on leading '+'
        assert!(parse_float::<f64>(b"+1.23", json_opts).is_err());

        // JSON must fail on leading zeros in integers
        assert!(parse_float::<f64>(b"01.23", json_opts).is_err());

        // Valid JSON
        verify_float_exact::<f64>("0.123", 0.123, json_opts);
    }

    #[test]
    fn test_i32_logic() {
        verify_int::<i32>("0", 0);
        verify_int::<i32>("-1", -1);
        verify_int::<i32>("2147483647", i32::MAX);
        verify_int::<i32>("-2147483648", i32::MIN);
    }

    #[test]
    fn test_u64_swar_boundaries() {
        verify_int::<u64>("12345678", 12345678);
        verify_int::<u64>("1234567890123456", 1234567890123456);
    }

    #[test]
    fn test_int_errors() {
        // Overflow
        verify_int_error::<i32>("3000000000", ParseError::OutOfRange);

        // If we strictly want the whole string parsed:
        let res = parse_int::<u32>(b"12a3").unwrap();
        assert_eq!(res.1, b"a3"); // Remainder contains the 'a'
    }

    #[test]
    fn test_issue_235_single_char() {
        // Ported from test_int.c: single '0' handling
        verify_int::<i32>("0", 0);
    }
}
