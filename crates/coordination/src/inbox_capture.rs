//! The inbox capture use case (SPEC-118 R3, R4, R5, R10): the one capture both surfaces call. The
//! bot's media (V1b) streams its attachment through [`InboxCaptures::stream`] and then calls
//! [`StreamingCapture::capture`]; the Mini App's quick capture calls [`InboxCaptures::quick`],
//! which bounds its text and writes the same capture with no attachment.
//!
//! Each capture locates the inbox anew, so a vault folder removed after start reads
//! `vault_missing` and no folder is ever created (R4). Nothing here writes a file: every write is
//! the vault's `inbox::capture`, the protocol `formal/tla/CaptureOnce` models, and this module adds
//! no step to it (no second claim and no retry).

use std::path::PathBuf;

use deck_streak_kernel::{Db, UtcMillis};
use deck_streak_vault::inbox;
pub use deck_streak_vault::inbox::{
    Attachment, Capture, CaptureKind, Captured, Inbox, QUICK_TEXT_CHARS, Source,
};
pub use deck_streak_vault::{JournalGuard, LayoutInForce, RealFs, VaultError, VaultFs};

/// A quick capture's kind (R10): the Mini App sends `text` or `journal`, and a journal capture
/// still lands in the inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickKind {
    /// A plain note.
    Text,
    /// A note the owner marked for the journal.
    Journal,
}

impl QuickKind {
    /// The kind the Mini App names, `text` or `journal`, or `None` for any other name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "journal" => Some(Self::Journal),
            _ => None,
        }
    }

    /// The capture kind the vault writes for it.
    #[must_use]
    pub const fn kind(self) -> CaptureKind {
        match self {
            Self::Text => CaptureKind::Text,
            Self::Journal => CaptureKind::Journal,
        }
    }
}

/// What a quick capture answered (R10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuickAnswer {
    /// The capture was written; `name` is its stub's file.
    Saved {
        /// The file the owner looks for.
        name: String,
    },
    /// The capture id was already recorded: nothing was written, and `name` is the first
    /// capture's file (`already_captured`).
    AlreadyCaptured {
        /// The recorded capture's file.
        name: String,
    },
    /// The text is empty or longer than [`QUICK_TEXT_CHARS`] after the stub's trim: nothing was
    /// written.
    TextOutOfBounds,
}

/// The vault's inbox as a role opened it: the file system, guarded against every journal folder of
/// the layout in force, the vault root and that layout.
pub struct InboxCaptures<F: VaultFs> {
    fs: JournalGuard<F>,
    root: PathBuf,
    layout: LayoutInForce,
}

impl<F: VaultFs> std::fmt::Debug for InboxCaptures<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InboxCaptures(..)")
    }
}

impl<F: VaultFs> InboxCaptures<F> {
    /// The captures of the inbox `layout` names inside `root`, written through `fs`. It creates
    /// nothing. It resolves the root once, so the guard names each journal folder by the resolved
    /// path the inbox's files are written at; a root that does not resolve yet is kept as given.
    #[must_use]
    pub fn new(fs: F, root: PathBuf, layout: LayoutInForce) -> Self {
        let resolved = fs
            .canonicalize(&root)
            .unwrap_or_else(|_missing| root.clone());
        let journal = layout.journal_paths(&resolved);
        Self {
            fs: JournalGuard::new(fs, journal),
            root,
            layout,
        }
    }

    /// The inbox, located for this capture (R4).
    fn inbox(&self) -> Result<Inbox, VaultError> {
        Inbox::locate(&self.fs, &self.root, &self.layout)
    }

    /// The streamed entry the bot calls (V1b): locates the inbox and starts `capture`'s attachment,
    /// whose bytes then stream into its temporary file before [`StreamingCapture::capture`].
    ///
    /// # Errors
    ///
    /// [`VaultError::VaultMissing`] when the vault root or the inbox folder is missing,
    /// [`VaultError::JournalRefused`] when the inbox lies under a journal folder, and the vault's
    /// errors for the extension and the temporary file.
    pub fn stream(
        &self,
        capture: Capture,
        ext: &str,
    ) -> Result<StreamingCapture<'_, F>, VaultError> {
        let located = self.inbox()?;
        let attachment = located.attachment(&self.fs, &capture, ext)?;
        Ok(StreamingCapture {
            captures: self,
            inbox: located,
            capture,
            attachment,
        })
    }

    /// The Mini App's quick capture (R10): `text` of `kind` under the retry key `capture_id`, at
    /// `at`. A text out of bounds writes nothing; a capture id already recorded answers the first
    /// capture's name, whatever day it was sent on.
    ///
    /// # Errors
    ///
    /// [`VaultError::VaultMissing`] when the vault root or the inbox folder is missing,
    /// [`VaultError::JournalRefused`], and the vault's errors for the ledger and the write.
    pub async fn quick(
        &self,
        db: &Db,
        capture_id: &str,
        kind: QuickKind,
        text: &str,
        at: UtcMillis,
    ) -> Result<QuickAnswer, VaultError> {
        if !inbox::quick_text_fits(text) {
            // The text is the owner's: the event names the outcome, never the text.
            tracing::info!(
                outcome = "text_out_of_bounds",
                "a quick capture wrote nothing"
            );
            return Ok(QuickAnswer::TextOutOfBounds);
        }
        let located = self.inbox()?;
        let capture = Capture {
            kind: kind.kind(),
            source: Source::MiniApp,
            unique: capture_id.to_owned(),
            when: at,
            caption: text.to_owned(),
        };
        Ok(
            match inbox::capture(db, &self.fs, &located, &capture, None).await? {
                Captured::Saved { name } => QuickAnswer::Saved { name },
                Captured::AlreadyCaptured { name } => QuickAnswer::AlreadyCaptured { name },
            },
        )
    }
}

/// A capture whose attachment is streaming into its temporary file. Dropped before
/// [`StreamingCapture::capture`], it removes the temporary file and records nothing.
pub struct StreamingCapture<'c, F: VaultFs> {
    captures: &'c InboxCaptures<F>,
    inbox: Inbox,
    capture: Capture,
    attachment: Attachment<'c, JournalGuard<F>>,
}

impl<F: VaultFs> std::fmt::Debug for StreamingCapture<'_, F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StreamingCapture")
            .field("name", &self.attachment.name())
            .finish_non_exhaustive()
    }
}

impl<F: VaultFs> StreamingCapture<'_, F> {
    /// The attachment's file name in the inbox.
    #[must_use]
    pub fn name(&self) -> &str {
        self.attachment.name()
    }

    /// Appends `chunk` to the attachment's temporary file.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the write fails; drop the capture then, which removes the file.
    pub fn write(&mut self, chunk: &[u8]) -> Result<(), VaultError> {
        self.attachment.write(chunk)
    }

    /// Writes the capture once (R3): the claim, the attachment's rename, the stub last, the commit.
    ///
    /// # Errors
    ///
    /// The vault's errors for the ledger and the write; on any error nothing is recorded.
    pub async fn capture(self, db: &Db) -> Result<Captured, VaultError> {
        inbox::capture(
            db,
            &self.captures.fs,
            &self.inbox,
            &self.capture,
            Some(self.attachment),
        )
        .await
    }
}
