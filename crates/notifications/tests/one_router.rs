//! Only the router module can make a delivery call (SPEC-041 A2; R1): each bot transport call takes
//! the router's `Pass`, which no other module can make, so a call anywhere else does not compile.
//!
//! One compile-fail case for each way a pass could be had outside the router, each with the
//! refusal its `.stderr` records: its private field (`tests/ui/push_outside_the_router.rs`), its
//! `Default` (`tests/ui/pass_by_default.rs`), and a clone of the pass a transport borrows
//! (`tests/ui/pass_kept_by_a_clone.rs`). The records move with the pinned toolchain, and are
//! regenerated in the change that bumps it (`TRYBUILD=overwrite`).

#[test]
fn a_delivery_call_outside_the_router_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/push_outside_the_router.rs");
    cases.compile_fail("tests/ui/pass_by_default.rs");
    cases.compile_fail("tests/ui/pass_kept_by_a_clone.rs");
}
