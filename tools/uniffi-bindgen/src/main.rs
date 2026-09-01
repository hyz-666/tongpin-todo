//! Thin binary wrapper around the workspace-pinned UniFFI bindings generator.
//! See tools/uniffi-bindgen/Cargo.toml for why this exists.

fn main() {
    uniffi::uniffi_bindgen_main()
}
