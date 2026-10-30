//! A media file's name: a certified `ds-` name that carries its file's hash, or a name an import
//! brought, kept as it was written (SPEC-513 R2; ADR-531 D2). STUB: every function keeps its shape
//! and answers wrongly, so the crate's tests are seen red first.

use std::fmt;

use crate::hash::MediaHash;

/// The prefix of a certified name.
pub const CERTIFIED_PREFIX: &str = "ds-";

/// The longest extension a certified name carries.
pub const EXTENSION_MAX: usize = 10;

/// A certified name's parts: its file's hash and its extension, if any.
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
    /// The extension is not one the certified form admits.
    BadExtension,
    /// A certified name's hash is not the hash of the file's bytes.
    Mismatch,
}

impl MediaName {
    /// Reads `text` as a media name. STUB: refuses every name.
    ///
    /// # Errors
    ///
    /// STUB: [`NameRefusal::NotPlain`] for every name.
    pub fn parse(_text: &str) -> Result<Self, NameRefusal> {
        Err(NameRefusal::NotPlain)
    }

    /// The certified name of the file whose hash is `hash`. STUB: refuses every extension.
    ///
    /// # Errors
    ///
    /// STUB: [`NameRefusal::BadExtension`] always.
    pub fn certified(_hash: MediaHash, _extension: Option<&str>) -> Result<Self, NameRefusal> {
        Err(NameRefusal::BadExtension)
    }

    /// The certified name of `bytes`.
    ///
    /// # Errors
    ///
    /// As [`MediaName::certified`].
    pub fn for_bytes(bytes: &[u8], extension: Option<&str>) -> Result<Self, NameRefusal> {
        Self::certified(MediaHash::of(bytes), extension)
    }

    /// The hash a certified name claims. STUB: none.
    #[must_use]
    pub const fn claimed(&self) -> Option<MediaHash> {
        None
    }

    /// Whether a file whose hash is `hash` may carry this name. STUB: always.
    ///
    /// # Errors
    ///
    /// STUB: none.
    pub fn check(&self, _hash: MediaHash) -> Result<(), NameRefusal> {
        Ok(())
    }

    /// Whether `bytes` may be stored under this name.
    ///
    /// # Errors
    ///
    /// As [`MediaName::check`].
    pub fn check_bytes(&self, bytes: &[u8]) -> Result<(), NameRefusal> {
        self.check(MediaHash::of(bytes))
    }
}

impl fmt::Display for MediaName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Certified(name) => {
                f.write_str(CERTIFIED_PREFIX)?;
                f.write_str(&name.hash.to_hex())
            }
            Self::Imported(name) => f.write_str(name),
        }
    }
}

/// Whether `name` is a plain file name. STUB: never.
#[must_use]
pub fn plain_name(_name: &str) -> bool {
    false
}
