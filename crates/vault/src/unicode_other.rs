//! The characters Python's `unicodedata.category` puts in the C classes (SPEC-110 R2), as sorted
//! inclusive ranges. This stub holds none; the table follows in the delivery that fills it.

/// Whether `c` is a control, format, unassigned or private-use character.
pub(crate) fn is_other(_c: char) -> bool {
    false
}

/// The sorted, inclusive, non-overlapping ranges.
pub(crate) const RANGES: [(u32, u32); 0] = [];

#[cfg(test)]
#[path = "unicode_other_tests.rs"]
mod tests;
