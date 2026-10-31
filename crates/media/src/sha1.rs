//! SHA-1 (FIPS 180-4), for the one name the engine derives by it: a LaTeX image's (SPEC-513 R9;
//! ADR-531 D8).
//!
//! SHA-1 is broken for collisions, so it certifies nothing here: a media file is known by its
//! SHA-256 ([`crate::MediaHash`]), and SHA-1 only spells a name the engine already gives. It is
//! written in the crate, rather than taken from a package, so the core keeps `sha2` its one
//! dependency.

/// The length of a SHA-1 digest, in bytes.
pub const DIGEST_LEN: usize = 20;

/// The lowercase hex digits, in order of their value.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The length of a block, in bytes.
const BLOCK: usize = 64;

/// The length of the message's bit count at the end of the padding, in bytes.
const LENGTH_LEN: usize = 8;

/// The state before the first block.
const INITIAL: [u32; 5] = [
    0x6745_2301,
    0xefcd_ab89,
    0x98ba_dcfe,
    0x1032_5476,
    0xc3d2_e1f0,
];

/// The SHA-1 of `bytes`.
#[must_use]
pub fn sha1(bytes: &[u8]) -> [u8; DIGEST_LEN] {
    let mut state = INITIAL;
    let (blocks, rest) = bytes.as_chunks::<BLOCK>();
    for block in blocks {
        compress(&mut state, block);
    }
    // The padding: a one bit, zeros, then the message's length in bits, big-endian, ending a
    // block; a tail too long to hold the length spills into a second block.
    let bits = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut tail = [0; 2 * BLOCK];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < BLOCK - LENGTH_LEN {
        BLOCK
    } else {
        2 * BLOCK
    };
    tail[end - LENGTH_LEN..end].copy_from_slice(&bits.to_be_bytes());
    for block in tail[..end].as_chunks::<BLOCK>().0 {
        compress(&mut state, block);
    }
    let mut digest = [0; DIGEST_LEN];
    for (out, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *out = word.to_be_bytes();
    }
    digest
}

/// One block folded into the state (FIPS 180-4, 6.1.2).
fn compress(state: &mut [u32; 5], block: &[u8; BLOCK]) {
    let mut schedule = [0_u32; 80];
    for (word, bytes) in schedule.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for at in 16..schedule.len() {
        schedule[at] =
            (schedule[at - 3] ^ schedule[at - 8] ^ schedule[at - 14] ^ schedule[at - 16])
                .rotate_left(1);
    }
    // The standard's working variables a to e, in order.
    let mut work = *state;
    for (round, word) in schedule.iter().enumerate() {
        let [first, second, third, fourth, fifth] = work;
        let (mix, constant) = match round {
            0..=19 => ((second & third) | (!second & fourth), 0x5a82_7999),
            20..=39 => (second ^ third ^ fourth, 0x6ed9_eba1),
            40..=59 => (
                (second & third) | (second & fourth) | (third & fourth),
                0x8f1b_bcdc,
            ),
            _ => (second ^ third ^ fourth, 0xca62_c1d6),
        };
        let next = first
            .rotate_left(5)
            .wrapping_add(mix)
            .wrapping_add(fifth)
            .wrapping_add(constant)
            .wrapping_add(*word);
        work = [next, first, second.rotate_left(30), third, fourth];
    }
    for (value, add) in state.iter_mut().zip(work) {
        *value = value.wrapping_add(add);
    }
}

/// The SHA-1 of `bytes`, in lowercase hex.
#[must_use]
pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(2 * DIGEST_LEN);
    for byte in sha1(bytes) {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}
