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

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

use crate::VaultError;
use crate::atomic::{self, Streamed};
use crate::capture_store::{self, CaptureRow, Claim};
use crate::config::LayoutInForce;
use crate::fs::{EntryKind, VaultFs};

/// The longest safe unique a stem keeps, in characters (`[:32]`).
pub const UNIQUE_CHARS: usize = 32;
/// The safe unique of a unique that keeps no character.
pub const FALLBACK_UNIQUE: &str = "capture";
/// The extension of an attachment given none.
pub const FALLBACK_EXTENSION: &str = ".bin";
/// The longest plain extension, in characters, without its dot (ADR-118).
pub const EXTENSION_CHARS: usize = 10;
/// The longest quick capture's text, in characters after the stub's trim: SPEC-118 R10's bound on
/// the Mini App's one text field.
pub const QUICK_TEXT_CHARS: usize = 4_000;
/// One UTC day, in milliseconds: a stem's date is the capture instant's UTC day.
const DAY_MS: i64 = 86_400_000;

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
    let safe: String = unique
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(UNIQUE_CHARS)
        .collect();
    if safe.is_empty() {
        FALLBACK_UNIQUE.to_owned()
    } else {
        safe
    }
}

/// The stem `<UTC date of when>-<kind>-<safe unique>` (R1). It holds no dot.
#[must_use]
pub fn stem(kind: CaptureKind, unique: &str, when: UtcMillis) -> String {
    let day = StudyDay::from_epoch_day(when.epoch_millis().div_euclid(DAY_MS));
    format!("{day}-{}-{}", kind.as_str(), safe_unique(unique))
}

/// The attachment's extension, with its leading dot: a plain one (up to ten ASCII letters or
/// digits, with or without a leading dot) is kept as it came, and an empty one reads
/// [`FALLBACK_EXTENSION`].
///
/// # Errors
///
/// [`VaultError::InvalidExtension`] for any other text, which could name a path.
pub fn extension(ext: &str) -> Result<String, VaultError> {
    let bare = ext.strip_prefix('.').unwrap_or(ext);
    if bare.is_empty() {
        return Ok(FALLBACK_EXTENSION.to_owned());
    }
    if bare.len() <= EXTENSION_CHARS && bare.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Ok(format!(".{bare}"))
    } else {
        Err(VaultError::InvalidExtension)
    }
}

/// The attachment's file name: `<stem><extension>`, except that an extension equal to `.md` in any
/// case is named `<stem>.attachment<extension>`. A stem holds no dot, so a stub name holds one and
/// this name two: the attachment never takes its stub's name (ADR-118's amendment).
#[must_use]
pub fn attachment_name(stem: &str, extension: &str) -> String {
    if extension.eq_ignore_ascii_case(".md") {
        format!("{stem}.attachment{extension}")
    } else {
        format!("{stem}{extension}")
    }
}

/// The stub's file name, `<stem>.md`.
#[must_use]
pub fn stub_name(stem: &str) -> String {
    format!("{stem}.md")
}

/// The capture instant to the second, with its offset, as the predecessor's `isoformat` writes
/// it: `YYYY-MM-DDTHH:MM:SS+00:00`. A second is floored, so an instant before the epoch keeps its
/// second.
fn instant(when: UtcMillis) -> String {
    let second = when.epoch_millis().div_euclid(1_000);
    let day = StudyDay::from_epoch_day(second.div_euclid(86_400));
    let into = second.rem_euclid(86_400);
    format!(
        "{day}T{:02}:{:02}:{:02}+00:00",
        into / 3_600,
        into % 3_600 / 60,
        into % 60
    )
}

/// A character Python's `str.strip` trims: Unicode white space, plus the separators U+001C to
/// U+001F, which Rust's `char::is_whitespace` keeps.
fn is_python_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// A Telegram capture's stub (R2).
#[must_use]
pub fn telegram_stub(
    kind: CaptureKind,
    when: UtcMillis,
    attachment: &str,
    caption: &str,
) -> String {
    let mut stub = format!(
        "---\nstatus: captured\nsource: telegram\nkind: {}\ncaptured: {}\nattachment: \
         {attachment}\ntags: [inbox, telegram-capture]\n---\n\nCaptured via Telegram. \
         Attachment: [[{attachment}]]\n",
        kind.as_str(),
        instant(when)
    );
    let caption = caption.trim_matches(is_python_space);
    if !caption.is_empty() {
        stub.push('\n');
        stub.push_str(caption);
        stub.push('\n');
    }
    stub
}

/// A Mini App capture's stub (R10): R2's keys without `attachment`, `source: miniapp`, and the
/// text after the line "Captured via the Mini App.".
#[must_use]
pub fn miniapp_stub(kind: CaptureKind, when: UtcMillis, text: &str) -> String {
    format!(
        "---\nstatus: captured\nsource: miniapp\nkind: {}\ncaptured: {}\ntags: [inbox, \
         miniapp-capture]\n---\n\nCaptured via the Mini App.\n\n{}\n",
        kind.as_str(),
        instant(when),
        text.trim_matches(is_python_space)
    )
}

