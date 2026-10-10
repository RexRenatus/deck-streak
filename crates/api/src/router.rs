//! The API's routes and the one stack of layers every route is served under (SPEC-025 R3 to R5;
//! ADR-025).
//!
//! The stack is applied with `Router::layer`, so it runs after routing and the trace can name the
//! MATCHED route rather than the raw path; axum then applies each layer to every route, and to the
//! fallback's 404, separately. That is why the concurrency bound is tower's
//! `GlobalConcurrencyLimitLayer`, whose one semaphore every route shares, rather than
//! `ConcurrencyLimitLayer`, which would give each route a bound of its own (SPEC-025 §7).
//!
//! Outermost first:
//!
//! 1. an `x-request-id` is set (`MakeRequestUuid`) unless the request carries one;
//! 2. `authorization`, `cookie` and `set-cookie` are marked sensitive on the request, before the
//!    trace can read them;
//! 3. the trace: an INFO span with the method, the matched route and the request id, and an INFO
//!    response event with the status and the latency. The span never records the URI, whose query
//!    string may carry `initData`;
//! 4. the same headers are marked sensitive on the response, before the trace sees it;
//! 5. the request id is copied to the response, outside every layer below, so a caught panic's
//!    500, a timeout's 408 and a shed 503 carry it too;
//! 6. a panic is caught and answered 500, and the service keeps serving;
//! 7. a request that has not answered in [`REQUEST_TIMEOUT`] is answered 408;
//! 8. a request past [`MAX_IN_FLIGHT`] in flight is shed with 503 at once, never queued
//!    (ADR-025);
//! 9. the extractors read a body of at most [`BODY_LIMIT_BYTES`], and answer 413 past it.

use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::{HeaderName, StatusCode};
use axum::{BoxError, Router};
use deck_streak_coordination::drills::{DrillNotes, RealFs};
use deck_streak_coordination::inbox_capture::InboxCaptures;
use deck_streak_coordination::instruments::InstrumentService;
use deck_streak_coordination::progression::level_view::LawTierSource;
use deck_streak_identity::sync_seal::SealSecret;
use deck_streak_identity::{LinkingConfig, Owner};
use deck_streak_kernel::Courses;
use tower::ServiceBuilder;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower::load_shed::LoadShedLayer;
use tower::load_shed::error::Overloaded;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::analytics_routes;
use crate::badges_routes;
use crate::drill_routes;
use crate::health::{self, Readiness};
use crate::inbox_capture_route;
use crate::insights_routes;
use crate::law_routes;
use crate::linking_routes;
use crate::minimum_client;
use crate::notifications_routes;
use crate::progress_routes;
use crate::sensitive_decks_routes;
use crate::session_routes::{self, OwnerAccess};
use crate::snapshot_routes::{self, SnapshotLister};
use crate::streak_routes;
use crate::sync_seal_routes;
use crate::wallet_routes;
use crate::xp_routes;

/// Requests served at once, the rust-service pack's reference value. Each holds its buffers until
/// it answers, so this bounds the service's memory; one owner's Mini App never reaches it.
pub const MAX_IN_FLIGHT: usize = 64;
/// A request that has not answered by now is answered 408, which also bounds how long a drain on
/// SIGTERM can take (the unit's `TimeoutStopSec=` sits above it).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest request body the extractors read: axum's own default, 2 MiB, stated here as the
/// number it is, so no route can mistake the limit for switched off.
pub const BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;
/// The header the request id travels in.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// What the API's handlers share.
#[derive(Clone)]
pub struct ApiState {
    readiness: Readiness,
    owner: Option<OwnerAccess>,
    instruments: Option<Arc<dyn InstrumentService>>,
    law_tiers: Option<Arc<dyn LawTierSource>>,
    drills: Option<Arc<DrillNotes<RealFs>>>,
    courses: Option<Courses>,
    inbox: Option<Arc<InboxCaptures<RealFs>>>,
    linking: Option<(LinkingConfig, Owner)>,
    seal: Option<Arc<SealSecret>>,
    snapshot: Option<Arc<dyn SnapshotLister>>,
}

