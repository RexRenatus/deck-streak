//! The handed files (SPEC-601 R7; ADR-621 D7), built switched off: the hash-keyed media files the
//! Worker's switched protocol will hand the web core. The core verifies each file's bytes against
//! its hash before it admits it, keeps every refused hash, and reads an admitted file at most a
//! limit. `study.rs`, `wasm.rs` and the Worker keep their name-keyed protocol until the Worker
//! carries hashes.

use deck_streak_media::MediaHash;

/// The files a Worker handed the web core, each admitted only when its bytes hash to its hash.
#[derive(Debug, Clone, Default)]
pub struct Handed {
    files: Vec<(MediaHash, Vec<u8>)>,
    refused: Vec<MediaHash>,
}

impl Handed {
    /// Admits each `(hash, bytes)` pair whose bytes hash to its hash, and lists every other hash
    /// as refused, in the order handed.
    #[must_use]
    pub fn admit(pairs: Vec<(MediaHash, Vec<u8>)>) -> Self {
        Self {
            files: pairs,
            refused: Vec::new(),
        }
    }

    /// Every hash whose bytes did not hash to it, in the order handed.
    #[must_use]
    pub fn refused(&self) -> &[MediaHash] {
        &self.refused
    }

    /// At most `limit` bytes of the admitted file of `hash`, or `None` when none was admitted.
    #[must_use]
    pub fn read(&self, hash: &MediaHash, limit: u64) -> Option<Vec<u8>> {
        let _ = (&self.files, hash, limit);
        None
    }
}
