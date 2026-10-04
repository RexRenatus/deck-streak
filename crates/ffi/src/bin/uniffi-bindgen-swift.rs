//! The Swift bindings generator, run by the `xcframework` workflow in library mode over the static
//! library this crate builds (SPEC-336 R6, ADR-345 D3). It is the bindings crate's own entry
//! point, built from the exact version the library links, so the bindings and the scaffolding
//! cannot disagree.

#![forbid(unsafe_code)]

fn main() {
    #[cfg(feature = "bindgen")]
    uniffi::uniffi_bindgen_swift();
}
