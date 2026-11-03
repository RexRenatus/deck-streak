//! The owner's scheduling companion route (SPEC-511 R1 to R4; ADR-529 D1).
//!
//! Three literal routes serve the owner's companion files from the directory
//! [`COMPANION_DIR`] names: [`MANIFEST_PATH`], [`MODULE_PATH`] and [`WEIGHTS_PATH`]. No route takes
//! a path parameter, so no part of a request reaches a file system path, and any other path under
//! `/api/companion/` answers the shell's 404.
//!
//! Each route's first extractor is identity's [`OwnerSession`]: no session, an unknown session and
//! a `link` session answer 401 `no_session`. With an owner session, a service with no directory, a
//! missing file and a file that is not a regular file answer 404 `companion_absent`, which is what
//! every public deployment answers; a file larger than [`COMPANION_FILE_CAP`] answers 404
//! `companion_over_cap`, and no read takes more than the cap plus one byte. A present file answers
//! 200 with its exact bytes and its content type. Every answer of the three routes, a refusal
//! included, carries `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`. No
//! answer and no line this route logs names a path: a refusal is logged by its reason code alone.

use std::future::{Ready, ready};
use std::path::{Path, PathBuf};

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::map_response;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Environment, Setting};
use tokio::io::AsyncReadExt;

use crate::ApiError;
use crate::session_routes::OwnerAccess;

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

/// Each route's file in the directory and the content type it is served with.
const MANIFEST: (&str, &str) = ("manifest.json", "application/json");
const MODULE: (&str, &str) = ("module.wasm", "application/wasm");
const WEIGHTS: (&str, &str) = ("weights.bin", "application/octet-stream");

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

    /// The directory [`COMPANION_DIR`] names, or `None` when it is unset or blank: the routes then
    /// answer 404 to the owner, as every public deployment does.
    ///
    /// # Errors
    ///
    /// [`ApiError::Settings`] when it is set and is not an absolute path. The refusal names the
    /// setting and never its value.
    pub fn from_env(env: &Environment) -> Result<Option<Self>, ApiError> {
        Ok(env.optional::<Self>(COMPANION_DIR)?)
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

/// What the routes read: the owner's access, for its sessions, and the directory when the
/// deployment names one.
#[derive(Clone)]
pub(crate) struct CompanionRead {
    access: OwnerAccess,
    directory: Option<CompanionDirectory>,
}

impl FromRef<CompanionRead> for Sessions {
    fn from_ref(read: &CompanionRead) -> Self {
        Self::from_ref(&read.access)
    }
}

/// The three routes over `access`, reading from `directory` when the deployment names one.
pub(crate) fn routes(access: OwnerAccess, directory: Option<CompanionDirectory>) -> Router {
    Router::new()
        .route(MANIFEST_PATH, get(manifest))
        .route(MODULE_PATH, get(module))
        .route(WEIGHTS_PATH, get(weights))
        .route_layer(map_response(private_no_store))
        .with_state(CompanionRead { access, directory })
}

/// `GET /api/companion/v1/manifest.json`, to the owner's live session.
async fn manifest(_owner: OwnerSession, State(read): State<CompanionRead>) -> Response {
    served(&read, MANIFEST).await
}

/// `GET /api/companion/v1/module.wasm`, to the owner's live session.
async fn module(_owner: OwnerSession, State(read): State<CompanionRead>) -> Response {
    served(&read, MODULE).await
}

/// `GET /api/companion/v1/weights.bin`, to the owner's live session.
async fn weights(_owner: OwnerSession, State(read): State<CompanionRead>) -> Response {
    served(&read, WEIGHTS).await
}

/// Why a file is not served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unserved {
    /// No directory is named, the file is missing, it is not a regular file, or it did not read.
    Absent,
    /// The file is larger than [`COMPANION_FILE_CAP`].
    OverCap,
}

impl Unserved {
    /// The reason code the answer and the log carry.
    const fn reason(self) -> &'static str {
        match self {
            Self::Absent => "companion_absent",
            Self::OverCap => "companion_over_cap",
        }
    }
}

/// The answer for one route's file: its bytes and its type, or the refusal.
async fn served(read: &CompanionRead, (name, content_type): (&str, &'static str)) -> Response {
    let Some(directory) = &read.directory else {
        return refused(Unserved::Absent);
    };
    match capped(&directory.path().join(name)).await {
        Ok(bytes) => {
            tracing::info!(file = name, "a companion file was served");
            (StatusCode::OK, [(CONTENT_TYPE, content_type)], bytes).into_response()
        }
        Err(why) => refused(why),
    }
}

/// The bytes of the regular file at `path`, read no further than the cap plus one byte, so every
/// answer's buffer is bounded whatever the file grew to after its size was read.
async fn capped(path: &Path) -> Result<Vec<u8>, Unserved> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|_| Unserved::Absent)?;
    if !metadata.is_file() {
        return Err(Unserved::Absent);
    }
    if metadata.len() > COMPANION_FILE_CAP {
        return Err(Unserved::OverCap);
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| Unserved::Absent)?;
    let mut bytes = Vec::new();
    file.take(COMPANION_FILE_CAP + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Unserved::Absent)?;
    if u64::try_from(bytes.len()).map_or(true, |read| read > COMPANION_FILE_CAP) {
        return Err(Unserved::OverCap);
    }
    Ok(bytes)
}

/// A refusal of the route's own: 404, and a JSON body naming its reason code alone. It is logged
/// by the reason code alone.
fn refused(why: Unserved) -> Response {
    let reason = why.reason();
    tracing::warn!(reason, "a companion read was refused");
    let body = serde_json::json!({ "reason": reason }).to_string();
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

/// Every answer of the three routes, a refusal included, is kept by no cache and sniffed by no
/// browser: it is the owner's alone.
fn private_no_store(mut response: Response) -> Ready<Response> {
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    ready(response)
}
