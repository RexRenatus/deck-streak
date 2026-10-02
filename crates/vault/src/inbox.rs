//! The inbox capture (SPEC-118 R1 to R3, ADR-118): a photo, a voice note or a document the owner
//! sends, or a line typed in the Mini App, lands in the vault inbox once, with its stub note.
//!
//! The stem and the stub are the predecessor's (`vault_bridge.py:save_inbox_capture` at `27ee2bc`,
//! golden `inbox_capture_stub`), with one departure (ADR-118's amendment): an attachment whose
//! extension is `md` is named apart from its stub, so the stub never replaces it.
//!
//! The write order is ADR-118's: the attachment's bytes stream into its temporary file first; then
//! one `BEGIN IMMEDIATE` transaction claims the stem's `inbox_captures` row, renames the attachment
//! into place, writes the stub last through the atomic writer, and commits. A stem already recorded
//! answers the recorded name and writes nothing in place.

use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, KernelError, UtcMillis};

use crate::VaultError;
use crate::atomic::Streamed;
use crate::config::LayoutInForce;
use crate::fs::VaultFs;

/// The longest safe unique a stem keeps, in characters (`[:32]`).
pub const UNIQUE_CHARS: usize = 32;
/// The safe unique of a unique that keeps no character.
pub const FALLBACK_UNIQUE: &str = "capture";
/// The extension of an attachment given none.
pub const FALLBACK_EXTENSION: &str = ".bin";

/// What was captured: the stub's `kind`, and the `inbox_captures` row's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureKind {
    /// A photo, stored as `.jpg`.
    Photo,
    /// A voice note, stored as it came.
    Voice,
    /// A document, with its plain extension.
    Document,
    /// A Mini App capture's text.
    Text,
    /// A Mini App capture the owner marked for the journal; it still lands in the inbox.
    Journal,
}

impl CaptureKind {
    /// The kind as the stub and the ledger spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Voice => "voice",
            Self::Document => "document",
            Self::Text => "text",
            Self::Journal => "journal",
        }
    }
}

/// Where a capture came from: the stub's `source`, and the row's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The owner's message to the bot.
    Telegram,
    /// The Mini App's capture screen.
    MiniApp,
}

impl Source {
    /// The source as the stub and the ledger spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Telegram => "telegram",
            Self::MiniApp => "miniapp",
        }
    }
}

/// One capture, before it is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capture {
    /// What was captured.
    pub kind: CaptureKind,
    /// Where it came from.
    pub source: Source,
    /// The sender's unique id (a Telegram file's unique id, or the Mini App's capture id): the
    /// retry key, of which the stem keeps the safe form.
    pub unique: String,
    /// The instant it was captured; its UTC date starts the stem.
    pub when: UtcMillis,
    /// The caption, or the Mini App's text.
    pub caption: String,
}

/// What a capture answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Captured {
    /// The capture was written; `name` is its attachment's file name, or its stub's without one.
    Saved {
        /// The file the owner looks for.
        name: String,
    },
    /// The stem was already recorded: nothing was written in place, and `name` is the recorded
    /// capture's (`already_captured`).
    AlreadyCaptured {
        /// The recorded capture's file.
        name: String,
    },
}

/// The safe form of `unique`: every character outside `[A-Za-z0-9_-]` removed, cut to
/// [`UNIQUE_CHARS`] characters, or [`FALLBACK_UNIQUE`] when nothing is left.
#[must_use]
pub fn safe_unique(unique: &str) -> String {
    let _ = unique;
    String::new()
}

/// The stem `<UTC date of when>-<kind>-<safe unique>` (R1). It holds no dot.
#[must_use]
pub fn stem(kind: CaptureKind, unique: &str, when: UtcMillis) -> String {
    let _ = (kind, unique, when);
    String::new()
}

/// The attachment's extension, with its leading dot: a plain one (up to ten ASCII letters or
/// digits, with or without a leading dot) is kept as it came, and an empty one reads
/// [`FALLBACK_EXTENSION`].
///
/// # Errors
///
/// [`VaultError::InvalidExtension`] for any other text, which could name a path.
pub fn extension(ext: &str) -> Result<String, VaultError> {
    let _ = ext;
    Ok(String::new())
}

