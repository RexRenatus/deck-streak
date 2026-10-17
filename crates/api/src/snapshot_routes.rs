//! The snapshot answer (SPEC-377 R12 to R14; ADR-388 D14).
//!
//! `GET /api/sync/snapshot` answers whether the archive holds a sealed snapshot of the server's
//! collection, and how old the newest is: 200 `{"found": true, "age_seconds": <n>}`,
//! `{"found": false}`, or `{"found": null}` when the service cannot tell. Every answer, a refusal
//! included, carries `Cache-Control: no-store`. Its extractors run in this order, and each refuses
//! before the next reads anything:
//!
//! 1. [`SnapshotOn`], the off arm: while no lister is wired, 200 `{"found": null}`, whoever asks,
//!    never 404, so an upload stays refused and the page can say why;
//! 2. identity's [`OwnerSession`]: 401 `no_session` for no session and for a `link` session;
//! 3. [`ReadSlot`]: 429 `too_many_snapshot_reads` with `Retry-After` past [`READS_PER_MINUTE`] in a
//!    minute of the kernel's clock, in a window of the route's own.
//!
//! A GET changes no state, so the route takes no state-change guard. A listing is kept for
//! [`CACHE_WINDOW_MS`] and one the lister could not make is never kept. `found` is said only of a
//! stamp the archive lists both sealed halves of, the archive and its manifest, and the age is the
//! newest such stamp's. No answer and no line this route logs names an object, a path or a prefix:
//! an answer is logged by its word alone, and a refusal by its reason code.

use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::map_response;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{StudyDay, UtcMillis};

use crate::session_routes::OwnerAccess;

/// The answer's path.
pub const SNAPSHOT_PATH: &str = "/api/sync/snapshot";
/// Answers read in one minute of the kernel's clock, per process.
pub const READS_PER_MINUTE: u32 = 30;
/// How long a listing is kept, in milliseconds of the kernel's clock: a burst of reads lists the
/// archive once, and an answer is never older than this.
pub const CACHE_WINDOW_MS: i64 = 60_000;
/// A minute of the kernel's clock, in milliseconds.
const MINUTE_MS: i64 = 60_000;
/// A day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// The suffixes of one snapshot's two sealed halves: the archive, then its manifest.
const SEALED: [&str; 2] = [".tar.age", ".sha256.age"];

/// The listing kept: when the lister last listed, in milliseconds of the kernel's clock, and the
/// newest sealed stamp it held.
type Kept = Option<(i64, Option<i64>)>;

/// The archive's listing as the service reads it. The daemon implements it over the list command.
pub trait SnapshotLister: Send + Sync {
    /// The names the archive lists, or `None` when it could not list them.
    fn list(&self) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + '_>>;
}

/// What the route reads: the owner's access, for its sessions and its clock, the lister when the
/// deployment wires one, the route's own window, and the last listing kept. A handle: every clone
/// shares the window and the listing.
#[derive(Clone)]
pub(crate) struct SnapshotRead {
    access: OwnerAccess,
    lister: Option<Arc<dyn SnapshotLister>>,
    bound: Arc<ReadBound>,
    /// The last listing the lister made.
    cache: Arc<Mutex<Kept>>,
}

impl FromRef<SnapshotRead> for Sessions {
    fn from_ref(read: &SnapshotRead) -> Self {
        Self::from_ref(&read.access)
    }
}

/// The snapshot route over `access`, with `lister` when the deployment wires one.
pub(crate) fn routes(access: OwnerAccess, lister: Option<Arc<dyn SnapshotLister>>) -> Router {
    Router::new()
        .route(SNAPSHOT_PATH, get(snapshot))
        .route_layer(map_response(no_store))
        .with_state(SnapshotRead {
            access,
            lister,
            bound: Arc::default(),
            cache: Arc::default(),
        })
}

/// `GET /api/sync/snapshot`: whether a sealed snapshot exists, and the newest one's age, to the
/// owner's live session.
async fn snapshot(
    SnapshotOn(lister): SnapshotOn,
    _owner: OwnerSession,
    _slot: ReadSlot,
    State(read): State<SnapshotRead>,
) -> Response {
    let now = read.access.clock().now().epoch_millis();
    let Some(newest) = listed(&read, lister.as_ref(), now).await else {
        return unknown();
    };
    let Some(stamp) = newest else {
        tracing::info!(answer = "not_found", "the snapshot was answered");
        return answered(&serde_json::json!({ "found": false }));
    };
    tracing::info!(answer = "found", "the snapshot was answered");
    // A stamp past the clock is no age below zero.
    let age = u64::try_from((now - stamp) / 1000).unwrap_or(0);
    answered(&serde_json::json!({ "found": true, "age_seconds": age }))
}

/// The newest sealed stamp of the listing kept, or of one the lister makes now when the kept one is
/// stale: `Some(None)` when the archive holds no sealed snapshot, and `None` when the lister could
/// not list it, which is not kept.
async fn listed(read: &SnapshotRead, lister: &dyn SnapshotLister, now: i64) -> Option<Option<i64>> {
    let kept = *read.cache.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((at, newest)) = kept {
        let stale = now - at >= CACHE_WINDOW_MS;
        if !stale {
            return Some(newest);
        }
    }
    let names = lister.list().await?;
    let newest = newest_sealed(&names);
    *read.cache.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, newest));
    Some(newest)
}

