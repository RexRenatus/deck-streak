//! The guard: the bearer's parser, the match, the one refusal and the request layer (SPEC-119 R9
//! to R12, R14; ADR-121, ADR-320).

use std::convert::Infallible;
use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use deck_streak_kernel::Clock;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tower::{Layer, Service};

use crate::grants::{Grants, Scope, Scopes};
use crate::limiter::{Bucket, Limiter, Outcome};

/// The one word every refusal answers (`mcp_auth.py:DENIED_MESSAGE`).
pub const DENIED: &str = "unauthorized";

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

    /// The response every refusal answers, whatever its outcome (R12).
    #[must_use]
    pub fn into_response(self) -> Response<Body> {
        refused()
    }
}

/// The one refusal: 401, `WWW-Authenticate: Bearer` and the body [`DENIED`], with no other header
/// (R12). A rate-limited refusal answers it too, so a caller cannot tell the limiter tripped.
fn refused() -> Response<Body> {
    let mut response = Response::new(Body::from(DENIED));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

/// The token a request's `Authorization` header presents (R9): exactly one header, the scheme
/// `Bearer` in any case, one space, then a non-empty token of bytes 0x21 to 0x7E. Anything else
/// presents none.
fn presented(headers: &HeaderMap) -> Option<&[u8]> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let (scheme, rest) = value.as_bytes().split_at_checked(6)?;
    if !scheme.eq_ignore_ascii_case(b"Bearer") {
        return None;
    }
    let token = rest.strip_prefix(b" ")?;
    if token.is_empty() || !token.iter().all(u8::is_ascii_graphic) {
        return None;
    }
    Some(token)
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
        let token = presented(headers);
        if let Some(token) = token {
            let scopes = self.matched(token);
            if scopes.holds(Scope::Core) {
                return Ok(Granted {
                    scopes,
                    bucket: Bucket::of(token),
                });
            }
        }
        // The bucket is the token's when one parses, else the first header's whole value.
        let bucket = Bucket::of(token.unwrap_or_else(|| {
            headers
                .get(AUTHORIZATION)
                .map_or(&[][..], HeaderValue::as_bytes)
        }));
        Err(self.refuse(bucket, Scope::Core))
    }

    /// Allows `scope` to an admitted request whose grant holds it, or refuses it (R12, R13).
    ///
    /// # Errors
    ///
    /// A [`Refusal`] when the grant does not hold the scope.
    pub fn authorize(&self, granted: &Granted, scope: Scope) -> Result<(), Refusal> {
        if granted.scopes.holds(scope) {
            return Ok(());
        }
        Err(self.refuse(granted.bucket.clone(), scope))
    }

    /// The scopes of the grant whose digest equals the presented token's, folded over every grant
    /// with no early exit and compared by `ct_eq` alone (R10).
    fn matched(&self, token: &[u8]) -> Scopes {
        let presented: [u8; 32] = Sha256::digest(token).into();
        self.grants
            .grants()
            .iter()
            .map(|grant| (grant.scopes, grant.digest.ct_eq(&presented)))
            .fold(Scopes::NONE, Scopes::or_if)
    }

    /// A refusal in `bucket` while asking for `scope`: the limiter decides it, and one warning
    /// names the outcome, the bucket and the scope, and never the token (R13, R14).
    fn refuse(&self, bucket: Bucket, scope: Scope) -> Refusal {
        let outcome = self.limiter.refuse(&bucket);
        tracing::warn!(
            outcome = outcome.name(),
            bucket = bucket.as_str(),
            scope = scope.name()
        );
        Refusal { outcome, bucket }
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
