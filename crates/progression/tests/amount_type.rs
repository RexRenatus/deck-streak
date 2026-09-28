//! An XP amount cannot be built from a signed integer, so a debit does not compile (SPEC-040 A6,
//! R6; CHARTER 5: XP is never confiscable).
//!
//! `tests/ui/signed_amount.rs` builds an amount from an `i64`, and must fail to compile with the
//! refusal `tests/ui/signed_amount.stderr` records. The record moves with the pinned toolchain, and
//! is regenerated in the change that bumps it (`TRYBUILD=overwrite`).

#[test]
fn a_signed_amount_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/signed_amount.rs");
}
