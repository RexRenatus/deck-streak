//! The bindings generator in a build without the `bindgen` feature (SPEC-336 R10, A8).
//!
//! RED-FIRST. Every build of the crate builds the generator's binary (ADR-345 D5), and a build
//! without the feature holds none of the generator's code: the binary must say so and fail, never
//! exit 0 having written no bindings. With the feature, the binary IS the generator, whose
//! library-mode run the `xcframework` workflow measures (SPEC-336 section 7), so this file holds
//! nothing in a build with the feature.

#![cfg(not(feature = "bindgen"))]

use std::process::Command;

/// The arguments the `xcframework` workflow passes in library mode. The paths name no file, and
/// nothing is written to them: the refusal comes before any argument is read.
const LIBRARY_MODE: [&str; 9] = [
    "target/aarch64-apple-ios/release/libdeck_streak_ffi.a",
    "bindings",
    "--swift-sources",
    "--headers",
    "--modulemap",
    "--module-name",
    "deck_streak_ffiFFI",
    "--modulemap-filename",
    "module.modulemap",
];

/// A8 (R10): built without `bindgen`, the generator answers the workflow's own library-mode call
/// with exit status 2 and one line on stderr that names the feature and the command that builds
/// it, and writes nothing to stdout.
#[test]
fn a8_the_generator_refuses_a_build_without_its_feature() {
    let run = Command::new(env!("CARGO_BIN_EXE_uniffi-bindgen-swift"))
        .args(LIBRARY_MODE)
        .output()
        .map(|output| {
            (
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
            )
        })
        .map_err(|error| error.to_string());
    assert_eq!(
        run,
        Ok((
            Some(2),
            String::from(
                "uniffi-bindgen-swift: this build holds no Swift bindings generator, because it \
                 was built without the `bindgen` feature; run it as `cargo run -p deck-streak-ffi \
                 --features bindgen --bin uniffi-bindgen-swift -- <arguments>`\n"
            ),
            String::new(),
        ))
    );
}
