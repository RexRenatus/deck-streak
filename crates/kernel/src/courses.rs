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

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::settings::{Environment, Setting};

/// The setting naming the private courses file.
pub const COURSES_FILE: &str = "DECKSTREAK_COURSES_FILE";
/// The one schema a courses file may carry.
pub const COURSES_SCHEMA: &str = "deckstreak.courses.v1";
/// The CEFR bands a course's units fall into, in their order.
pub const CEFR_BANDS: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];
/// The longest course code: a language code or a short subject code.
pub const MAX_CODE_LEN: usize = 8;

/// The shape a courses file must have, named in a refusal in place of any value.
const FILE_SHAPE: &str = "a JSON object of the schema deckstreak.courses.v1 with a list of courses";
/// The shape of one course.
const COURSE_SHAPE: &str = "courses each with a code of 1 to 8 lowercase letters, digits or \
    hyphens, a name, a flag, a deck root, a one-letter alias, a writing flag and unit bands";
/// The shape of one focus subject.
const SUBJECT_SHAPE: &str = "focus subjects each with a code, a name and a one-letter alias";
/// The shape of a course's unit bands.
const BANDS_SHAPE: &str = "unit bands keyed A1 to C2, each a pair of unit numbers";
/// FNV-1a's 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a's 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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
        let file: Value = serde_json::from_str(text).map_err(|_| malformed(FILE_SHAPE))?;
        let file = file.as_object().ok_or_else(|| malformed(FILE_SHAPE))?;
        if file.get("schema").and_then(Value::as_str) != Some(COURSES_SCHEMA) {
            return Err(malformed(FILE_SHAPE));
        }
        let courses = file
            .get("courses")
            .and_then(Value::as_array)
            .ok_or_else(|| malformed(FILE_SHAPE))?
            .iter()
            .map(course)
            .collect::<Result<Vec<_>, _>>()?;
        let focus_subjects = match file.get("focus_subjects") {
            None | Some(Value::Null) => Vec::new(),
            Some(subjects) => subjects
                .as_array()
                .ok_or_else(|| malformed(SUBJECT_SHAPE))?
                .iter()
                .map(focus_subject)
                .collect::<Result<Vec<_>, _>>()?,
        };
        let codes = courses
            .iter()
            .map(|course| course.code)
            .chain(focus_subjects.iter().map(|subject| subject.code));
        refuse_duplicates("code", codes)?;
        let aliases = courses
            .iter()
            .map(|course| course.alias)
            .chain(focus_subjects.iter().map(|subject| subject.alias));
        refuse_duplicates("alias", aliases)?;
        refuse_duplicates("deck root", courses.iter().map(|course| &course.deck_root))?;
        for course in &courses {
            check_bands(&course.unit_bands)?;
        }
        let digest = (!courses.is_empty() || !focus_subjects.is_empty())
            .then(|| content_digest(canonical(&courses, &focus_subjects).as_bytes()));
        Ok(Self {
            courses,
            focus_subjects,
            digest,
        })
    }

    /// The courses of the file the environment's [`COURSES_FILE`] names, read once at start; none,
    /// said once in the log, when the setting is unset.
    ///
    /// # Errors
    ///
    /// [`CoursesError::Unreadable`] when the file cannot be read, and every refusal of
    /// [`Courses::parse`].
    pub fn load(env: &Environment) -> Result<Self, CoursesError> {
        let path = env
            .optional::<CoursesPath>(COURSES_FILE)
            .map_err(|_| malformed(CoursesPath::SHAPE))?;
        let Some(CoursesPath(path)) = path else {
            tracing::info!(
                setting = COURSES_FILE,
                "no courses file is configured: no card has a course"
            );
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(path).map_err(|_| CoursesError::Unreadable {
            setting: COURSES_FILE,
        })?;
        Self::parse(&text)
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
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// The refusal of a file that does not have `expected`'s shape.
const fn malformed(expected: &'static str) -> CoursesError {
    CoursesError::Malformed {
        setting: COURSES_FILE,
        expected,
    }
}

/// The text field `key` of `object`: present and not blank.
fn text(
    object: &Map<String, Value>,
    key: &str,
    shape: &'static str,
) -> Result<String, CoursesError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| malformed(shape))
}

/// The code field of `object`.
fn code(object: &Map<String, Value>, shape: &'static str) -> Result<CourseCode, CoursesError> {
    object
        .get("code")
        .and_then(Value::as_str)
        .and_then(CourseCode::new)
        .ok_or_else(|| malformed(shape))
}

