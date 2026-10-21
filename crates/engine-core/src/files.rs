//! Where a client keeps its collection's files is the adapter's (SPEC-377 R4; ADR-388 D7): the core
//! asks a [`Files`] port whether a path holds a file and whether two paths name one file. The
//! native default answers as the standard library does. On `wasm32` the standard library answers
//! "absent" and "no other spelling" without failing, so there the default refuses every choice
//! path as the open collection until an adapter installs its own port.

use std::path::Path;
use std::sync::Arc;

/// What the core asks of the place a client keeps its files.
pub trait Files: Send + Sync {
    /// Whether `path` holds a file.
    fn holds(&self, path: &Path) -> bool;
    /// Whether `open` and `path` name one file, however each is spelled.
    fn same(&self, open: &Path, path: &Path) -> bool;
}

/// The standard library's answers: a native client's files.
#[derive(Debug, Clone, Copy, Default)]
pub struct Standard;

impl Files for Standard {
    fn holds(&self, path: &Path) -> bool {
        path.exists()
    }

    fn same(&self, open: &Path, path: &Path) -> bool {
        open == path
            || matches!(
                (open.canonicalize(), path.canonicalize()),
                (Ok(open), Ok(path)) if open == path
            )
    }
}

/// The answers of a client that installed no port where the standard library cannot answer: every
/// path holds a file and names the open collection, so every choice path is refused.
#[derive(Debug, Clone, Copy, Default)]
pub struct FailClosed;

impl Files for FailClosed {
    fn holds(&self, _path: &Path) -> bool {
        true
    }

    fn same(&self, _open: &Path, _path: &Path) -> bool {
        true
    }
}

/// The port a dispatcher starts with on `wasm32`: the fail-closed one.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn target_default() -> Arc<dyn Files> {
    Arc::new(FailClosed)
}

/// The port a dispatcher starts with natively: the standard library's.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn target_default() -> Arc<dyn Files> {
    Arc::new(Standard)
}
