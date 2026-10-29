//! The owner's scheduling companion route (SPEC-511 R1 to R4; ADR-529 D1).
//!
//! Three literal routes serve the owner's companion files from the directory
//! [`COMPANION_DIR`] names: [`MANIFEST_PATH`], [`MODULE_PATH`] and [`WEIGHTS_PATH`]. No route takes
//! a path parameter, so no part of a request reaches a file system path, and any other path under
//! `/api/companion/` answers the shell's 404.
//!
//! STUB: the shapes only. Every route is missing and every setting reads off.

use std::path::{Path, PathBuf};

use axum::Router;
use deck_streak_kernel::{Environment, Setting};

use crate::session_routes::OwnerAccess;
use crate::ApiError;

/// The companion's manifest: the two files' SHA-256 digests.
pub const MANIFEST_PATH: &str = "/api/companion/v1/manifest.json";
/// The companion's WebAssembly module.
pub const MODULE_PATH: &str = "/api/companion/v1/module.wasm";
/// The companion's weights.
pub const WEIGHTS_PATH: &str = "/api/companion/v1/weights.bin";
/// The largest file a route serves, in bytes: 16 MiB.
pub const COMPANION_FILE_CAP: u64 = 16 * 1024 * 1024;
/// The setting that names the directory the three files are read from.
pub const COMPANION_DIR: &str = "DECKSTREAK_COMPANION_DIR";

/// The directory the companion's files are read from: an absolute path, by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionDirectory(PathBuf);

impl CompanionDirectory {
    /// The directory at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The directory [`COMPANION_DIR`] names, or `None` when it is unset or blank.
    ///
    /// # Errors
    ///
    /// [`ApiError::Settings`] when it is set and is not an absolute path.
    pub fn from_env(_env: &Environment) -> Result<Option<Self>, ApiError> {
        Ok(None)
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Setting for CompanionDirectory {
    const SHAPE: &'static str = "an absolute directory path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// The three routes over `access`, reading from `directory` when the deployment names one.
pub(crate) fn routes(_access: OwnerAccess, _directory: Option<CompanionDirectory>) -> Router {
    Router::new()
}
