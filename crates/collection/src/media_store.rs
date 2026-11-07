//! The media store (SPEC-601 R1 to R5; ADR-621 D2 to D5): the index over the three media tables
//! of the collection store, the put flow that places a file before it commits its row, the pass
//! at open that empties the staging folder and deletes every placed file no present row names,
//! and [`CasReader`], which reads a media file by name.
//!
//! The row is the commit point: a file without its row is an orphan the next open deletes, never
//! adopts. Bytes are verified where they enter ([`MediaStore::put_claimed`]) and size-checked
//! where they are read ([`CasReader::read`]); a file whose length disagrees with its row is
//! quarantined and its hash reported wanted.

use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use deck_streak_media::MediaHash;
use rusqlite::{OptionalExtension, params};

use crate::blob::{BlobError, BlobStore, FsBlobStore, PLACED, STAGING};
use crate::error::StoreError;
use crate::sql;
use crate::store::Store;

/// The largest media file the store takes: 100 MiB, the largest file the engine's media sync
/// carries. A face reads at most its own 4 MiB cap of a file (ADR-359 D1), so the store keeps
/// every file a face may refuse as too large.
pub const FILE_CAP: u64 = 100 * 1024 * 1024;

/// The longest media name, in bytes, as the `media_name` table checks it.
const NAME_MAX: usize = 255;

/// A `media_file` row's state: the file is present at its hash's path.
const PRESENT: i64 = 0;

/// A `media_file` row's state: the file was quarantined or found missing, and is wanted.
const WANTED: i64 = 1;

/// A `media_file` row's kind, until the kinds are classified: not yet known.
const UNCLASSIFIED: i64 = 0;

