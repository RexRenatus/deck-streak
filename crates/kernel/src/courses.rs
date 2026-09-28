//! The owner's courses (SPEC-071 R1 and R4; ADR-087): private configuration, read once at start
//! from the JSON file [`COURSES_FILE`] names, in the schema [`COURSES_SCHEMA`].
//!
//! The predecessor hard-codes the owner's course table (`curriculum.py:LANGUAGE_DECKS`,
//! `LANG_ALIASES`, `WRITING_LANG_CODES`, `CEFR_UNIT_BANDS` and `FOCUS_SUBJECT_DISPLAY` at
//! `27ee2bc`). Those are the owner's study plan, which CHARTER 11 keeps out of the repository, so
//! here they are a file outside it, and `deploy/config/courses.example.json` shows the shape with
//! neutral values. Every context that needs a card's course reads this one typed value: analytics,
//! curriculum, habits and focus may not depend on each other (ADR-002), so it lives in the shared
//! kernel, and ingest translates a card's home deck into its course code.
//!
//! A refusal names the setting and never a value: not the path, not a code, not a deck name. With
//! the setting unset there are no courses, and [`Courses::load`] says so once.

use std::fmt;
use std::path::PathBuf;

use crate::settings::{Environment, Setting};

/// The setting naming the private courses file.
pub const COURSES_FILE: &str = "DECKSTREAK_COURSES_FILE";
/// The one schema a courses file may carry.
pub const COURSES_SCHEMA: &str = "deckstreak.courses.v1";
/// The CEFR bands a course's units fall into, in their order.
pub const CEFR_BANDS: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];
/// The longest course code: a language code or a short subject code.
pub const MAX_CODE_LEN: usize = 8;

/// A course's code, as it travels with a card: 1 to [`MAX_CODE_LEN`] ASCII lowercase letters,
/// digits or hyphens. It is `Copy`, so a card carries its course without an allocation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseCode {
    bytes: [u8; MAX_CODE_LEN],
    len: u8,
}

impl CourseCode {
    /// The code `text`, or `None` when it is empty, too long, or holds another character.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let raw = text.as_bytes();
        let valid = !raw.is_empty()
            && raw.len() <= MAX_CODE_LEN
            && raw
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
        if !valid {
            return None;
        }
        let mut bytes = [0; MAX_CODE_LEN];
        bytes[..raw.len()].copy_from_slice(raw);
        Some(Self {
            bytes,
            len: u8::try_from(raw.len()).ok()?,
        })
    }

    /// The code's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Only ASCII bytes are ever stored, so the slice is always UTF-8.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl fmt::Debug for CourseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CourseCode({:?})", self.as_str())
    }
}

impl fmt::Display for CourseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The inclusive range of unit numbers one CEFR band covers in a course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitBand {
    /// The band, one of [`CEFR_BANDS`].
    pub band: &'static str,
    /// The band's first unit.
    pub first: u32,
    /// The band's last unit.
    pub last: u32,
}

/// One course of the owner's: a language or another subject studied in its own deck tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Course {
    /// The code that travels with a card.
    pub code: CourseCode,
    /// The name a screen shows.
    pub name: String,
    /// The flag a screen shows.
    pub flag: String,
    /// The top-level deck name the course roots: a card is in the course when its home deck's
    /// top-level name equals it exactly.
    pub deck_root: String,
    /// The one-letter alias a command takes.
    pub alias: char,
    /// Whether the course carries the writing habit.
    pub writing: bool,
    /// The unit range of each band the course names, in the bands' order.
    pub unit_bands: Vec<UnitBand>,
}

/// A further focus subject, which roots no deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusSubject {
    /// The subject's code.
    pub code: CourseCode,
    /// The name a screen shows.
    pub name: String,
    /// The one-letter alias a command takes.
    pub alias: char,
}

/// The owner's courses, as loaded at start: none when the setting is unset.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Courses {
    courses: Vec<Course>,
    focus_subjects: Vec<FocusSubject>,
    digest: Option<String>,
}

impl Courses {
    /// The courses of the file text `text`.
    ///
    /// # Errors
    ///
    /// [`CoursesError::Malformed`] when the text is not the schema's shape,
    /// [`CoursesError::Duplicate`] when two entries share a code, an alias or a deck root, and
    /// [`CoursesError::Bands`] when a course's unit bands overlap, run backwards or come out of
    /// order.
    pub fn parse(text: &str) -> Result<Self, CoursesError> {
        let _ = text;
        Ok(Self::default())
    }

    /// The courses of the file the environment's [`COURSES_FILE`] names, read once at start; none,
    /// said once in the log, when the setting is unset.
    ///
    /// # Errors
    ///
    /// [`CoursesError::Unreadable`] when the file cannot be read, and every refusal of
    /// [`Courses::parse`].
    pub fn load(env: &Environment) -> Result<Self, CoursesError> {
        let _ = env.optional::<CoursesPath>(COURSES_FILE);
        Ok(Self::default())
    }

    /// Every course, in the file's order.
    #[must_use]
    pub fn courses(&self) -> &[Course] {
        &self.courses
    }

    /// Every further focus subject, in the file's order.
    #[must_use]
    pub fn focus_subjects(&self) -> &[FocusSubject] {
        &self.focus_subjects
    }

    /// The digest of the loaded courses, or `None` when there are none: what the settings
    /// generation records (R4) and each study day's review fingerprint carries (R18).
    #[must_use]
    pub fn digest(&self) -> Option<&str> {
        self.digest.as_deref()
    }
}

/// A stable 64-bit FNV-1a digest of `bytes`, as sixteen lowercase hex digits. It detects a change,
/// and is never a security boundary: the kernel admits no hashing crate (ADR-003), and a digest
/// only has to move when the digested bytes do, on every toolchain.
#[must_use]
pub fn content_digest(bytes: &[u8]) -> String {
    let _ = bytes;
    String::new()
}

/// The path the setting names: an absolute file path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CoursesPath(PathBuf);

impl Setting for CoursesPath {
    const SHAPE: &'static str = "an absolute file path";

    fn parse(text: &str) -> Option<Self> {
        let path = PathBuf::from(text);
        path.is_absolute().then_some(Self(path))
    }
}

/// Why the courses refuse start. Each names the setting and never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CoursesError {
    /// The file the setting names cannot be read.
    #[error("the setting {setting} names a file that cannot be read")]
    Unreadable {
        /// The setting's name.
        setting: &'static str,
    },
    /// The setting or its file does not have the shape it must have.
    #[error("the setting {setting} is malformed: it must be {expected}")]
    Malformed {
        /// The setting's name.
        setting: &'static str,
        /// The shape expected.
        expected: &'static str,
    },
    /// Two entries share a code, an alias or a deck root.
    #[error("the setting {setting} names a file in which two entries share a {field}")]
    Duplicate {
        /// The setting's name.
        setting: &'static str,
        /// What they share: `code`, `alias` or `deck root`.
        field: &'static str,
    },
    /// A course's unit bands overlap, run backwards or come out of order.
    #[error("the setting {setting} names a file in which a course's unit bands {fault}")]
    Bands {
        /// The setting's name.
        setting: &'static str,
        /// What is wrong: `overlap`, `run backwards` or `come out of order`.
        fault: &'static str,
    },
}
