//! The blob store (SPEC-601 R2 to R4 and R8; ADR-621 D1, D4 and D5): each media file kept as a
//! file of the user's blob folder, under its SHA-256. The layout is `<root>/sha256/` joined with
//! [`MediaHash::blob_path`], a put stages its bytes in the root's [`STAGING`] folder, and a file whose length
//! disagrees with its row moves to `<root>/bad/<hex>`.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use deck_streak_media::MediaHash;

/// The folder under the root that holds every placed file.
pub const PLACED: &str = "sha256";

/// The folder under the root that stages a put's bytes before the rename.
pub const STAGING: &str = "tmp";

/// The folder under the root that holds a quarantined file.
pub const QUARANTINE: &str = "bad";

/// How many staging names a put tries before it gives up on a crowded staging folder.
const STAGING_TRIES: u32 = 64;

/// The next staging name's number, so two puts of one process never share a staging file.
static NEXT_STAGED: AtomicU64 = AtomicU64::new(0);

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

/// The steps a put takes on the blob folder, apart from hashing. [`put_flow`] drives them in
/// order and discards what it staged when a later step refuses.
pub trait Staging {
    /// Writes `bytes` to a new staging file.
    ///
    /// # Errors
    ///
    /// [`BlobError::Io`] when the staging file cannot be made or written.
    fn stage(&mut self, bytes: &[u8]) -> Result<(), BlobError>;

    /// Syncs the staging file to its disk.
    ///
    /// # Errors
    ///
    /// [`BlobError::Io`] when the sync fails.
    fn sync(&mut self) -> Result<(), BlobError>;

    /// Renames the staging file to the path of `hash`.
    ///
    /// # Errors
    ///
    /// [`BlobError::Io`] when the rename fails.
    fn place(&mut self, hash: &MediaHash) -> Result<(), BlobError>;

    /// Deletes the staging file, when a step after [`Staging::stage`] refused.
    fn discard(&mut self);
}

/// The put flow over an abstract hasher and staging (ADR-621 D2 and D4): stage the bytes, hash
/// them, refuse them past `cap` or when they miss the `claimed` hash, sync, then place them under
/// their hash. Nothing is placed unless every step before the rename passed, and a refused put
/// leaves no staging file.
///
/// # Errors
///
/// [`BlobError::TooLarge`] past the cap, [`BlobError::Mismatch`] when the bytes miss the claimed
/// hash, and whatever a step of `staging` returns.
pub fn put_flow<S: Staging>(
    staging: &mut S,
    hasher: impl Fn(&[u8]) -> MediaHash,
    bytes: &[u8],
    cap: u64,
    claimed: Option<&MediaHash>,
) -> Result<MediaHash, BlobError> {
    staging.stage(bytes)?;
    let hash = hasher(bytes);
    let checked = if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > cap {
        Err(BlobError::TooLarge)
    } else if claimed.is_some_and(|claimed| *claimed != hash) {
        Err(BlobError::Mismatch)
    } else {
        staging.sync().and_then(|()| staging.place(&hash))
    };
    match checked {
        Ok(()) => Ok(hash),
        Err(error) => {
            staging.discard();
            Err(error)
        }
    }
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

    /// The staging of one put into this folder.
    fn staging(&self) -> FsStaging<'_> {
        FsStaging {
            blobs: self,
            staged: None,
        }
    }

    /// The put flow with SHA-256 over this folder.
    fn place(
        &self,
        bytes: &[u8],
        cap: u64,
        claimed: Option<&MediaHash>,
    ) -> Result<MediaHash, BlobError> {
        put_flow(&mut self.staging(), MediaHash::of, bytes, cap, claimed)
    }
}

/// One put's staging file in the root's [`STAGING`] folder, open until it is placed or discarded.
struct FsStaging<'b> {
    blobs: &'b FsBlobStore,
    staged: Option<(PathBuf, File)>,
}

impl FsStaging<'_> {
    /// A new staging file under a name no other put of this process holds.
    fn create(&self) -> Result<(PathBuf, File), BlobError> {
        let folder = self.blobs.root.join(STAGING);
        fs::create_dir_all(&folder)?;
        for _ in 0..STAGING_TRIES {
            let number = NEXT_STAGED.fetch_add(1, Ordering::Relaxed);
            let path = folder.join(format!("put-{number}"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => return Err(BlobError::Io(error)),
            }
        }
        Err(BlobError::Io(std::io::Error::from(
            ErrorKind::AlreadyExists,
        )))
    }

    /// The open staging file, or the error of a step taken before [`Staging::stage`].
    fn staged(&mut self) -> Result<&mut (PathBuf, File), BlobError> {
        self.staged
            .as_mut()
            .ok_or_else(|| BlobError::Io(std::io::Error::from(ErrorKind::NotFound)))
    }
}

impl Staging for FsStaging<'_> {
    fn stage(&mut self, bytes: &[u8]) -> Result<(), BlobError> {
        let (path, mut file) = self.create()?;
        let written = file.write_all(bytes);
        self.staged = Some((path, file));
        written.map_err(BlobError::Io)
    }

    fn sync(&mut self) -> Result<(), BlobError> {
        let (_, file) = self.staged()?;
        file.sync_all().map_err(BlobError::Io)
    }

    fn place(&mut self, hash: &MediaHash) -> Result<(), BlobError> {
        let target = self.blobs.path_of(hash);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let (path, _) = self.staged()?;
        fs::rename(path.as_path(), &target)?;
        self.staged = None;
        Ok(())
    }

    fn discard(&mut self) {
        if let Some((path, file)) = self.staged.take() {
            drop(file);
            let _ = fs::remove_file(path);
        }
    }
}

impl BlobStore for FsBlobStore {
    fn put(&self, bytes: &[u8], cap: u64) -> Result<MediaHash, BlobError> {
        self.place(bytes, cap, None)
    }

    fn put_claimed(&self, claimed: &MediaHash, bytes: &[u8], cap: u64) -> Result<(), BlobError> {
        self.place(bytes, cap, Some(claimed)).map(|_| ())
    }

    fn get(&self, hash: &MediaHash, limit: u64) -> Option<Vec<u8>> {
        let file = File::open(self.path_of(hash)).ok()?;
        let mut bytes = Vec::new();
        file.take(limit).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    fn quarantine(&self, hash: &MediaHash) -> Result<(), BlobError> {
        let target = self.quarantined_path_of(hash);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(self.path_of(hash), target)?;
        Ok(())
    }
}
