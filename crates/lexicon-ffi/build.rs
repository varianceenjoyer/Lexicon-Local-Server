//! Build script for lexicon-ffi.
//!
//! Generates UniFFI bindings from the UDL file.

fn main() {
    uniffi::generate_scaffolding("src/lexicon.udl").expect("Failed to generate UniFFI scaffolding");
}
