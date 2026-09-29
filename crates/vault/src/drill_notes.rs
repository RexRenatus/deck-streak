//! The drill notes' folder (SPEC-110 R1 to R7, R9): listing and viewing the Active drills, reading
//! the Graded ones, and the one write the vault contract lets this context make, an answer's append
//! (ADR-011).
//!
//! The folder is the layout's `drill-coach` folder. A missing `Active` or `Graded` subfolder is an
//! empty list. A note that cannot be read is left out and logged by its error's type, never by its
//! path. A note is written only through the atomic writer and only after the rails pass it.

use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use serde_json::Value;

use crate::config::{StartRefusal, VaultSettings};
use crate::drill_store::{self, Surface};
use crate::drills::{self, ACTIVE, Appended, DrillMeta, DrillView, GRADED, GradedDrill};
use crate::fs::{EntryKind, VaultFs};
use crate::rails::Rails;
use crate::staged::VENDORED_LAYOUT;
use crate::{VaultError, atomic};

/// What an answer came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// The answer was appended and its marker ticked; the drill's title, or its id.
    Appended {
        /// The drill's title.
        title: String,
    },
    /// The answer was empty or all space: nothing was read or written.
    EmptyAnswer,
    /// There is no such note in the Active folder.
    NotActive,
    /// The drill has an answer already, by its row or by its ticked marker.
    AlreadyAnswered,
    /// The rails refused the note as it would have been written: nothing was written.
    RailRefused,
}

/// The drill notes of one vault.
#[derive(Debug)]
pub struct DrillNotes<F: VaultFs> {
    fs: F,
    rails: Rails,
    folder: PathBuf,
}

/// The drills folder's name in the layout.
fn drills_folder_name() -> Option<String> {
    let layout: Value = serde_json::from_str(VENDORED_LAYOUT).ok()?;
    layout
        .get("duties")?
        .get("drill-coach")?
        .get("writes")?
        .get(0)?
        .as_str()
        .map(str::to_owned)
}

