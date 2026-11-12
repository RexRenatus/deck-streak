//! The sealing key the API releases for a browser's seal id, the seal secret's lower bound and the
//! seal id's shape (SPEC-363 R5; A9, A10, A18).
//!
//! Every expected key is computed here by hand, from RFC 2104 over SHA-256, and never by calling
//! the code under test. The hand computation is checked first against RFC 4231's second test case,
//! so a wrong oracle cannot agree with a wrong key. Each seal secret is built from its parts at run
//! time.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fmt::Write as _;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use deck_streak_identity::sync_seal::{
    MIN_SECRET_BYTES, SEAL_ID_BYTES, SEAL_LABEL, SealError, SealId, SealSecret,
};
use sha2::{Digest, Sha256};

/// The label, spelled here rather than read from the code under test.
const LABEL: &[u8] = b"deck-streak sync seal v1";
/// SHA-256's block, in bytes.
const BLOCK: usize = 64;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// HMAC-SHA-256 of `message` under `key`, by RFC 2104's definition: a key longer than the block
/// is hashed first, the key is padded with zeros to the block, and the result is
/// `H((K ^ opad) || H((K ^ ipad) || message))`.
fn hmac_by_hand(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut block = [0_u8; BLOCK];
    if key.len() > BLOCK {
        let hashed = Sha256::digest(key);
        block[..hashed.len()].copy_from_slice(hashed.as_slice());
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let inner_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
    let outer_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(inner.as_slice());
    outer.finalize().as_slice().to_vec()
}

/// Lower-case hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// A seal secret of `length` bytes, each made from its index at run time.
fn secret_of(length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| u8::try_from((index * 7 + 3) % 251).expect("below 251"))
        .collect()
}

/// A seal id's 16 bytes, made from `seed` at run time.
fn id_bytes(seed: u8) -> [u8; SEAL_ID_BYTES] {
    let mut id = [0_u8; SEAL_ID_BYTES];
    for (index, byte) in id.iter_mut().enumerate() {
        *byte = seed
            .wrapping_mul(31)
            .wrapping_add(u8::try_from(index).expect("below 16") * 13);
    }
    id
}

#[test]
fn the_seal_key_is_the_labelled_hmac_of_the_seal_id() {
    // The oracle first: RFC 4231, test case 2.
    assert_eq!(
        hex(&hmac_by_hand(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        "the hand computation reproduces RFC 4231's second test case"
    );
    let cases = examined(
        "seal secret and seal id pairs",
        vec![(32, 1_u8), (33, 2), (64, 3), (65, 4), (100, 5)],
    );
    for (length, seed) in &cases {
        let secret = secret_of(*length);
        let id = id_bytes(*seed);
        let text = URL_SAFE_NO_PAD.encode(id);
        let mut message = LABEL.to_vec();
        message.extend_from_slice(&id);
        let expected = URL_SAFE_NO_PAD.encode(hmac_by_hand(&secret, &message));
        let parsed = SealId::parse(&text).expect("a 16-byte seal id parses");
        let key = SealSecret::new(&secret)
            .expect("a secret of at least 32 bytes is accepted")
            .key_for(&parsed);
        assert_eq!(
            key.encoded(),
            expected,
            "the sealing key under a {length}-byte secret is the labelled HMAC of the seal id"
        );
        assert_eq!(key.encoded().len(), 43, "a key is 43 base64url characters");
        let unlabelled = URL_SAFE_NO_PAD.encode(hmac_by_hand(&secret, &id));
        assert_ne!(
            key.encoded(),
            unlabelled,
            "the label is part of what the key is made over"
        );
        assert_eq!(
            format!("{key:?}"),
            "SealKey(..)",
            "a key's Debug is redacted"
        );
    }
    assert_eq!(SEAL_LABEL, LABEL, "the label is the one the SPEC fixes");
}

#[test]
fn a_seal_secret_under_32_bytes_refuses_start() {
    let refused = examined("seal secrets under 32 bytes", vec![0, 1, 16, 31]);
    for length in &refused {
        assert!(
            matches!(
                SealSecret::new(&secret_of(*length)),
                Err(SealError::TooShort)
            ),
            "a {length}-byte seal secret refuses start"
        );
    }
    let accepted = examined("seal secrets of 32 bytes or more", vec![32, 33, 64, 100]);
    for length in &accepted {
        let secret = SealSecret::new(&secret_of(*length)).expect("the secret is accepted");
        let shown = format!("{secret:?}");
        assert_eq!(
            shown, "SealSecret(..)",
            "a seal secret's Debug prints none of it"
        );
    }
    assert_eq!(MIN_SECRET_BYTES, 32);
    assert_eq!(
        SealError::TooShort.to_string(),
        "the seal secret is shorter than 32 bytes",
        "the refusal names the bound and no byte of the secret"
    );
}

#[test]
fn a_seal_id_that_is_not_sixteen_bytes_does_not_parse() {
    let good = URL_SAFE_NO_PAD.encode(id_bytes(9));
    assert_eq!(good.len(), 22);
    let parsed = SealId::parse(&good).expect("22 base64url characters of 16 bytes parse");
    assert_eq!(
        format!("{parsed:?}"),
        "SealId(..)",
        "a seal id's Debug is redacted"
    );
    assert_eq!(
        Some(parsed),
        SealId::parse(&good),
        "the same text parses to the same id"
    );

    let padded = base64::engine::general_purpose::URL_SAFE.encode(id_bytes(9));
    let standard = good.replace('-', "+").replace('_', "/");
    let mut trailing = good.clone();
    trailing.pop();
    trailing.push('B');
    let refused = examined(
        "seal id texts that are not 16 bytes in unpadded base64url",
        vec![
            ("15 bytes", URL_SAFE_NO_PAD.encode([7_u8; 15])),
            ("17 bytes", URL_SAFE_NO_PAD.encode([7_u8; 17])),
            ("32 bytes", URL_SAFE_NO_PAD.encode([7_u8; 32])),
            ("an empty text", String::new()),
            ("16 bytes with padding", padded),
            ("a character outside base64", format!("{}!", &good[..21])),
            ("16 bytes with bits past the last byte", trailing),
            ("the standard alphabet", standard.clone()),
        ],
    );
    assert_ne!(standard, good, "the chosen id spells a URL-safe character");
    for (what, text) in &refused {
        assert!(SealId::parse(text).is_none(), "{what} does not parse");
    }
}
