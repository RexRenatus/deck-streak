//! Credentials: a secret is read only as `<credentials directory>/<credential id>`, from the
//! directory systemd passes as `$CREDENTIALS_DIRECTORY` (SPEC-020 R11, R12; ADR-010).
//!
//! No production code reads a secret from an environment variable, an argument or a row: the
//! environment is readable from `/proc` for the life of the process (systemd.exec(5)). A missing
//! credential refuses start by its id, one trailing newline is trimmed, an empty credential (no
//! bytes, or only that newline) refuses start by its id too (SPEC-066 R1), and every credential of
//! at least [`crate::redact::MIN_SECRET_LEN`] characters is registered with the [`Redactor`]
//! before the loader returns it, so no log line written after that can carry it.

use std::fmt;
use std::fs;
use std::io;

use crate::error::CredentialError;
use crate::redact::Redactor;
use crate::settings::CredentialsDirectory;

/// A secret read from a credential. Its `Debug` never shows the value.
pub struct Secret(String);

impl Secret {
    /// The value, for the one call that needs it (an authorization header, a sync login).
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(..)")
    }
}

/// Reads credentials from the credentials directory, registering each with the redactor.
#[derive(Debug)]
pub struct CredentialLoader {
    directory: CredentialsDirectory,
    redactor: Redactor,
}

impl CredentialLoader {
    /// A loader over `directory` that registers every secret it reads with `redactor`.
    #[must_use]
    pub const fn new(directory: CredentialsDirectory, redactor: Redactor) -> Self {
        Self {
            directory,
            redactor,
        }
    }

    /// The credential `id`: the file `id` in the credentials directory, less one trailing newline.
    ///
    /// # Errors
    ///
    /// [`CredentialError::Missing`] when the directory holds no such file,
    /// [`CredentialError::Empty`] when the file holds no value once its one trailing newline is
    /// trimmed, [`CredentialError::Unreadable`] when it cannot be read, [`CredentialError::NotText`]
    /// when it is not UTF-8, and [`CredentialError::InvalidId`] when `id` is not a plain file name.
    pub fn load(&self, id: &'static str) -> Result<Secret, CredentialError> {
        if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\0']) {
            return Err(CredentialError::InvalidId { id });
        }
        let bytes = match fs::read(self.directory.path().join(id)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(CredentialError::Missing { id });
            }
            Err(source) => return Err(CredentialError::Unreadable { id, source }),
        };
        let mut value = String::from_utf8(bytes).map_err(|_| CredentialError::NotText { id })?;
        if value.ends_with('\n') {
            value.pop();
        }
        // Nothing left: refused by its id as a missing credential is, before anything is
        // registered. The service manager's manual promises no start failure for a credential
        // that arrives empty, so the loader is where every role refuses one (ADR-067).
        if value.is_empty() {
            return Err(CredentialError::Empty { id });
        }
        // Registered before it is returned, so no line written after this can carry it.
        self.redactor.register(&value);
        Ok(Secret(value))
    }
}
