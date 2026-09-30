//! The C-class table's pins (SPEC-110 R2, ADR-110): its boundaries by literal code point, its
//! order, and the deferral reason's golden through it.

use super::{RANGES, is_other};

/// Each range's first and last code point are in, and the point just outside each is out. The
/// values are literal: they come from Python's `unicodedata.category`, not from the table.
#[test]
fn the_table_holds_each_boundary_and_nothing_beside_it() {
    let inside: [u32; 17] = [
        0x0000,
        0x001F,
        0x007F,
        0x009F,
        0x00AD,
        0x0378,
        0x0379,
        0x200B,
        0x200F,
        0xD7FC,
        0xD7FF,
        0xE000,
        0xF8FF,
        0xFDD0,
        0xFDEF,
        0xE01F0,
        0x0010_FFFF,
    ];
    let outside: [u32; 14] = [
        0x0020, 0x007E, 0x00A0, 0x00AC, 0x00AE, 0x0377, 0x037A, 0x200A, 0x2010, 0xD7FB, 0xF900,
        0xFDCF, 0xFDF0, 0xE01EF,
    ];
    for point in inside {
        let c = char::from_u32(point).expect("a scalar value");
        assert!(
            is_other(c),
            "U+{point:04X} is a control, format, unassigned or private-use point"
        );
    }
    for point in outside {
        let c = char::from_u32(point).expect("a scalar value");
        assert!(!is_other(c), "U+{point:04X} is not one");
    }
}

/// The table is sorted and no two ranges touch or overlap, which the binary search relies on.
#[test]
fn the_table_is_sorted_and_its_ranges_are_disjoint() {
    assert!(!RANGES.is_empty());
    for range in RANGES {
        assert!(range.0 <= range.1, "a range runs upward");
    }
    for pair in RANGES.windows(2) {
        assert!(pair[0].1 < pair[1].0, "ranges are sorted and disjoint");
    }
}
