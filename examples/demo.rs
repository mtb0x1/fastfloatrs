use fastfloatrs::{FfFormat, FfcParseOptions, parse_float, parse_int};

fn main() {
    // float parsing (f64)
    let float_input = b"123.456e-2 leftover";
    match parse_float::<f64>(float_input, FfcParseOptions::default()) {
        Ok((val, remainder)) => {
            let remain_str = core::str::from_utf8(remainder).unwrap_or("");
            println!("Parsed f64: {} | Remainder: {:?}", val, remain_str);
        }
        Err(e) => println!("f64 Parse Error: {:?}", e),
    }

    // parsing (f32) with JSON constraints
    let json_opts = FfcParseOptions {
        format: FfFormat::PRESET_JSON,
        ..Default::default()
    };
    let f32_input = b"01.23"; // Invalid JSON (leading zero)
    match parse_float::<f32>(f32_input, json_opts) {
        Ok((val, _)) => println!("Parsed f32: {}", val),
        Err(e) => println!(
            "f32 JSON Parse Error: {:?} (Expected due to leading zero)",
            e
        ),
    }

    // SWAR optimized unsigned integer parsing (u64)
    let u64_input = b"123456789012345 trailing";
    match parse_int::<u64>(u64_input) {
        Ok((val, remainder)) => {
            let remain_str = core::str::from_utf8(remainder).unwrap_or("");
            println!("Parsed u64: {} | Remainder: {:?}", val, remain_str);
        }
        Err(e) => println!("u64 Parse Error: {:?}", e),
    }

    // Integer parsing (i32) with a negative sign
    let i32_input = b"-987654";
    match parse_int::<i32>(i32_input) {
        Ok((val, _)) => println!("Parsed i32: {}", val),
        Err(e) => println!("i32 Parse Error: {:?}", e),
    }
}
