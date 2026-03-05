# fastfloatrs

A Rust implementation (kinda) of the fast float parsing library, providing high-performance parsing of floating-point and integer numbers from byte slices.

## Overview

`fastfloatrs` is an approximation of a translation of the [fast_float](https://github.com/lemire/fast_float) library into Rust. It supports parsing:

- **Floating-point numbers**: `f32`, `f64` with multiple format options (scientific, fixed, hexadecimal, JSON, Fortran)
- **Integers**: Signed and unsigned integers using SWAR-optimized parsing

## Features

- `#![no_std]` compatible
- Format-aware parsing with customizable options
- Support for special values (NaN, Infinity)
- Multiple parsing presets: General, JSON, Fortran
- SWAR-optimized integer parsing

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
fastfloatrs = "0.1.0"
```

### Example

```rust
use fastfloatrs::{FfcFormat, FfcParseOptions, parse_float};

// Parse a float with default settings
let input = b"123.456e-2 leftover";
match parse_float::<f64>(input, FfcParseOptions::default()) {
    Ok((value, remainder)) => {
        println!("Parsed: {}", value);
        println!("Remainder: {:?}", std::str::from_utf8(remainder).unwrap());
    }
    Err(e) => println!("Parse error: {:?}", e),
}

// Parse with JSON format constraints
let json_opts = FfcParseOptions {
    format: FfcFormat::PRESET_JSON,
    ..Default::default()
};
```

## TODOs

### 1. **Testing** 
   - align test coverage with upstream projects:
     - [kolemannix/ffc.h](https://github.com/kolemannix/ffc.h)
     - [lemire/fast_float](https://github.com/fastfloat/fast_float)
   - add edge case coverage (overflow, underflow, special values)
   - validate against reference implementations

### 2. **FFI Wrapper**
   - create C-compatible FFI bindings

### 3. **Benchmarking**
   - integrate [lemire/simple_fastfloat_benchmark](https://github.com/lemire/simple_fastfloat_benchmark)
   - compare performance against:
     - Standard Rust parsing (`FromStr`)
     - Other fast-float implementations
   - establish performance baselines

## License

Matching upstream fast_float, ffc.h projects.

## References

- [fast_float](https://github.com/lemire/fast_float) - Original C++ implementation
- [ffc.h](https://github.com/kolemannix/ffc.h) - Alternative C reference
- [simple_fastfloat_benchmark](https://github.com/lemire/simple_fastfloat_benchmark) - Benchmarking suite
