//! The Kani harnesses as they are written (SPEC-602 R11; ADR-622 D7). `src/kani_proofs.rs`
//! compiles only under the Kani verifier (`#[cfg(kani)]`), so no native build holds a harness and
//! no native test can call one. The verifier is not installed, so K-wire-walk and K-field-split
//! are not run yet. This test reads the file as text instead and holds each harness to the text it
//! is written as: a harness changed in any way fails here until this copy changes with it, so a
//! change to a harness is a change a reviewer reads before the verifier runs it.

#![allow(clippy::print_stdout)]

/// The harness file, as the Kani verifier reads it.
const HARNESSES: &str = include_str!("../src/kani_proofs.rs");

/// Each harness of the file, from its `fn` line to its closing brace, as written.
const WRITTEN: [(&str, &str); 2] = [
    (
        "k_wire_walk",
        r"fn k_wire_walk() {
    let data: [u8; 16] = kani::any();
    if let Ok(found) = wire::walk(&data) {
        assert!(found.len() <= data.len());
        for field in &found {
            assert!(matches!(field.value.wire_type(), 0 | 1 | 2 | 5));
        }
    }
}",
    ),
    (
        "k_field_split",
        r"fn k_field_split() {
    let bytes: [u8; 8] = kani::any();
    let length: usize = kani::any();
    kani::assume(length <= bytes.len());
    if let Ok(text) = std::str::from_utf8(&bytes[..length]) {
        let split = fields::split(text);
        assert_eq!(fields::join(&split), text);
        let separators = text
            .chars()
            .filter(|c| *c == fields::FIELD_SEPARATOR)
            .count();
        assert_eq!(split.len(), separators + 1);
    }
}",
    ),
];

/// The function `name` of `text`: from its `fn` line to the first line after it that is `}`.
fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let start = text.find(&format!("\nfn {name}("))? + 1;
    let length = text[start..].find("\n}\n")? + 2;
    Some(&text[start..start + length])
}

#[test]
fn each_kani_harness_reads_as_it_is_written() {
    for (name, written) in WRITTEN {
        assert_eq!(function(HARNESSES, name), Some(written), "{name}");
    }
    let planted = HARNESSES.replacen(
        "kani::assume(length <= bytes.len());",
        "kani::assume(length < bytes.len());",
        1,
    );
    assert_ne!(
        function(&planted, "k_field_split"),
        function(HARNESSES, "k_field_split"),
        "a planted change to a harness reads as a change"
    );
    let count = HARNESSES.matches("#[kani::proof]").count();
    assert_eq!(count, WRITTEN.len(), "every harness of the file is held");
    println!("examined {} of {count} harness function(s)", WRITTEN.len());
}
