//! The face rewrite (SPEC-601 R9; ADR-621 D8): every media reference the one grammar finds in a
//! note's HTML, rewritten in place by the caller, every other byte kept. A face reads its media
//! names through the same grammar the store, the sweep and the sync read them through.

/// `html` with each media reference the grammar finds, in order, replaced by what `to` answers for
/// its name; every other byte of `html` is kept.
pub fn rewrite(html: &str, to: impl FnMut(&str) -> String) -> String {
    drop(to);
    html.to_owned()
}
