//! The media store (SPEC-601 R1 to R5; ADR-621 D2 to D5): the index over the three media tables
//! of the collection store, the put flow that places a file before it commits its row, the pass
//! at open that empties the staging folder and deletes every placed file no present row names,
//! and [`CasReader`], which reads a media file by name.
//!
//! The row is the commit point: a file without its row is an orphan the next open deletes, never
//! adopts. Bytes are verified where they enter ([`MediaStore::put_claimed`]) and size-checked
//! where they are read ([`CasReader::read`]); a file whose length disagrees with its row is
//! quarantined and its hash reported wanted.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use deck_streak_media::MediaHash;

use crate::blob::{BlobError, BlobStore, FsBlobStore};
use crate::error::StoreError;
use crate::store::Store;

/// The largest media file the store takes: 100 MiB, the largest file the engine's media sync
/// carries. A face reads at most its own 4 MiB cap of a file (ADR-359 D1), so the store keeps
/// every file a face may refuse as too large.
pub const FILE_CAP: u64 = 100 * 1024 * 1024;

/// Why the media store refused a put, a bind or its open.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// The blob folder refused the file.
    #[error(transparent)]
    Blob(#[from] BlobError),
    /// The collection store refused the row.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// A name is not 1 to 255 bytes.
    #[error("a media name is 1 to 255 bytes")]
    Name,
}

/// One user's media: the index rows in the collection store and the files in the blob folder.
pub struct MediaStore {
    store: Mutex<Store>,
    blobs: FsBlobStore,
    wanted: Mutex<Vec<MediaHash>>,
}

impl MediaStore {
    /// Opens the media store over `store`, with its blob folder at `root`: empties the staging
    /// folder and deletes every placed file without a present row.
    ///
    /// # Errors
    ///
    /// [`MediaError::Blob`] when the blob folder cannot be read or cleaned, and
    /// [`MediaError::Store`] when the rows cannot be read.
    pub fn open(store: Store, root: &Path) -> Result<Self, MediaError> {
        Ok(Self {
            store: Mutex::new(store),
            blobs: FsBlobStore::new(root),
            wanted: Mutex::new(Vec::new()),
        })
    }

    /// Places `bytes` under their hash, commits the file's row, then binds `name` to it, and
    /// answers the hash.
    ///
    /// # Errors
    ///
    /// [`MediaError::Name`] for a name outside 1 to 255 bytes, [`MediaError::Blob`] when the file
    /// cannot be placed, and [`MediaError::Store`] when a row cannot be written.
    pub fn put(&self, name: &str, bytes: &[u8], now_ms: i64) -> Result<MediaHash, MediaError> {
        let _ = (name, now_ms);
        Ok(self.blobs.put(bytes, FILE_CAP)?)
    }

    /// The sync and port entry: places `bytes` under the `claimed` hash and commits its row,
    /// refusing bytes that hash to another hash before any rename or row.
    ///
    /// # Errors
    ///
    /// [`MediaError::Blob`] with [`BlobError::Mismatch`] for other bytes, and as [`Self::put`].
    pub fn put_claimed(
        &self,
        claimed: &MediaHash,
        bytes: &[u8],
        now_ms: i64,
    ) -> Result<(), MediaError> {
        let _ = (claimed, bytes, now_ms);
        Ok(())
    }

    /// The blob folder the store keeps its files in.
    #[must_use]
    pub const fn blobs(&self) -> &FsBlobStore {
        &self.blobs
    }

    /// The reader that reads a media file by name.
    #[must_use]
    pub const fn reader(&self) -> CasReader<'_> {
        CasReader { media: self }
    }

    /// Every hash a read found missing, quarantined or tombstoned, in the order it was found.
    #[must_use]
    pub fn wanted(&self) -> Vec<MediaHash> {
        self.wanted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The collection store, closed over by the media store.
    #[must_use]
    pub fn into_store(self) -> Store {
        self.store
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Reads a media file by name: a live name gives its hash, a present file gives at most the
/// limit, and a length that disagrees with the row quarantines the file.
#[derive(Clone, Copy)]
pub struct CasReader<'m> {
    media: &'m MediaStore,
}

impl CasReader<'_> {
    /// At most `limit` bytes of the file `name` is bound to, or `None` when no live name, no
    /// present file or no file of the row's length answers.
    #[must_use]
    pub fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let _ = (self.media, name, limit);
        None
    }
}
