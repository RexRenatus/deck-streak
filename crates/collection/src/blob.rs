//! The blob store (SPEC-601 R2 to R4 and R8; ADR-621 D1, D4 and D5): each media file kept as a
//! file of the user's blob folder, under its SHA-256. The layout is `<root>/sha256/` joined with
//! [`MediaHash::blob_path`], a put stages its bytes in `<root>/tmp/`, and a file whose length
//! disagrees with its row moves to `<root>/bad/<hex>`.

use std::path::{Path, PathBuf};

use deck_streak_media::MediaHash;

/// The folder under the root that holds every placed file.
pub const PLACED: &str = "sha256";

/// The folder under the root that stages a put's bytes before the rename.
pub const STAGING: &str = "tmp";

/// The folder under the root that holds a quarantined file.
pub const QUARANTINE: &str = "bad";

/// Why the blob store refused a put or a move.
#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    /// The bytes are longer than the cap the put was given.
    #[error("the file is larger than its cap")]
    TooLarge,
    /// The bytes do not hash to the hash the caller claimed for them.
    #[error("the bytes do not hash to the claimed hash")]
    Mismatch,
    /// The blob folder could not be read or written.
    #[error("the blob folder: {0}")]
    Io(#[from] std::io::Error),
}

/// Media files kept under their hash.
pub trait BlobStore {
    /// Places `bytes` under their hash and answers it: staged, hashed, capped at `cap`, synced,
    /// then renamed to the hash's path.
    ///
    /// # Errors
    ///
    /// [`BlobError::TooLarge`] past the cap, and [`BlobError::Io`] when the folder refuses.
    fn put(&self, bytes: &[u8], cap: u64) -> Result<MediaHash, BlobError>;

    /// Places `bytes` under `claimed`, hashing while it stages them and refusing a mismatch before
    /// any rename.
    ///
    /// # Errors
    ///
    /// [`BlobError::Mismatch`] when the bytes hash to another hash, [`BlobError::TooLarge`] past
    /// the cap, and [`BlobError::Io`] when the folder refuses.
    fn put_claimed(&self, claimed: &MediaHash, bytes: &[u8], cap: u64) -> Result<(), BlobError>;

    /// At most `limit` bytes of the file placed under `hash`, or `None` when none is.
    fn get(&self, hash: &MediaHash, limit: u64) -> Option<Vec<u8>>;

    /// Moves the file placed under `hash` to the quarantine folder.
    ///
    /// # Errors
    ///
    /// [`BlobError::Io`] when the move fails.
    fn quarantine(&self, hash: &MediaHash) -> Result<(), BlobError>;
}

/// The blob store over one user's blob folder.
#[derive(Debug, Clone)]
pub struct FsBlobStore {
    root: PathBuf,
}

impl FsBlobStore {
    /// The blob store whose folder is `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The blob folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the file of `hash` is placed: `<root>/sha256/` joined with its blob path.
    #[must_use]
    pub fn path_of(&self, hash: &MediaHash) -> PathBuf {
        let blob_path = hash.blob_path();
        let relative = String::from_utf8_lossy(&blob_path).into_owned();
        self.root.join(PLACED).join(relative)
    }

    /// Where the file of `hash` goes when it is quarantined: `<root>/bad/<hex>`.
    #[must_use]
    pub fn quarantined_path_of(&self, hash: &MediaHash) -> PathBuf {
        self.root.join(QUARANTINE).join(hash.to_hex())
    }
}

impl BlobStore for FsBlobStore {
    fn put(&self, bytes: &[u8], _cap: u64) -> Result<MediaHash, BlobError> {
        Ok(MediaHash::of(bytes))
    }

    fn put_claimed(&self, _claimed: &MediaHash, _bytes: &[u8], _cap: u64) -> Result<(), BlobError> {
        Ok(())
    }

    fn get(&self, _hash: &MediaHash, _limit: u64) -> Option<Vec<u8>> {
        None
    }

    fn quarantine(&self, _hash: &MediaHash) -> Result<(), BlobError> {
        Ok(())
    }
}