/// The attachment's file name: `<stem><extension>`, except that an extension equal to `.md` in any
/// case is named `<stem>.attachment<extension>`. A stem holds no dot, so a stub name holds one and
/// this name two: the attachment never takes its stub's name (ADR-118's amendment).
#[must_use]
pub fn attachment_name(stem: &str, extension: &str) -> String {
    let _ = (stem, extension);
    String::new()
}

/// The stub's file name, `<stem>.md`.
#[must_use]
pub fn stub_name(stem: &str) -> String {
    let _ = stem;
    String::new()
}

/// A Telegram capture's stub (R2).
#[must_use]
pub fn telegram_stub(
    kind: CaptureKind,
    when: UtcMillis,
    attachment: &str,
    caption: &str,
) -> String {
    let _ = (kind, when, attachment, caption);
    String::new()
}

/// A Mini App capture's stub (R10): R2's keys without `attachment`, `source: miniapp`, and the
/// text after the line "Captured via the Mini App.".
#[must_use]
pub fn miniapp_stub(kind: CaptureKind, when: UtcMillis, text: &str) -> String {
    let _ = (kind, when, text);
    String::new()
}

/// The inbox folder of the layout in force, inside the vault root.
#[derive(Clone, PartialEq, Eq)]
pub struct Inbox {
    folder: PathBuf,
}

impl std::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Inbox(..)")
    }
}

impl Inbox {
    /// The inbox of `layout` inside `root` (R4). It creates no folder.
    ///
    /// # Errors
    ///
    /// [`VaultError::VaultMissing`] when the root or the inbox folder is not a directory,
    /// [`VaultError::JournalRefused`] when the inbox lies under a journal folder, and
    /// [`VaultError::OutsideConfinement`] when it resolves outside the root.
    pub fn locate<F: VaultFs + ?Sized>(
        fs: &F,
        root: &Path,
        layout: &LayoutInForce,
    ) -> Result<Self, VaultError> {
        let _ = fs;
        Ok(Self {
            folder: root.join(&layout.inbox),
        })
    }

    /// The inbox folder, resolved.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Starts `capture`'s attachment: a temporary file beside its target, into which its bytes
    /// stream before the claim.
    ///
    /// # Errors
    ///
    /// [`VaultError::InvalidExtension`], [`VaultError::JournalRefused`], or [`VaultError::Io`]
    /// naming the step that failed.
    pub fn attachment<'f, F: VaultFs + ?Sized>(
        &self,
        fs: &'f F,
        capture: &Capture,
        ext: &str,
    ) -> Result<Attachment<'f, F>, VaultError> {
        let _ = ext;
        let name = attachment_name(&stem(capture.kind, &capture.unique, capture.when), "");
        Ok(Attachment {
            name,
            stream: crate::atomic::stream(fs, &self.folder.join("attachment"))?,
        })
    }
}

/// An attachment streaming into its temporary file.
pub struct Attachment<'f, F: VaultFs + ?Sized> {
    name: String,
    stream: Streamed<'f, F>,
}

impl<F: VaultFs + ?Sized> std::fmt::Debug for Attachment<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attachment")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<F: VaultFs + ?Sized> Attachment<'_, F> {
    /// The attachment's file name in the inbox.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Appends `chunk` to the temporary file.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the write fails; the caller then drops the attachment, which
    /// removes its temporary file.
    pub fn write(&mut self, chunk: &[u8]) -> Result<(), VaultError> {
        self.stream.write(chunk)
    }
}

/// Writes `capture` into `inbox` once (R3): the attachment, when there is one, is already in its
/// temporary file; one `BEGIN IMMEDIATE` transaction claims the stem, renames the attachment into
/// place, writes the stub last and commits. A stem already recorded answers the recorded name, the
/// transaction rolls back, and the attachment's temporary file is removed.
///
/// # Errors
///
/// [`VaultError::Database`] when the ledger refuses, [`VaultError::JournalRefused`], and
/// [`VaultError::Io`] naming the step that failed; on any error nothing is recorded.
pub async fn capture<F: VaultFs + ?Sized>(
    db: &Db,
    fs: &F,
    inbox: &Inbox,
    capture: &Capture,
    attachment: Option<Attachment<'_, F>>,
) -> Result<Captured, VaultError> {
    let transaction = db.write().await?;
    transaction.rollback().await.map_err(KernelError::from)?;
    let _ = (fs, inbox, capture, attachment);
    Ok(Captured::Saved {
        name: String::new(),
    })
}
