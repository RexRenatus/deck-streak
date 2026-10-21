//! SHA-256 (FIPS 180-4), for the staged run record's `sha256` and `sha256_before` and for the
//! reading note's body hash (SPEC-042 R4, R10).
//!
//! It is written here because no ADR admits a hashing crate, as the kernel matches its token
//! pattern by hand (ADR-020); the tests hold it to the standard's own examples. It proves that a
//! file still has the bytes a writer last wrote, which is an integrity check, not a secret.

/// The first 32 bits of the fractional parts of the cube roots of the first 64 primes.
const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The first 32 bits of the fractional parts of the square roots of the first 8 primes.
const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// The SHA-256 digest of `bytes`.
#[must_use]
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut state = INITIAL_STATE;
    let mut blocks = bytes.chunks_exact(64);
    for block in &mut blocks {
        compress(&mut state, block);
    }
    // The padding: a one bit, zeros to 56 bytes past a block boundary, then the length in bits.
    let rest = blocks.remainder();
    let mut tail = [0_u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let tail_len = if rest.len() < 56 { 64 } else { 128 };
    let bits = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    tail[tail_len - 8..tail_len].copy_from_slice(&bits.to_be_bytes());
    for block in tail[..tail_len].chunks_exact(64) {
        compress(&mut state, block);
    }
    let mut out = [0_u8; 32];
    for (word, chunk) in state.iter().zip(out.chunks_exact_mut(4)) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// `bytes` as lowercase hexadecimal, two digits a byte.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The digest written as 64 lowercase hexadecimal digits, or `None` for text of another shape.
#[must_use]
pub fn parse_hex(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (byte, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        *byte = (high << 4) | low;
    }
    Some(out)
}

/// The value of one lowercase hexadecimal digit.
fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

/// One round of the compression function over a 64-byte block.
#[allow(
    clippy::many_single_char_names,
    reason = "the working variables are FIPS 180-4's a to h"
)]
fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut schedule = [0_u32; 64];
    for (word, bytes) in schedule.iter_mut().zip(block.chunks_exact(4)) {
        *word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    }
    for i in 16..64 {
        let s0 = schedule[i - 15].rotate_right(7)
            ^ schedule[i - 15].rotate_right(18)
            ^ (schedule[i - 15] >> 3);
        let s1 = schedule[i - 2].rotate_right(17)
            ^ schedule[i - 2].rotate_right(19)
            ^ (schedule[i - 2] >> 10);
        schedule[i] = schedule[i - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (constant, word) in ROUND_CONSTANTS.iter().zip(schedule.iter()) {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(choose)
            .wrapping_add(*constant)
            .wrapping_add(*word);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::{digest, hex, parse_hex};

    /// The standard's examples (FIPS 180-4, the empty message, `abc` and the 448-bit message),
    /// the padding boundaries of 55, 56 and 64 bytes, and a message of many blocks, each digest as
    /// Python's `hashlib` gives it.
    #[test]
    fn the_digest_equals_the_standards_examples() {
        let million = vec![b'a'; 1_000_000];
        let cases: [(&[u8], &str); 6] = [
            (
                b"",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                &[0_u8; 55],
                "02779466cdec163811d078815c633f21901413081449002f24aa3e80f0b88ef7",
            ),
            (
                &[0_u8; 56],
                "d4817aa5497628e7c77e6b606107042bbba3130888c5f47a375e6179be789fbb",
            ),
            (
                &million,
                "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            ),
        ];
        for (message, expected) in cases {
            assert_eq!(
                hex(&digest(message)),
                expected,
                "a message of {} byte(s)",
                message.len()
            );
        }
        assert_eq!(
            hex(&digest(&[0_u8; 64])),
            "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"
        );
    }

    #[test]
    fn a_hex_digest_reads_back_and_another_shape_is_refused() {
        let value = digest(b"abc");
        assert_eq!(parse_hex(&hex(&value)), Some(value));
        assert_eq!(parse_hex("ABC"), None);
        assert_eq!(parse_hex(&hex(&value).to_uppercase()), None);
    }
}
