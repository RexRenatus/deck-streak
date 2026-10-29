//! The scheduling companion on the web (SPEC-511 R9, R10; ADR-529 D3, D4).
//!
//! STUB: the shapes only. The decision answers one constant, no frame is built, and every
//! checkpoint is kept.

/// The first byte of an admitted answer.
pub const ADMITTED: u8 = 0;
/// The first byte of a refused answer.
pub const REFUSED: u8 = 1;
/// The status byte of a refusal that carries no status.
pub const NO_STATUS: u8 = 0xff;

/// What the web load check says of a stored checkpoint: keep it, or one named discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Every condition holds: the checkpoint is kept.
    Keep,
    /// The hello does not negotiate.
    NotNegotiated,
    /// The stored frame is not an `Ok` checkpoint.
    Unreadable,
    /// The checkpoint belongs to another model.
    ModelChanged,
    /// The checkpoint was folded under another adapter mode.
    AdapterChanged,
    /// The companion did not take the checkpoint back.
    LoadError,
    /// The checkpoint's head names no review in the store.
    HeadNamesNoReview,
    /// Reviews were folded and no fold index is held.
    IndexMissing,
    /// The fold index's count or head is not the checkpoint's.
    IndexMismatch,
}

impl Verdict {
    /// The verdict's code: 0 to keep, else the discard's place in the declared order.
    #[must_use]
    pub fn code(self) -> u8 {
        0
    }
}

/// The decision over one press's frames.
#[must_use]
pub fn decide(_hello: &[u8], _epoch: u32, _preview: &[u8], _reply: &[u8]) -> Vec<u8> {
    vec![REFUSED, 2, NO_STATUS]
}

/// The `Restore` frame for a stored checkpoint, or empty when it builds none.
#[must_use]
pub fn restore(_epoch: u32, _checkpoint: &[u8]) -> Vec<u8> {
    Vec::new()
}

/// The `Purge` frame at `epoch`.
#[must_use]
pub fn purge(_epoch: u32) -> Vec<u8> {
    Vec::new()
}

/// The web load check.
#[must_use]
pub fn load_check(
    _hello: &[u8],
    _checkpoint: &[u8],
    _ack: &[u8],
    _head_named: bool,
    _index_count: Option<u64>,
    _index_head: Option<i64>,
) -> Verdict {
    Verdict::Keep
}
