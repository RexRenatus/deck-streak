//! SPEC-601's seventh criterion: the web core verifies handed bytes before it admits them. A matching pair is
//! admitted and read back capped; a pair whose bytes differ by one byte is refused and listed; an
//! empty set admits nothing.

use deck_streak_media::MediaHash;
use deck_streak_web_engine::handed::Handed;

const BYTES: &[u8] = b"a handed media file";

#[test]
fn a_matching_pair_is_admitted_and_read_back_capped() {
    let hash = MediaHash::of(BYTES);
    let handed = Handed::admit(vec![(hash, BYTES.to_vec())]);
    assert_eq!(handed.read(&hash, 8), Some(BYTES[..8].to_vec()));
    assert_eq!(handed.read(&hash, 1_000), Some(BYTES.to_vec()));
    assert_eq!(handed.refused(), []);
}

#[test]
fn a_pair_whose_bytes_differ_by_one_byte_is_refused_and_listed() {
    let hash = MediaHash::of(BYTES);
    let mut planted = BYTES.to_vec();
    planted[0] ^= 1;
    let good = MediaHash::of(b"another handed file");
    let handed = Handed::admit(vec![
        (hash, planted),
        (good, b"another handed file".to_vec()),
    ]);
    assert_eq!(handed.refused(), &[hash]);
    assert_eq!(handed.read(&hash, 1_000), None);
    assert_eq!(
        handed.read(&good, 1_000),
        Some(b"another handed file".to_vec())
    );
}

#[test]
fn an_empty_set_admits_nothing() {
    let hash = MediaHash::of(BYTES);
    let matching = Handed::admit(vec![(hash, BYTES.to_vec())]);
    assert_eq!(matching.read(&hash, 1_000), Some(BYTES.to_vec()));
    let empty = Handed::admit(Vec::new());
    assert_eq!(empty.refused(), []);
    assert_eq!(empty.read(&hash, 1_000), None);
}

/// The crate root, as the compiler reads it.
const CRATE_ROOT: &str = include_str!("../src/lib.rs");

/// A source-text guard: the crate root declares `handed` among the native modules, as written.
/// The row of SPEC-601 A12 on that line mutates it, a change no behaviour test can see.
#[test]
fn the_crate_root_declares_the_handed_module_as_written() {
    assert!(
        CRATE_ROOT.contains("pub mod files;\npub mod handed;\npub mod study;\n"),
        "the crate root declares handed between files and study, with no attribute"
    );
}