impl std::fmt::Debug for ApiState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line = formatter.debug_struct("ApiState");
        line.field("readiness", &self.readiness)
            .field("owner", &self.owner)
            .field("instruments", &self.instruments.is_some())
            .field("law_tiers", &self.law_tiers.is_some())
            .field("drills", &self.drills.is_some())
            .field("courses", &self.courses.is_some())
            .field("inbox", &self.inbox.is_some())
            .field("linking", &self.linking.is_some());
        // The seal port is named only while it is held: off is the release's default (SPEC-363 R5).
        if self.seal.is_some() {
            line.field("seal", &true);
        }
        // The archive's lister is named only while one is wired: off answers unknown (SPEC-377 R14).
        if self.snapshot.is_some() {
            line.field("snapshot", &true);
        }
        line.finish()
    }
}

impl ApiState {
    /// The state over `readiness`, which the daemon's `api` role records the opened database in.
    #[must_use]
    pub const fn new(readiness: Readiness) -> Self {
        Self {
            readiness,
            owner: None,
            instruments: None,
            law_tiers: None,
            drills: None,
            courses: None,
            inbox: None,
            linking: None,
            seal: None,
            snapshot: None,
        }
    }

    /// This state, serving the instruments' routes (SPEC-094) over `service` too.
    #[must_use]
    pub fn with_instruments(mut self, service: Arc<dyn InstrumentService>) -> Self {
        self.instruments = Some(service);
        self
    }

    /// This state, serving the owner's session routes over `access` too (SPEC-024).
    #[must_use]
    pub fn with_owner(mut self, access: OwnerAccess) -> Self {
        self.owner = Some(access);
        self
    }

    /// This state, serving SPEC-359's linking routes for `owner` as `config` turns linking on or
    /// off. The owner travels beside the configuration because the owner's access exposes none.
    #[must_use]
    pub fn with_linking(mut self, config: LinkingConfig, owner: Owner) -> Self {
        self.linking = Some((config, owner));
        self
    }

    /// This state, releasing the web client's sealing key under `secret` (SPEC-363 R5, R6).
    /// Without it the release route answers 404 `sync_seal_off`.
    #[must_use]
    pub fn with_seal(mut self, secret: SealSecret) -> Self {
        self.seal = Some(Arc::new(secret));
        self
    }

    /// This state, answering the snapshot route from `lister`, the archive's listing (SPEC-377
    /// R14). Without it the route answers `{"found": null}`, unknown.
    #[must_use]
    pub fn with_snapshot(mut self, lister: Arc<dyn SnapshotLister>) -> Self {
        self.snapshot = Some(lister);
        self
    }

    /// This state, answering `GET /api/level/law-tiers` from `source` (SPEC-072 R24).
    #[must_use]
    pub fn with_law_tiers(mut self, source: Arc<dyn LawTierSource>) -> Self {
        self.law_tiers = Some(source);
        self
    }

    /// This state, serving the drill routes over the vault's drill notes (SPEC-110). Without them
    /// the routes answer 503 `vault_not_open`.
    #[must_use]
    pub fn with_drills(mut self, notes: Arc<DrillNotes<RealFs>>) -> Self {
        self.drills = Some(notes);
        self
    }

    /// This state, rendering the badge catalog's descriptions from the configured `courses`
    /// (SPEC-073 R16) and keeping Road to C2's progress to them (SPEC-077 R15). Without them the
    /// descriptions take their generic wording and the progress route answers no course.
    #[must_use]
    pub fn with_courses(mut self, courses: Courses) -> Self {
        self.courses = Some(courses);
        self
    }

    /// This state, serving the quick capture route over the vault's inbox (SPEC-118 R10). Without
    /// it the route answers 503 `vault_not_open`.
    #[must_use]
    pub fn with_inbox(mut self, captures: Arc<InboxCaptures<RealFs>>) -> Self {
        self.inbox = Some(captures);
        self
    }