/// Whether `text` is a quick capture's text (R10): 1 to [`QUICK_TEXT_CHARS`] characters after the
/// trim [`miniapp_stub`] applies, so the bound counts exactly what the stub holds.
#[must_use]
pub fn quick_text_fits(text: &str) -> bool {
    let chars = text.trim_matches(is_python_space).chars().count();
    (1..=QUICK_TEXT_CHARS).contains(&chars)
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
        let configured = root.join(&layout.inbox);
        if !is_folder(fs, root, "read the vault root")?
            || !is_folder(fs, &configured, "read the inbox folder")?
        {
            return Err(VaultError::VaultMissing);
        }
        atomic::refuse_journal(&layout.journal_paths(root), &configured)?;
        let resolved_root = fs
            .canonicalize(root)
            .map_err(VaultError::io("resolve the vault root"))?;
        let folder = fs
            .canonicalize(&configured)
            .map_err(VaultError::io("resolve the inbox folder"))?;
        let journal: Vec<PathBuf> = layout
            .journal_paths(&resolved_root)
            .iter()
            .map(|folder| atomic::resolve(fs, folder))
            .collect();
        atomic::refuse_journal(&journal, &folder)?;
        if folder == resolved_root || !folder.starts_with(&resolved_root) {
            return Err(VaultError::OutsideConfinement);
        }
        Ok(Self { folder })
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
        let extension = extension(ext)?;
        let name = attachment_name(
            &stem(capture.kind, &capture.unique, capture.when),
            &extension,
        );
        let stream = atomic::stream(fs, &self.folder.join(&name))?;
        Ok(Attachment { name, stream })
    }
}

/// Whether `path` is a folder, through a link or not; a missing path is not one.
fn is_folder<F: VaultFs + ?Sized>(
    fs: &F,
    path: &Path,
    step: &'static str,
) -> Result<bool, VaultError> {
    match fs.kind(path).map_err(VaultError::io(step))? {
        Some(EntryKind::Dir) => Ok(true),
        Some(EntryKind::Symlink) => match fs.canonicalize(path) {
            Ok(target) => {
                Ok(fs.kind(&target).map_err(VaultError::io(step))? == Some(EntryKind::Dir))
            }
            Err(_dangling) => Ok(false),
        },
        Some(_) | None => Ok(false),
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
/// place, writes the stub last and commits. A stem, or a Mini App capture's key, already recorded
/// answers the recorded name, the transaction rolls back, and the attachment's temporary file is
/// removed (ADR-118 and its capture-key amendment).
///
/// # Errors
///
/// [`VaultError::CaptureUnpaired`] when a Telegram capture has no attachment or a Mini App capture
/// has one, [`VaultError::Database`] when the ledger refuses, [`VaultError::JournalRefused`], and
/// [`VaultError::Io`] naming the step that failed; on any error nothing is recorded.
pub async fn capture<F: VaultFs + ?Sized>(
    db: &Db,
    fs: &F,
    inbox: &Inbox,
    capture: &Capture,
    attachment: Option<Attachment<'_, F>>,
) -> Result<Captured, VaultError> {
    let stem = stem(capture.kind, &capture.unique, capture.when);
    let capture_key = safe_unique(&capture.unique);
    let stub = match (capture.source, &attachment) {
        (Source::Telegram, Some(file)) => {
            telegram_stub(capture.kind, capture.when, &file.name, &capture.caption)
        }
        (Source::MiniApp, None) => miniapp_stub(capture.kind, capture.when, &capture.caption),
        (Source::Telegram, None) | (Source::MiniApp, Some(_)) => {
            return Err(VaultError::CaptureUnpaired);
        }
    };
    let attachment_file = attachment.as_ref().map(|file| file.name.clone());
    let row = CaptureRow {
        stem: &stem,
        capture_key: &capture_key,
        kind: capture.kind,
        source: capture.source,
        attachment: attachment_file.as_deref(),
        captured_at: capture.when,
    };
    let mut transaction = db.write().await?;
    if let Claim::Recorded { name } = capture_store::claim(&mut transaction, &row).await? {
        transaction.rollback().await.map_err(KernelError::from)?;
        if let Some(file) = attachment {
            file.stream.discard();
        }
        tracing::info!(
            source = capture.source.as_str(),
            kind = capture.kind.as_str(),
            outcome = "already_captured",
            "an inbox capture was already recorded"
        );
        return Ok(Captured::AlreadyCaptured { name });
    }
    if let Some(file) = attachment {
        file.stream.land()?;
    }
    atomic::write(fs, &inbox.folder.join(stub_name(&stem)), stub.as_bytes())?;
    transaction.commit().await.map_err(KernelError::from)?;
    tracing::info!(
        source = capture.source.as_str(),
        kind = capture.kind.as_str(),
        outcome = "saved",
        "an inbox capture was saved"
    );
    Ok(Captured::Saved {
        name: attachment_file.unwrap_or_else(|| stub_name(&stem)),
    })
}