/// The alias field of `object`: exactly one letter.
fn alias(object: &Map<String, Value>, shape: &'static str) -> Result<char, CoursesError> {
    let alias = object
        .get("alias")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(shape))?;
    let mut letters = alias.chars();
    match (letters.next(), letters.next()) {
        (Some(letter), None) if letter.is_alphabetic() => Ok(letter),
        _ => Err(malformed(shape)),
    }
}

/// One course of the file.
fn course(value: &Value) -> Result<Course, CoursesError> {
    let object = value.as_object().ok_or_else(|| malformed(COURSE_SHAPE))?;
    let writing = object
        .get("writing")
        .and_then(Value::as_bool)
        .ok_or_else(|| malformed(COURSE_SHAPE))?;
    Ok(Course {
        code: code(object, COURSE_SHAPE)?,
        name: text(object, "name", COURSE_SHAPE)?,
        flag: text(object, "flag", COURSE_SHAPE)?,
        deck_root: text(object, "deck_root", COURSE_SHAPE)?,
        alias: alias(object, COURSE_SHAPE)?,
        writing,
        unit_bands: unit_bands(object.get("unit_bands"))?,
    })
}

/// One focus subject of the file.
fn focus_subject(value: &Value) -> Result<FocusSubject, CoursesError> {
    let object = value.as_object().ok_or_else(|| malformed(SUBJECT_SHAPE))?;
    Ok(FocusSubject {
        code: code(object, SUBJECT_SHAPE)?,
        name: text(object, "name", SUBJECT_SHAPE)?,
        alias: alias(object, SUBJECT_SHAPE)?,
    })
}

/// A course's unit bands, in the bands' order: each named once, from A1 to C2, as a pair of unit
/// numbers.
fn unit_bands(value: Option<&Value>) -> Result<Vec<UnitBand>, CoursesError> {
    let bands = value
        .and_then(Value::as_object)
        .ok_or_else(|| malformed(BANDS_SHAPE))?;
    let mut read = Vec::with_capacity(bands.len());
    for (name, range) in bands {
        let band = CEFR_BANDS
            .iter()
            .copied()
            .find(|band| band == name)
            .ok_or_else(|| malformed(BANDS_SHAPE))?;
        let unit = |index: usize| {
            range
                .get(index)
                .and_then(Value::as_u64)
                .and_then(|unit| u32::try_from(unit).ok())
        };
        let pair = range.as_array().map_or(0, Vec::len);
        let (Some(first), Some(last), 2) = (unit(0), unit(1), pair) else {
            return Err(malformed(BANDS_SHAPE));
        };
        read.push(UnitBand { band, first, last });
    }
    read.sort_by_key(|band| CEFR_BANDS.iter().position(|name| *name == band.band));
    Ok(read)
}

/// Refuses bands that run backwards, overlap or come out of the A1 to C2 order.
fn check_bands(bands: &[UnitBand]) -> Result<(), CoursesError> {
    let fault = |fault| CoursesError::Bands {
        setting: COURSES_FILE,
        fault,
    };
    if bands.iter().any(|band| band.first > band.last) {
        return Err(fault("run backwards"));
    }
    for pair in bands.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        if after.first <= before.last {
            return Err(if after.last < before.first {
                fault("come out of order")
            } else {
                fault("overlap")
            });
        }
    }
    Ok(())
}

/// Refuses two entries that share a `field`.
fn refuse_duplicates<T: Ord>(
    field: &'static str,
    values: impl Iterator<Item = T>,
) -> Result<(), CoursesError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(CoursesError::Duplicate {
                setting: COURSES_FILE,
                field,
            });
        }
    }
    Ok(())
}

/// The courses in one fixed text, whatever the file's layout or key order: what the digest reads.
fn canonical(courses: &[Course], focus_subjects: &[FocusSubject]) -> String {
    let courses: Vec<Value> = courses
        .iter()
        .map(|course| {
            let bands: Vec<Value> = course
                .unit_bands
                .iter()
                .map(|band| json!([band.band, band.first, band.last]))
                .collect();
            json!([
                course.code.as_str(),
                course.name,
                course.flag,
                course.deck_root,
                course.alias.to_string(),
                course.writing,
                bands,
            ])
        })
        .collect();
    let subjects: Vec<Value> = focus_subjects
        .iter()
        .map(|subject| {
            json!([
                subject.code.as_str(),
                subject.name,
                subject.alias.to_string()
            ])
        })
        .collect();
    json!([COURSES_SCHEMA, courses, subjects]).to_string()
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