impl<F: VaultFs> DrillNotes<F> {
    /// Opens the drill notes of the vault `settings` names.
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] when the vault root is not a folder, and [`VaultError::Io`].
    pub fn open(settings: &VaultSettings, fs: F, rails: Rails) -> Result<Self, VaultError> {
        let root = match fs.canonicalize(settings.root.path()) {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StartRefusal::RootNotADirectory.into());
            }
            Err(error) => return Err(VaultError::io("resolve the vault root")(error)),
        };
        if fs
            .kind(&root)
            .map_err(VaultError::io("read the vault root"))?
            != Some(EntryKind::Dir)
        {
            return Err(StartRefusal::RootNotADirectory.into());
        }
        let name = drills_folder_name().ok_or(VaultError::NotAFolder)?;
        Ok(Self {
            fs,
            rails,
            folder: root.join(name),
        })
    }

    /// The Active folder.
    fn active(&self) -> PathBuf {
        self.folder.join(ACTIVE)
    }

    /// The names of the `.md` notes directly inside `folder`, sorted, or none when it is missing.
    fn notes(&self, folder: &Path) -> Result<Vec<String>, VaultError> {
        let entries = match self.fs.list(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(VaultError::io("list a folder")(error)),
        };
        let mut stems: Vec<String> = entries
            .into_iter()
            .filter_map(|entry| entry.name.into_string().ok())
            .filter_map(|name| name.strip_suffix(".md").map(str::to_owned))
            .filter(|stem| !stem.is_empty())
            .collect();
        stems.sort();
        Ok(stems)
    }

    /// The text of the note `stem` in `folder` with universal newlines, or nothing when it cannot
    /// be read. The reason is logged by its type.
    fn read(&self, folder: &Path, stem: &str) -> Option<String> {
        let path = folder.join(format!("{stem}.md"));
        let outcome = self
            .fs
            .read(&path)
            .map_err(|error| format!("{:?}", error.kind()))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "NotUtf8".to_owned()));
        match outcome {
            Ok(text) => Some(drills::universal_newlines(&text)),
            Err(kind) => {
                tracing::warn!(error_type = %kind, "a drill note could not be read");
                None
            }
        }
    }

    /// Every Active drill, sorted by file name, as of `today` (R2). A note that cannot be read is
    /// left out.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the folder cannot be listed.
    pub fn list_active(&self, today: StudyDay) -> Result<Vec<DrillMeta>, VaultError> {
        let folder = self.active();
        Ok(self
            .notes(&folder)?
            .iter()
            .filter_map(|stem| {
                self.read(&folder, stem)
                    .map(|raw| drills::read_meta(stem, &raw, today))
            })
            .collect())
    }

    /// The single view of the Active drill `id` (R2), or nothing for an unsafe id, a missing note
    /// or one that cannot be read. An unsafe id is refused before any read.
    #[must_use]
    pub fn view(&self, id: &str, today: StudyDay) -> Option<DrillView> {
        if !drills::safe_stem(id) {
            return None;
        }
        self.read(&self.active(), id)
            .map(|raw| drills::read_view(id, &raw, today))
    }

    /// The graded drills in the Graded folder (R8, R9), sorted by file name. A missing Graded folder
    /// is none.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the folder cannot be listed.
    pub fn graded(&self) -> Result<Vec<GradedDrill>, VaultError> {
        let folder = self.folder.join(GRADED);
        Ok(self
            .notes(&folder)?
            .iter()
            .filter_map(|stem| {
                self.read(&folder, stem)
                    .and_then(|raw| drills::parse_graded(stem, &raw))
            })
            .collect())
    }

    /// Answers the Active drill `id` (R4 to R6): the marker ticked, the answer appended under its
    /// heading, and the drill's answer row made, in one write. A note write that fails rolls the
    /// row back, and a drill that has a row is `AlreadyAnswered` whatever its marker reads.
    ///
    /// # Errors
    ///
    /// [`VaultError::Database`] when the row cannot be written, and [`VaultError::Io`] or
    /// [`VaultError::NotARegularFile`] when the note cannot be read or written.
    #[allow(clippy::too_many_arguments)]
    pub async fn answer(
        &self,
        db: &Db,
        id: &str,
        answer: &str,
        surface: Surface,
        day: StudyDay,
        when: &str,
        at: UtcMillis,
    ) -> Result<AnswerOutcome, VaultError> {
        if drills::py_strip(answer).is_empty() {
            return Ok(AnswerOutcome::EmptyAnswer);
        }
        let path = self.active().join(format!("{id}.md"));
        if !drills::safe_stem(id)
            || self
                .fs
                .kind(&path)
                .map_err(VaultError::io("read an entry"))?
                != Some(EntryKind::File)
        {
            return Ok(AnswerOutcome::NotActive);
        }
        let bytes = self.fs.read(&path).map_err(VaultError::io("read a note"))?;
        let raw = String::from_utf8(bytes)
            .map_err(|_| VaultError::Malformed(crate::note::Malformed::NotUtf8))?;
        let raw = drills::universal_newlines(&raw);
        let Appended::Written { text, title } = drills::append_answer(id, &raw, answer, when)
        else {
            return Ok(AnswerOutcome::AlreadyAnswered);
        };
        let mut write = db.write().await?;
        if !drill_store::insert_answer(&mut write, id, day, surface, at).await? {
            return Ok(AnswerOutcome::AlreadyAnswered);
        }
        match self.rails.check(&text).into_result() {
            Ok(()) => {}
            Err(_) => return Ok(AnswerOutcome::RailRefused),
        }
        atomic::write(&self.fs, &path, text.as_bytes())?;
        write
            .commit()
            .await
            .map_err(deck_streak_kernel::KernelError::from)?;
        Ok(AnswerOutcome::Appended { title })
    }
}
