//! The photo's JPEG size reader (SPEC-132 R3): the bound checks read the size a JPEG's header
//! names, so each test builds a JPEG whose segments walk a different path to its frame header and
//! reads the size back through the bounds, which name it exactly at 5000 by 5000.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used)]

use deck_streak_notifications::{Photo, PhotoError};

/// A start-of-frame segment of `marker`, naming `width` by `height`.
fn frame(marker: u8, width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xff, marker, 0x00, 0x11, 0x08];
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    bytes
}

/// A segment of `marker` holding `payload`.
fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len() + 2).expect("a payload that fits a segment");
    let mut bytes = vec![0xff, marker];
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

/// A payload that reads as a frame header of 1 by 1 if a walker mistakes its segment for one.
fn decoy(length: usize) -> Vec<u8> {
    let mut payload = vec![0x08, 0x00, 0x01, 0x00, 0x01];
    payload.resize(length, 0);
    payload
}

/// A JPEG of `parts` after its start marker.
fn jpeg(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8];
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes.resize(bytes.len().max(64), 0);
    bytes
}

/// Reads `parts` as the JPEG that names 5000 by 5000, and as the one that names 5001 by 5000.
fn reads_the_frame(walk: &[Vec<u8>], marker: u8) {
    let mut exact = walk.to_vec();
    exact.push(frame(marker, 5_000, 5_000));
    assert!(
        Photo::new(jpeg(&exact), "").is_ok(),
        "the exact size is read"
    );
    let mut wide = walk.to_vec();
    wide.push(frame(marker, 5_001, 5_000));
    assert_eq!(
        Photo::new(jpeg(&wide), "").err(),
        Some(PhotoError::Dimensions),
        "one pixel over is read"
    );
    let mut tall = walk.to_vec();
    tall.push(frame(marker, 5_000, 5_001));
    assert_eq!(
        Photo::new(jpeg(&tall), "").err(),
        Some(PhotoError::Dimensions),
        "one pixel over in height is read"
    );
}

#[test]
fn a_frame_header_right_after_the_start_is_read() {
    reads_the_frame(&[], 0xc0);
}

#[test]
fn a_progressive_frame_header_is_read() {
    reads_the_frame(&[], 0xc2);
    reads_the_frame(&[], 0xcf);
}

#[test]
fn segments_are_walked_by_their_lengths() {
    let walk = [
        segment(0xe0, &decoy(14)),
        segment(0xdb, &vec![7; 254]),
        segment(0xe1, &[]),
        segment(0xfe, &decoy(9)),
    ];
    reads_the_frame(&walk, 0xc0);
}

#[test]
fn a_segment_of_the_smallest_length_is_walked() {
    reads_the_frame(&[segment(0xe2, &[]), segment(0xe0, &decoy(5))], 0xc0);
}

#[test]
fn fill_bytes_before_a_marker_are_skipped() {
    reads_the_frame(&[vec![0xff, 0xff, 0xff]], 0xc0);
    reads_the_frame(&[segment(0xe0, &decoy(6)), vec![0xff]], 0xc0);
}

#[test]
fn the_markers_without_a_length_are_skipped() {
    for marker in [0x01, 0xd8, 0xd0, 0xd3, 0xd7] {
        reads_the_frame(&[vec![0xff, marker]], 0xc0);
    }
    reads_the_frame(
        &[
            segment(0xe1, &decoy(20)),
            vec![0xff, 0x01],
            vec![0xff, 0xd8],
            vec![0xff, 0xd0],
            segment(0xe0, &decoy(8)),
        ],
        0xc0,
    );
}

#[test]
fn the_table_and_reserved_markers_are_not_frames() {
    for marker in [0xc4, 0xc8, 0xcc] {
        reads_the_frame(&[segment(marker, &decoy(12))], 0xc0);
    }
    reads_the_frame(
        &[
            segment(0xc4, &decoy(12)),
            segment(0xc8, &decoy(10)),
            segment(0xcc, &decoy(7)),
        ],
        0xc1,
    );
}

#[test]
fn a_header_that_does_not_name_a_size_is_not_a_photo() {
    let refused = |bytes: Vec<u8>| Photo::new(bytes, "").err();

    assert_eq!(refused(jpeg(&[vec![0x00, 0x00]])), Some(PhotoError::Format));
    assert_eq!(
        refused(vec![0xff, 0xd8, 0xff, 0xe0]),
        Some(PhotoError::Format)
    );
    assert_eq!(
        refused(jpeg(&[vec![0xff, 0xe0, 0x00, 0x01], frame(0xc0, 64, 48)])),
        Some(PhotoError::Format)
    );
    assert_eq!(
        refused(jpeg(&[vec![0xff, 0xe0, 0x00, 0x00], frame(0xc0, 64, 48)])),
        Some(PhotoError::Format)
    );
    assert_eq!(
        refused(vec![0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x13]),
        Some(PhotoError::Format)
    );
    assert_eq!(refused(jpeg(&[vec![0xff, 0xd9]])), Some(PhotoError::Format));
    assert!(Photo::new(jpeg(&[frame(0xc0, 64, 48)]), "").is_ok());
    assert_eq!(
        refused(jpeg(&[frame(0xc0, 0, 48)])),
        Some(PhotoError::Dimensions)
    );
}

#[test]
fn a_refusal_and_a_photo_print_without_their_bytes() {
    assert_eq!(PhotoError::Format.to_string(), "photo_invalid");
    assert_eq!(PhotoError::Caption.to_string(), "photo_invalid");
    let photo = Photo::new(jpeg(&[frame(0xc0, 64, 48)]), "caption").expect("a photo");
    assert_eq!(format!("{photo:?}"), "Photo { len: 64, .. }");
}
