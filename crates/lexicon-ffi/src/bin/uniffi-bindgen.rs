//! UniFFI binding generator binary.
//!
//! This binary generates Swift and Kotlin bindings from the UDL file.
//!
//! Usage:
//!   cargo run --bin uniffi-bindgen -- generate src/lexicon.udl --language swift --out-dir ./bindings/swift
//!   cargo run --bin uniffi-bindgen -- generate src/lexicon.udl --language kotlin --out-dir ./bindings/kotlin

fn main() {
    uniffi::uniffi_bindgen_main()
}
