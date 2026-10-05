//! The voice choice (SPEC-348 R6; ADR-359 D3), stubbed: it opens no file, chooses nothing, offers
//! nothing and records nothing.

#![allow(
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::unnecessary_wraps,
    reason = "the stub reads none of what it is given"
)]

use std::fmt;
use std::sync::Arc;

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

/// The voice chosen for each language, kept in one file (stubbed).
#[derive(uniffi::Object)]
pub struct VoiceChoices {}

#[uniffi::export]
impl VoiceChoices {
    /// Opens the choices kept in the file at `path` (stubbed: it reads nothing).
    #[uniffi::constructor]
    #[must_use]
    pub fn open(path: String) -> Arc<Self> {
        Arc::new(Self {})
    }

    /// The identifier chosen for `language` (stubbed: nothing).
    #[must_use]
    pub fn chosen(&self, language: String, installed: Vec<Voice>) -> Option<String> {
        None
    }

    /// The installed voices a picker offers for `language` (stubbed: none).
    #[must_use]
    pub fn options(&self, language: String, installed: Vec<Voice>) -> Vec<Voice> {
        Vec::new()
    }

    /// Records `identifier` as the voice for `language` (stubbed: it writes nothing).
    ///
    /// # Errors
    ///
    /// None while stubbed.
    pub fn choose(
        &self,
        language: String,
        identifier: Option<String>,
    ) -> Result<(), VoiceChoiceRefusal> {
        Ok(())
    }
}
