//! A media file's name: a certified `ds-` name that carries its file's hash, or a name an import
//! brought, kept as it was written (SPEC-513 R2; ADR-531 D2).
//!
//! A certified name is `ds-`, the 64 lowercase hex digits of its file's [`MediaHash`], and an
//! optional `.` with an extension of 1 to [`EXTENSION_MAX`] lowercase ASCII letters or digits. It
//! is written in one form only, so it reads back to the same name, and a file whose bytes hash to
//! anything else is refused. Every other plain name is an imported one: it claims no hash, and it
//! is never renamed. A name that starts like a certified one, `ds-` and 64 hex digits in any case,
//! but is not written in the certified form is refused, so no name looks certified without being
//! one.

use std::fmt;

use crate::hash::{HEX_LEN, MediaHash};

/// The prefix of a certified name.
pub const CERTIFIED_PREFIX: &str = "ds-";

/// The longest extension a certified name carries.
pub const EXTENSION_MAX: usize = 10;

/// A certified name's parts: its file's hash and its extension, if any. Built only by
/// [`MediaName::certified`] and [`MediaName::parse`], so its extension is always one the form
/// admits.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CertifiedName {
    hash: MediaHash,
    extension: Option<String>,
}

impl CertifiedName {
    /// The hash the name certifies.
    #[must_use]
    pub const fn hash(&self) -> MediaHash {
        self.hash
    }

    /// The name's extension, without its dot.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }
}

/// A media file's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MediaName {
    /// A `ds-` name that carries its file's hash.
    Certified(CertifiedName),
    /// A name an import brought, kept as it was written; it claims no hash.
    Imported(String),
}

/// Why a name, or a file under a name, is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRefusal {
    /// The name is empty, `.` or `..`, or holds a separator or a NUL.
    NotPlain,
    /// The name starts like a certified one but is not written in the certified form.
    Malformed,
    /// The extension is empty, too long, or holds a character other than a lowercase ASCII
    /// letter or digit.
    BadExtension,
    /// A certified name's hash is not the hash of the file's bytes.
    Mismatch,
}

impl fmt::Display for NameRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotPlain => "the name is not a plain file name",
            Self::Malformed => "the name starts like a certified name but is not one",
            Self::BadExtension => "the extension is not 1 to 10 lowercase letters or digits",
            Self::Mismatch => "the name certifies another file's bytes",
        })
    }
}

impl std::error::Error for NameRefusal {}

impl MediaName {
    /// Reads `text` as a media name: certified when it is written in the certified form, imported
    /// when it is any other plain name.
    ///
    /// # Errors
    ///
    /// [`NameRefusal::NotPlain`] for a name that is not a plain file name, and
    /// [`NameRefusal::Malformed`] for one that starts like a certified name but is not one.
    pub fn parse(text: &str) -> Result<Self, NameRefusal> {
        if !plain_name(text) {
            return Err(NameRefusal::NotPlain);
        }
        let Some(rest) = text.strip_prefix(CERTIFIED_PREFIX) else {
            return Ok(Self::Imported(text.to_owned()));
        };
        let looks_certified =
            rest.len() >= HEX_LEN && rest.as_bytes()[..HEX_LEN].iter().all(u8::is_ascii_hexdigit);
        if !looks_certified {
            return Ok(Self::Imported(text.to_owned()));
        }
        let (digits, tail) = rest.split_at(HEX_LEN);
        let hash = MediaHash::from_hex(digits).ok_or(NameRefusal::Malformed)?;
        let extension = if tail.is_empty() {
            None
        } else {
            let extension = tail.strip_prefix('.').ok_or(NameRefusal::Malformed)?;
            if !admitted_extension(extension) {
                return Err(NameRefusal::Malformed);
            }
            Some(extension.to_owned())
        };
        Ok(Self::Certified(CertifiedName { hash, extension }))
    }

    /// The certified name of the file whose hash is `hash`, with `extension` if one is given.
    ///
    /// # Errors
    ///
    /// [`NameRefusal::BadExtension`] for an extension the certified form does not admit.
    pub fn certified(hash: MediaHash, extension: Option<&str>) -> Result<Self, NameRefusal> {
        if let Some(extension) = extension
            && !admitted_extension(extension)
        {
            return Err(NameRefusal::BadExtension);
        }
        Ok(Self::Certified(CertifiedName {
            hash,
            extension: extension.map(str::to_owned),
        }))
    }

    /// The certified name of `bytes`, with `extension` if one is given.
    ///
    /// # Errors
    ///
    /// [`NameRefusal::BadExtension`] for an extension the certified form does not admit.
    pub fn for_bytes(bytes: &[u8], extension: Option<&str>) -> Result<Self, NameRefusal> {
        Self::certified(MediaHash::of(bytes), extension)
    }

    /// The hash a certified name claims; an imported name claims none.
    #[must_use]
    pub const fn claimed(&self) -> Option<MediaHash> {
        match self {
            Self::Certified(name) => Some(name.hash),
            Self::Imported(_) => None,
        }
    }

    /// Whether a file whose hash is `hash` may carry this name: always for an imported name, and
    /// for a certified one only when the name certifies that hash.
    ///
    /// # Errors
    ///
    /// [`NameRefusal::Mismatch`] when a certified name certifies another hash.
    pub fn check(&self, hash: MediaHash) -> Result<(), NameRefusal> {
        match self.claimed() {
            Some(claimed) if claimed != hash => Err(NameRefusal::Mismatch),
            _ => Ok(()),
        }
    }

    /// Whether `bytes` may be stored under this name ([`MediaName::check`] of their hash).
    ///
    /// # Errors
    ///
    /// [`NameRefusal::Mismatch`] when a certified name certifies other bytes.
    pub fn check_bytes(&self, bytes: &[u8]) -> Result<(), NameRefusal> {
        self.check(MediaHash::of(bytes))
    }
}

impl fmt::Display for MediaName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Certified(name) => {
                f.write_str(CERTIFIED_PREFIX)?;
                f.write_str(&name.hash.to_hex())?;
                if let Some(extension) = &name.extension {
                    f.write_str(".")?;
                    f.write_str(extension)?;
                }
                Ok(())
            }
            Self::Imported(name) => f.write_str(name),
        }
    }
}

/// Whether `name` is a plain file name: not empty, not `.` or `..`, and with no `/`, `\` or NUL.
#[must_use]
pub fn plain_name(name: &str) -> bool {
    let separator = name.contains(['/', '\\', '\0']);
    let dots = name == "." || name == "..";
    !(name.is_empty() || separator || dots)
}

/// Whether `extension` is 1 to [`EXTENSION_MAX`] lowercase ASCII letters or digits.
fn admitted_extension(extension: &str) -> bool {
    (1..=EXTENSION_MAX).contains(&extension.len())
        && extension
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