/// The newest stamp, in milliseconds since the epoch, for which `names` lists both the sealed
/// archive and its sealed manifest.
fn newest_sealed(names: &[String]) -> Option<i64> {
    names
        .iter()
        .filter_map(|name| sealed_stamp(name, SEALED[0]))
        .filter_map(|stamp| {
            let manifest = names
                .iter()
                .any(|other| sealed_stamp(other, SEALED[1]) == Some(stamp));
            manifest.then(|| stamp_millis(stamp)).flatten()
        })
        .max()
}

/// The stamp of `name` when its last segment is `sync-<stamp><suffix>` and the stamp has the
/// archive's form; any other name is none.
fn sealed_stamp<'a>(name: &'a str, suffix: &str) -> Option<&'a str> {
    let file = name.rsplit_once('/').map_or(name, |(_, file)| file);
    let stamp = file.strip_prefix("sync-")?.strip_suffix(suffix)?;
    formed(stamp).then_some(stamp)
}

/// Whether `stamp` has the archive's form, `YYYYMMDDTHHMMSSZ`: sixteen characters, `T` and `Z` at
/// their places and a digit at every other.
fn formed(stamp: &str) -> bool {
    stamp.len() == 16
        && stamp.bytes().enumerate().all(|(at, byte)| match at {
            8 => byte == b'T',
            15 => byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

/// The instant `stamp` names, in milliseconds since the epoch, read by position: a date the
/// calendar has, and a time of day inside it; anything else is none.
fn stamp_millis(stamp: &str) -> Option<i64> {
    let part = |from: usize, to: usize| stamp.get(from..to);
    let date = [part(0, 4)?, part(4, 6)?, part(6, 8)?].join("-");
    let day = StudyDay::from_str(&date).ok()?;
    let number = |from: usize, to: usize| part(from, to)?.parse::<i64>().ok();
    let (hours, minutes, seconds) = (number(9, 11)?, number(11, 13)?, number(13, 15)?);
    if !(hours < 24 && minutes < 60 && seconds < 60) {
        return None;
    }
    Some(day.epoch_day() * DAY_MS + ((hours * 60 + minutes) * 60 + seconds) * 1000)
}

/// The lister, when the deployment wires one. It is the route's FIRST extractor, so a service with
/// none answers unknown before anything else is judged: the route is off, never missing.
struct SnapshotOn(Arc<dyn SnapshotLister>);

impl FromRequestParts<SnapshotRead> for SnapshotOn {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        read: &SnapshotRead,
    ) -> Result<Self, Self::Rejection> {
        match &read.lister {
            Some(lister) => Ok(Self(Arc::clone(lister))),
            None => Err(unknown()),
        }
    }
}

/// A read the window admitted this minute; past [`READS_PER_MINUTE`], 429 until the minute turns,
/// with a `Retry-After` of the whole seconds left.
struct ReadSlot;

impl FromRequestParts<SnapshotRead> for ReadSlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        read: &SnapshotRead,
    ) -> Result<Self, Self::Rejection> {
        match read.bound.admit(read.access.clock().now()) {
            Ok(()) => Ok(Self),
            Err(seconds) => {
                let mut response =
                    refused(StatusCode::TOO_MANY_REQUESTS, "too_many_snapshot_reads");
                response.headers_mut().insert(RETRY_AFTER, seconds.into());
                Err(response)
            }
        }
    }
}

/// The reads counted in the current minute of the kernel's clock.
#[derive(Debug, Default)]
struct ReadBound {
    /// The minute, as whole minutes since the epoch, and the reads admitted in it.
    window: Mutex<(i64, u32)>,
}

impl ReadBound {
    /// Admits one more read at `now`, or answers how many whole seconds remain until the minute
    /// turns.
    fn admit(&self, now: UtcMillis) -> Result<(), u64> {
        let millis = now.epoch_millis();
        let minute = millis.div_euclid(MINUTE_MS);
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.0 != minute {
            *window = (minute, 0);
        }
        if window.1 >= READS_PER_MINUTE {
            // Between 1 and 60 000 milliseconds are left, so between 1 and 60 whole seconds.
            let left = MINUTE_MS - millis.rem_euclid(MINUTE_MS);
            return Err(u64::try_from(left).map_or(60, |left| left.div_ceil(1000)));
        }
        window.1 += 1;
        Ok(())
    }
}

/// The answer when the service cannot tell: no lister is wired, or the lister could not list.
fn unknown() -> Response {
    tracing::info!(answer = "unknown", "the snapshot was answered");
    answered(&serde_json::json!({ "found": null }))
}

/// A 200 answer with `value` as its JSON body.
fn answered(value: &serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
        .into_response()
}

/// A refusal of the route's own: its status, and a JSON body naming its reason code alone. It is
/// logged by the reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a snapshot read was refused");
    let body = serde_json::json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// Every answer of the route, a refusal included, is kept by no cache: it says what the archive
/// held a moment ago, for one owner.
fn no_store(mut response: Response) -> Ready<Response> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    ready(response)
}
