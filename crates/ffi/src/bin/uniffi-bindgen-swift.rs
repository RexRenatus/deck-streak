//! The Swift bindings generator, run by the `xcframework` workflow in library mode over the static
//! library this crate builds (SPEC-336 R6, ADR-345 D3). It is the bindings crate's own entry
//! point, built from the exact version the library links, so the bindings and the scaffolding
//! cannot disagree.
//!
//! Every build of the crate builds this binary, so a default-feature test runs it, and only the
//! `bindgen` feature puts the generator's code in it (SPEC-336 R10, ADR-345 D5). Built without the
//! feature it refuses every run, rather than exit 0 having written no bindings. Each arm is a
//! statement of `main`, never a function of its own: a function behind the feature would be a
//! mutant that the default-feature mutation run lists and never builds.

#![forbid(unsafe_code)]

fn main() {
    #[cfg(feature = "bindgen")]
    uniffi::uniffi_bindgen_swift();
    #[cfg(not(feature = "bindgen"))]
    {
        eprintln!(
            "uniffi-bindgen-swift: this build holds no Swift bindings generator, because it was \
             built without the `bindgen` feature; run it as `cargo run -p deck-streak-ffi \
             --features bindgen --bin uniffi-bindgen-swift -- <arguments>`"
        );
        std::process::exit(2);
    }
}
