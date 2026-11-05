//! Kani harnesses for the template tokenizer and parser (SPEC-603 R7, ADR-624 D5).
//!
//! Each harness draws every 8-byte input, keeps the ones that are UTF-8 text, and calls the
//! shipped function of `crates/render/src/template.rs`; Kani then proves no panic is reachable.
//! They compile only under `cfg(kani)`, as Kani documents for proofs placed under `tests/`, so a
//! plain `cargo test` builds this file as an empty test crate. The verifier is not installed
//! here: the harnesses read VOID until the delivery that installs it, which adds their witnesses.

#[cfg(kani)]
mod harnesses {
    use deck_streak_render::template::{parse, tokenize};

    /// The tokenizer is total over every 8-byte text.
    #[kani::proof]
    #[kani::unwind(12)]
    fn tokenize_never_panics() {
        let bytes: [u8; 8] = kani::any();
        if let Ok(text) = core::str::from_utf8(&bytes) {
            let _ = tokenize(text);
        }
    }

    /// The parser is total over every 8-byte text: a tree or an issue, never a panic.
    #[kani::proof]
    #[kani::unwind(12)]
    fn parse_never_panics() {
        let bytes: [u8; 8] = kani::any();
        if let Ok(text) = core::str::from_utf8(&bytes) {
            let _ = parse(text);
        }
    }
}