/// A `media_name` row's origin: bound by a put on this store.
const BOUND_BY_PUT: i64 = 1;

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
        let media = Self {
            store: Mutex::new(store),
            blobs: FsBlobStore::new(root),
            wanted: Mutex::new(Vec::new()),
        };
        media.empty_staging()?;
        media.delete_orphans()?;
        Ok(media)
    }

    /// Deletes every entry of the staging folder: a put the last run never finished.
    fn empty_staging(&self) -> Result<(), MediaError> {
        let Some(entries) = entries(&self.blobs.root().join(STAGING))? else {
            return Ok(());
        };
        for entry in entries {
            let path = entry.path();
            if entry.file_type().map_err(BlobError::Io)?.is_dir() {
                fs::remove_dir_all(path).map_err(BlobError::Io)?;
            } else {
                fs::remove_file(path).map_err(BlobError::Io)?;
            }
        }
        Ok(())
    }

    /// Deletes every placed file whose hash has no present `media_file` row; a file is never
    /// adopted by the open.
    fn delete_orphans(&self) -> Result<(), MediaError> {
        let Some(prefixes) = entries(&self.blobs.root().join(PLACED))? else {
            return Ok(());
        };
        for prefix in prefixes {
            let Some(files) = entries(&prefix.path())? else {
                continue;
            };
            for file in files {
                let relative = format!(
                    "{}/{}",
                    prefix.file_name().to_string_lossy(),
                    file.file_name().to_string_lossy()
                );
                let Some(hash) = MediaHash::from_blob_path(relative.as_bytes()) else {
                    continue;
                };
                if self.state_of(&hash)? != Some(PRESENT) {
                    fs::remove_file(file.path()).map_err(BlobError::Io)?;
                }
            }
        }
        Ok(())
    }

    /// The state of the `media_file` row of `hash`, or `None` when there is no row.
    fn state_of(&self, hash: &MediaHash) -> Result<Option<i64>, StoreError> {
        self.lock().read(|snapshot| {
            Ok(snapshot
                .conn()
                .query_row(sql::MEDIA_FILE_STATE, [hash.as_bytes().as_slice()], |row| {
                    row.get(0)
                })
                .optional()?)
        })
    }

    /// The collection store, held for one read or write.
    fn lock(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Commits the present `media_file` row of `hash`, `size` bytes long.
    fn commit_file(&self, hash: &MediaHash, size: usize, now_ms: i64) -> Result<(), MediaError> {
        let size = i64::try_from(size).map_err(|_| BlobError::TooLarge)?;
        self.lock().write(|tx| {
            tx.conn().execute(
                sql::MEDIA_FILE_PUT,
                params![hash.as_bytes().as_slice(), size, UNCLASSIFIED, now_ms],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Places `bytes` under their hash, commits the file's row, then binds `name` to it, and
    /// answers the hash.
    ///
    /// # Errors
    ///
    /// [`MediaError::Name`] for a name outside 1 to 255 bytes, [`MediaError::Blob`] when the file
    /// cannot be placed, and [`MediaError::Store`] when a row cannot be written.
    pub fn put(&self, name: &str, bytes: &[u8], now_ms: i64) -> Result<MediaHash, MediaError> {
        if name.is_empty() || name.len() > NAME_MAX {
            return Err(MediaError::Name);
        }
        let hash = self.blobs.put(bytes, FILE_CAP)?;
        self.commit_file(&hash, bytes.len(), now_ms)?;
        self.lock().write(|tx| {
            tx.conn().execute(
                sql::MEDIA_NAME_BIND,
                params![name, hash.as_bytes().as_slice(), BOUND_BY_PUT, now_ms],
            )?;
            Ok(())
        })?;
        Ok(hash)
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
        self.blobs.put_claimed(claimed, bytes, FILE_CAP)?;
        self.commit_file(claimed, bytes.len(), now_ms)
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

    /// Records `hash` as wanted, once.
    fn want(&self, hash: MediaHash) {
        let mut wanted = self.wanted.lock().unwrap_or_else(PoisonError::into_inner);
        if !wanted.contains(&hash) {
            wanted.push(hash);
        }
    }

    /// Quarantines the file of `hash`, marks its row wanted and records it.
    fn quarantine(&self, hash: MediaHash) {
        let _ = self.blobs.quarantine(&hash);
        let _ = self.lock().write(|tx| {
            tx.conn().execute(
                sql::MEDIA_FILE_QUARANTINE,
                params![hash.as_bytes().as_slice(), WANTED],
            )?;
            Ok(())
        });
        self.want(hash);
    }

    /// The collection store, closed over by the media store.
    #[must_use]
    pub fn into_store(self) -> Store {
        self.store
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The entries of the folder at `path`, or `None` when there is no folder.
fn entries(path: &Path) -> Result<Option<Vec<fs::DirEntry>>, BlobError> {
    match fs::read_dir(path) {
        Ok(read) => Ok(Some(read.collect::<Result<Vec<_>, _>>()?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(BlobError::Io(error)),
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
        let (hash, size, state) = self.bound(name)?;
        if state != PRESENT {
            self.media.want(hash);
            return None;
        }
        let size = u64::try_from(size).ok()?;
        if limit <= size {
            let bytes = self.media.blobs.get(&hash, limit);
            if bytes.is_none() {
                self.media.want(hash);
            }
            return bytes;
        }
        let Some(bytes) = self.media.blobs.get(&hash, size.saturating_add(1)) else {
            self.media.want(hash);
            return None;
        };
        if u64::try_from(bytes.len()).ok() == Some(size) {
            Some(bytes)
        } else {
            self.media.quarantine(hash);
            None
        }
    }

    /// The hash, size and state of the file a live `name` is bound to.
    fn bound(self, name: &str) -> Option<(MediaHash, i64, i64)> {
        let row: Option<(Vec<u8>, i64, i64)> = self
            .media
            .lock()
            .read(|snapshot| {
                Ok(snapshot
                    .conn()
                    .query_row(sql::MEDIA_NAME_READ, [name], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .optional()?)
            })
            .ok()?;
        let (hash, size, state) = row?;
        let hash: [u8; 32] = hash.try_into().ok()?;
        Some((MediaHash::from_bytes(hash), size, state))
    }
}
