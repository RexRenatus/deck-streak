//! The synthetic notes `seed` writes into an empty collection, for the browser tests and the
//! measurements (SPEC-338 R12): the same on every target, so the native tests judge exactly the
//! fields the `wasm32` module stores. A stub: the tests come first.

/// The two fields of synthetic note `number`: its front and its back.
#[must_use]
pub fn fields(number: u32) -> [String; 2] {
    [
        format!("synthetic front {number}"),
        format!("synthetic back {number}"),
    ]
}
