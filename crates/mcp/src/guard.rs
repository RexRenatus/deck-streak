//! The guard: the bearer's parser, the match, the one refusal and the request layer (SPEC-119 R9
//! to R12, R14; ADR-121, ADR-320).

use std::convert::Infallible;
use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{HeaderMap, Request, Response};
use deck_streak_kernel::Clock;
use tower::{Layer, Service};

use crate::grants::{Grants, Scope, Scopes};
use crate::limiter::{Bucket, Limiter, Outcome};

/// The one word every refusal answers (`mcp_auth.py:DENIED_MESSAGE`).
pub const DENIED: &str = "";

/// What a request's matched grant holds, and the bucket its token's failures go to. The layer
/// inserts it into the request's extensions, where a tool reads it.
#[derive(Debug, Clone)]
pub struct Granted {
    scopes: Scopes,
    bucket: Bucket,
}

impl Granted {
    /// The scopes the matched grant holds.
    #[must_use]
    pub const fn scopes(&self) -> Scopes {
        self.scopes
    }
}

/// A refusal: why it was refused, and the bucket its failure went to. Every cause answers the one
/// response.
#[derive(Debug)]
pub struct Refusal {
    outcome: Outcome,
    bucket: Bucket,
}

impl Refusal {
    /// What the limiter decided.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// The bucket the refusal was decided in.
    #[must_use]
    pub const fn bucket(&self) -> &Bucket {
        &self.bucket
    }

    /// The response every refusal answers.
    #[must_use]
    pub fn into_response(self) -> Response<Body> {
        Response::new(Body::empty())
    }
}

/// The grants and the limiter every request and every scope check shares.
pub struct Guard {
    grants: Grants,
    limiter: Limiter,
}

impl Guard {
    /// A guard over `grants` whose limiter reads `clock`.
    #[must_use]
    pub fn new(grants: Grants, clock: Arc<dyn Clock>) -> Self {
        Self {
            grants,
            limiter: Limiter::new(clock),
        }
    }

    /// The limiter, which a test reads and records into directly.
    #[must_use]
    pub const fn limiter(&self) -> &Limiter {
        &self.limiter
    }

    /// Admits a request whose `headers` present a granted token, or refuses it (R9 to R11, R13).
    ///
    /// # Errors
    ///
    /// A [`Refusal`] when no granted token is presented.
    pub fn admit(&self, headers: &HeaderMap) -> Result<Granted, Refusal> {
        let _ = (headers, &self.grants);
        Err(Refusal {
            outcome: Outcome::Denied,
            bucket: Bucket::of(&[]),
        })
    }

    /// Allows `scope` to an admitted request whose grant holds it, or refuses it (R12, R13).
    ///
    /// # Errors
    ///
    /// A [`Refusal`] when the grant does not hold the scope.
    pub fn authorize(&self, granted: &Granted, scope: Scope) -> Result<(), Refusal> {
        let _ = (granted, scope);
        Ok(())
    }
}

/// The request layer: answers a refusal before its inner service is called (R11).
#[derive(Clone)]
pub struct GuardLayer {
    guard: Arc<Guard>,
}

impl GuardLayer {
    /// The layer of `guard`.
    #[must_use]
    pub const fn new(guard: Arc<Guard>) -> Self {
        Self { guard }
    }
}

impl<S> Layer<S> for GuardLayer {
    type Service = GuardService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GuardService {
            guard: Arc::clone(&self.guard),
            inner,
        }
    }
}

/// The guard around a service.
#[derive(Clone)]
pub struct GuardService<S> {
    guard: Arc<Guard>,
    inner: S,
}

impl<S, B> Service<Request<B>> for GuardService<S>
where
    S: Service<Request<B>, Response = Response<Body>, Error = Infallible>,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        match self.guard.admit(request.headers()) {
            Ok(granted) => {
                request.extensions_mut().insert(granted);
                Box::pin(self.inner.call(request))
            }
            Err(refusal) => Box::pin(ready(Ok(refusal.into_response()))),
        }
    }
}
