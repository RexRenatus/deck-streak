//! SPEC-119 R6 to R8, R11: the scope set is a set, the grants' `Debug` names its type and never a
//! token, and the guard's service is ready exactly when the service it wraps is.
//!
//! Every token is built from parts at run time.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::convert::Infallible;
use std::fs;
use std::future::{Ready, ready};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use axum::body::Body;
use axum::http::{Request, Response};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, ManualClock, Redactor, UtcMillis,
};
use deck_streak_mcp::{Grants, Guard, GuardLayer, Scope, Scopes};
use tower::{Layer, Service};

/// A service that answers every request and is ready only when `ready` says so.
struct Inner {
    ready: bool,
}

impl Service<Request<Body>> for Inner {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Response<Body>, Infallible>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        if self.ready {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        ready(Ok(Response::new(Body::empty())))
    }
}

/// The grants over a core and a law-track credential, and the two tokens built from parts.
fn grants() -> (Grants, String, String) {
    let core = format!("scopes-core-{}", "c".repeat(30));
    let law = format!("scopes-law-{}", "l".repeat(30));
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, value) in [("mcp-core-token", &core), ("mcp-law-track-token", &law)] {
        fs::write(directory.path().join(id), format!("{value}\n")).expect("a credential");
    }
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    let grants =
        Grants::load(&CredentialLoader::new(path, Redactor::new())).expect("the grants load");
    (grants, core, law)
}

/// The guard's layer over `inner`, polled once for readiness.
fn poll(inner: Inner) -> Poll<Result<(), Infallible>> {
    let (grants, _, _) = grants();
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        1_760_000_000_000,
    )));
    let guard = Arc::new(Guard::new(grants, Arc::<ManualClock>::clone(&clock)));
    let mut service = GuardLayer::new(guard).layer(inner);
    let mut context = Context::from_waker(Waker::noop());
    service.poll_ready(&mut context)
}

#[test]
fn adding_a_scope_already_held_keeps_it_held() {
    let core = Scopes::NONE.with(Scope::Core);
    assert_eq!(core.with(Scope::Core), core, "core added twice");
    assert!(core.with(Scope::Core).holds(Scope::Core));
    let both = core.with(Scope::LawTrack);
    assert_eq!(both.with(Scope::LawTrack), both, "law_track added twice");
    assert!(both.with(Scope::LawTrack).holds(Scope::LawTrack));
    assert_eq!(both.names(), vec!["core", "law_track"]);
}

#[test]
fn the_grants_debug_names_its_type_and_never_a_token() {
    let (grants, core, law) = grants();
    let shown = format!("{grants:?}");
    assert!(shown.contains("Grants"), "the type's name: {shown}");
    assert!(shown.contains("scopes"), "the scopes field: {shown}");
    assert!(!shown.contains(&core), "the core token: {shown}");
    assert!(!shown.contains(&law), "the law-track token: {shown}");
}

#[test]
fn the_guard_service_is_pending_while_its_inner_service_is() {
    assert!(
        poll(Inner { ready: false }).is_pending(),
        "an inner service that is not ready"
    );
    assert!(
        matches!(poll(Inner { ready: true }), Poll::Ready(Ok(()))),
        "an inner service that is ready"
    );
}
