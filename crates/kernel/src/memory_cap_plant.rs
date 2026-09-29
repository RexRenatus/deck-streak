//! A plant for this delivery's proof, removed by the next commit: one of its mutants never ends.

/// The bytes each chunk holds.
const PLANT_CHUNK: usize = 1_048_576;

/// One chunk for each halving of `n` until it reaches zero, every byte written so each chunk is
/// resident. Mutating `>` into `>=` never reaches the end, so that mutant only allocates.
#[must_use]
pub fn plant_chunks(mut n: u64) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while n > 0 {
        out.push(vec![1u8; PLANT_CHUNK]);
        n /= 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{PLANT_CHUNK, plant_chunks};

    #[test]
    fn eight_halves_to_four_full_chunks() {
        let chunks = plant_chunks(8);
        assert_eq!(chunks.len(), 4);
        for chunk in &chunks {
            assert_eq!(chunk.len(), PLANT_CHUNK);
            assert!(chunk.iter().all(|&byte| byte == 1));
        }
    }
}
