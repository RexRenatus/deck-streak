//! A media file is known by the SHA-256 of its bytes (SPEC-513 A1): `MediaHash::of` gives the
//! NIST examples' digests, and a hash reads back from its hex form and its blob path while every
//! other text is refused.

use deck_streak_media::MediaHash;
use deck_streak_media::hash::{BLOB_PATH_LEN, HEX_LEN};

/// The SHA-256 examples of NIST's FIPS 180-2 (appendix B) and its example values: each message
/// and its digest, written out here.
const NIST: [(&str, &str); 4] = [
    (
        "abc",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ),
    (
        "",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    ),
    (
        "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    ),
];

/// One million repetitions of `a`, the long message of FIPS 180-2 appendix B.3.
const MILLION_A: &str = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";

#[test]
fn media_hash_of_matches_the_nist_sha256_vectors() {
    for (message, digest) in NIST {
        assert_eq!(
            MediaHash::of(message.as_bytes()).to_hex(),
            digest,
            "the SHA-256 of {message:?}"
        );
    }
    assert_eq!(
        MediaHash::of(&vec![b'a'; 1_000_000]).to_hex(),
        MILLION_A,
        "the SHA-256 of a million a"
    );
}

#[test]
fn a_hash_reads_back_from_its_hex_form_and_its_blob_path() {
    let hash = MediaHash::of(b"abc");
    let hex = hash.to_hex();
    assert_eq!(hex.len(), HEX_LEN);
    assert_eq!(hash.to_string(), hex, "a hash displays as its hex form");
    assert_eq!(MediaHash::from_hex(&hex), Some(hash));
    let path = hash.blob_path();
    assert_eq!(path.len(), BLOB_PATH_LEN);
    assert_eq!(
        &path[..],
        b"ba/ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "the blob path is the first two digits, a separator and the whole hex form"
    );
    assert_eq!(MediaHash::from_blob_path(&path), Some(hash));
    for byte in [0_u8, 0x0f, 0xf0, 0xff] {
        let hash = MediaHash::from_bytes([byte; 32]);
        assert_eq!(MediaHash::from_hex(&hash.to_hex()), Some(hash));
        assert_eq!(MediaHash::from_blob_path(&hash.blob_path()), Some(hash));
    }
}

#[test]
fn a_malformed_hex_form_or_blob_path_is_refused() {
    let hex = MediaHash::of(b"abc").to_hex();
    let refused_hex = [
        hex.to_uppercase(),
        hex[..63].to_owned(),
        format!("{hex}0"),
        format!("g{}", &hex[1..]),
        String::new(),
    ];
    for text in &refused_hex {
        assert_eq!(MediaHash::from_hex(text), None, "{text:?} is refused");
    }
    let path = MediaHash::of(b"abc").blob_path();
    let mut swapped = path;
    swapped.swap(0, 1);
    let mut no_separator = path;
    no_separator[2] = b'_';
    let mut other_fan = path;
    other_fan[0] = b'0';
    for refused in [&swapped[..], &no_separator[..], &other_fan[..], &path[1..]] {
        assert_eq!(
            MediaHash::from_blob_path(refused),
            None,
            "{} is refused",
            String::from_utf8_lossy(refused)
        );
    }
}
