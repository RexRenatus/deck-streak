//! Only the router module can make a delivery call (SPEC-041 A2; R1): each bot transport call takes
//! the router's `Pass`, whose one field is private to that module, so a call anywhere else does not
//! compile.
//!
//! `tests/ui/push_outside_the_router.rs` calls `push_message` from outside the router, and must fail
//! to compile with the refusal `tests/ui/push_outside_the_router.stderr` records. The record moves
//! with the pinned toolchain, and is regenerated in the change that bumps it (`TRYBUILD=overwrite`).

#[test]
fn a_delivery_call_outside_the_router_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/push_outside_the_router.rs");
}
