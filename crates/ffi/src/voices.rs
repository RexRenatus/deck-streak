//! The voice choice (SPEC-348 R6; ADR-359 D3).
//!
//! A learner may choose, per language, which installed voice speaks a card's TTS tags. The choice
//! lives in one small file the adapter keeps on the device, outside the collection and its media
//! folder, so nothing of it is synced: one `language<TAB>identifier` line per language, written
//! whole to a temporary file and renamed over the old one. A line that does not parse is skipped,
//! so one damaged line costs that language's choice and no other. Swift lists the installed
//! voices and passes them in; every rule about them is here.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// A voice's quality, as the platform grades it; a better one sorts first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
pub enum VoiceQuality {
    /// The voice the system ships.
    Default,
    /// A larger download.
    Enhanced,
    /// The largest download, and the most natural voice.
    Premium,
}

/// One installed voice, as the platform lists it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Voice {
    /// The platform's identifier for the voice, which a choice records.
    pub identifier: String,
    /// The voice's name, which a picker shows.
    pub name: String,
    /// The voice's BCP 47 language (`en-US`).
    pub language: String,
    /// The voice's quality.
    pub quality: VoiceQuality,
}

/// Why a choice was not recorded.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum VoiceChoiceRefusal {
    /// The file could not be written whole or renamed into place; the choice it held stands.
    NotWritten {
        /// The file system's reason.
        reason: String,
    },
}

impl fmt::Display for VoiceChoiceRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotWritten { reason } => write!(f, "the voice choice was not written: {reason}"),
        }
    }
}

impl std::error::Error for VoiceChoiceRefusal {}

/// The voice chosen for each language, kept in one file.
#[derive(uniffi::Object)]
pub struct VoiceChoices {
    path: PathBuf,
    choices: Mutex<BTreeMap<String, String>>,
}

/// One line's language and identifier, or `None` when the line does not parse: no tab, an empty
/// side, or a second tab.
fn parse(line: &str) -> Option<(&str, &str)> {
    let (language, identifier) = line.split_once('\t')?;
    let parsed = !language.is_empty() && !identifier.is_empty() && !identifier.contains('\t');
    parsed.then_some((language, identifier))
}

/// The choices the file at `path` holds; none when it is absent or unreadable.
fn read(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let mut choices = BTreeMap::new();
    for line in text.lines() {
        let Some((language, identifier)) = parse(line) else {
            continue;
        };
        choices.insert(language.to_owned(), identifier.to_owned());
    }
    choices
}

/// A language's primary subtag: `en` for `en-US`.
fn primary_subtag(language: &str) -> &str {
    language.split('-').next().unwrap_or(language)
}

/// Whether the requested `entry` names `voice`: by its identifier, or by its name with each space
/// written `_`, the entry compared whole or after its first `_` (SPEC-393 R8; ADR-407 D3).
fn names(entry: &str, voice: &Voice) -> bool {
    let name = voice.name.replace(' ', "_");
    let unprefixed = entry.split_once('_').map(|(_, rest)| rest);
    [Some(entry), unprefixed]
        .into_iter()
        .flatten()
        .any(|candidate| candidate == voice.identifier || candidate == name)
}

#[uniffi::export]
impl VoiceChoices {
    /// Opens the choices kept in the file at `path`. An absent file holds none, and a line that
    /// does not parse is skipped.
    #[uniffi::constructor]
    #[must_use]
    pub fn open(path: String) -> Arc<Self> {
        let path = PathBuf::from(path);
        let choices = Mutex::new(read(&path));
        Arc::new(Self { path, choices })
    }

    /// The identifier chosen for `language`, when that voice is among `installed`; else nothing,
    /// so a voice the system removed falls back to the language's own.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn chosen(&self, language: String, installed: Vec<Voice>) -> Option<String> {
        let identifier = self
            .choices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&language)
            .cloned()?;
        let present = installed.iter().any(|voice| voice.identifier == identifier);
        present.then_some(identifier)
    }

    /// The identifier that speaks a TTS tag in `language` asking for the `requested` voices: the
    /// kept choice when it is installed; else the first requested entry naming a voice the picker
    /// offers, by its identifier or by its name with each space written `_`, the entry compared
    /// whole or after its first `_`; else nothing, so the language's own voice speaks.
    pub fn voice_for(
        &self,
        language: String,
        requested: Vec<String>,
        installed: Vec<Voice>,
    ) -> Option<String> {
        let kept = self.chosen(language.clone(), installed.clone());
        if kept.is_some() {
            return kept;
        }
        let offered = self.options(language, installed);
        requested.into_iter().find_map(|entry| {
            offered
                .iter()
                .find(|voice| names(&entry, voice))
                .map(|voice| voice.identifier.clone())
        })
    }

    /// The installed voices a picker offers for `language`: those whose language equals it, or,
    /// when none does, those sharing its primary subtag; the better quality first, then by name.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn options(&self, language: String, installed: Vec<Voice>) -> Vec<Voice> {
        let mut options: Vec<Voice> = installed
            .iter()
            .filter(|voice| voice.language == language)
            .cloned()
            .collect();
        if options.is_empty() {
            let primary = primary_subtag(&language);
            options = installed
                .into_iter()
                .filter(|voice| primary_subtag(&voice.language).eq_ignore_ascii_case(primary))
                .collect();
        }
        options.sort_by(|a, b| b.quality.cmp(&a.quality).then_with(|| a.name.cmp(&b.name)));
        options
    }

    /// Records `identifier` as the voice for `language`, or clears the choice when it is nothing,
    /// and writes the file whole.
    ///
    /// # Errors
    ///
    /// [`VoiceChoiceRefusal::NotWritten`] when the file cannot be written or renamed; the
    /// choices then stay as they were.
    pub fn choose(
        &self,
        language: String,
        identifier: Option<String>,
    ) -> Result<(), VoiceChoiceRefusal> {
        let mut choices = self.choices.lock().unwrap_or_else(PoisonError::into_inner);
        let mut next = choices.clone();
        match identifier {
            Some(identifier) => next.insert(language, identifier),
            None => next.remove(&language),
        };
        let mut text = String::new();
        for (language, identifier) in &next {
            text.push_str(language);
            text.push('\t');
            text.push_str(identifier);
            text.push('\n');
        }
        let mut temporary = self.path.clone().into_os_string();
        temporary.push(".tmp");
        std::fs::write(&temporary, text)
            .and_then(|()| std::fs::rename(&temporary, &self.path))
            .map_err(|error| VoiceChoiceRefusal::NotWritten {
                reason: error.to_string(),
            })?;
        *choices = next;
        Ok(())
    }
}