    /// Whether the API can answer from its database.
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

/// The API: every route the Mini App's backend serves, under the layers. The health routes are
/// always served; the owner's session routes, analytics routes and the in-app feed (SPEC-041) are
/// served when the state carries the owner's access, which the daemon's `api` role always gives, since it refuses to start without the
/// owner's credentials. A router built for the health routes alone needs none.
pub fn router(state: ApiState) -> Router {
    let owner = state.owner.clone();
    let readiness = state.readiness.clone();
    let instruments = state.instruments.clone();
    let law_tiers = state.law_tiers.clone();
    let drills = state.drills.clone();
    let courses = state.courses.clone().unwrap_or_default();
    let inbox = state.inbox.clone();
    let linking = state.linking.clone();
    let seal = state.seal.clone();
    let snapshot = state.snapshot.clone();
    let routes = health::routes()
        .merge(minimum_client::routes())
        .with_state(state);
    let routes = match owner {
        Some(access) => {
            let routes = routes
                .merge(analytics_routes::routes(access.clone(), readiness.clone()))
                .merge(xp_routes::routes(
                    access.clone(),
                    readiness.clone(),
                    law_tiers,
                ))
                .merge(streak_routes::routes(access.clone(), readiness.clone()))
                .merge(wallet_routes::routes(access.clone(), readiness.clone()))
                .merge(badges_routes::routes(
                    access.clone(),
                    readiness.clone(),
                    courses.clone(),
                ))
                .merge(progress_routes::routes(
                    access.clone(),
                    readiness.clone(),
                    courses,
                ))
                .merge(law_routes::routes(access.clone(), readiness.clone()))
                .merge(session_routes::routes(access.clone()))
                .merge(sync_seal_routes::routes(access.clone(), seal))
                .merge(snapshot_routes::routes(access.clone(), snapshot))
                .merge(drill_routes::routes(
                    access.clone(),
                    readiness.clone(),
                    drills,
                ))
                .merge(inbox_capture_route::routes(
                    access.clone(),
                    readiness.clone(),
                    inbox,
                ))
                .merge(linking_routes::routes(&access, readiness.clone(), linking))
                .merge(sensitive_decks_routes::routes(
                    access.clone(),
                    readiness.clone(),
                ))
                .merge(notifications_routes::routes(access.clone(), readiness));
            match instruments {
                Some(service) => routes.merge(insights_routes::routes(access, service)),
                None => routes,
            }
        }
        None => routes,
    };
    layered(routes)
}

/// `routes` under the service's layers, as listed in this module's documentation. The production
/// router is built through here, and so are the tests' own routes, so a route is never served
/// without the bounds.
pub fn layered(routes: Router) -> Router {
    routes.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(SetSensitiveRequestHeadersLayer::new(sensitive_headers()))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &Request| {
                        let route = request
                            .extensions()
                            .get::<MatchedPath>()
                            .map(MatchedPath::as_str);
                        let request_id = request
                            .headers()
                            .get(REQUEST_ID_HEADER)
                            .and_then(|value| value.to_str().ok());
                        tracing::info_span!(
                            "request",
                            method = %request.method(),
                            route,
                            request_id,
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .layer(SetSensitiveResponseHeadersLayer::new(sensitive_headers()))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(CatchPanicLayer::new())
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            ))
            .layer(HandleErrorLayer::new(shed))
            .layer(LoadShedLayer::new())
            .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT))
            .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES)),
    )
}

/// The headers whose values never reach a log: the Mini App's `initData` travels as
/// `Authorization`, and a session as a cookie.
fn sensitive_headers() -> [HeaderName; 3] {
    [AUTHORIZATION, COOKIE, SET_COOKIE]
}

/// The answer to a request the bound refused: 503 at once, so the connection is released and the
/// shed is a counted response event (ADR-025). Only the load shed below this layer can fail, so
/// any other error would be a defect, answered 500.
async fn shed(error: BoxError) -> StatusCode {
    if error.is::<Overloaded>() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
