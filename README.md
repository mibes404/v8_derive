# Rusty V8 Derive Macros

Derive macros for the [Rusty V8](https://github.com/denoland/rusty_v8) Bindings.

These annotations make it easier to convert JavaScript Values to Rust structs and vice versa.

## Usage

See the [v8_derive_example](https://github.com/mibes404/v8_derive/tree/main/v8_derive_sample) for a complete example.

`Cargo.toml`
```toml
[dependencies]
v8_derive = "*"
```

`Main.rs`
```rust
use v8_derive::{
    macros::{FromValue, IntoValue},
    IntoValue, TryFromValue,
};

#[derive(FromValue, IntoValue)]
struct SimpleObject {
    yes_no: bool,
    name: String,
    age: i32,
    opt: Option<i32>,
}

fn main() {
    // Set the v8 Javascript Value from a Rust struct
    let js_obj = rust_obj.into_value(scope);

    // Get the V8 Javascript Value as a Rust struct
    let rust_obj = SimpleObject::try_from_value(&js_obj, scope).unwrap();
}
```

## Supported Types

- `bool`
- `String`
- `i8`
- `i32`
- `i64`
- `f64`
- `u32`
- `f32`
- `Option<T>` where `T` is one of the above types
- `struct` where all fields are one of the above types
- `Vec<T>` where `T` is one of the above types
- `HashMap<String, T>` where `T` is one of the above types

## DISCLAIMER

Please note: all content in this repository is released for use "AS IS" without any warranties of any kind, including, but not limited to their installation, use, or performance. We disclaim any and all warranties, either express or implied, including but not limited to any warranty of noninfringement, merchantability, and/ or fitness for a particular purpose. We do not warrant that the technology will meet your requirements, that the operation thereof will be uninterrupted or error-free, or that any errors will be corrected.

Any use of these scripts and tools is at your own risk. There is no guarantee that they have been through thorough testing in a comparable environment and we are not responsible for any damage or data loss incurred with their use.

You are responsible for reviewing and testing any generated code you run thoroughly before use in any non-testing environment.