//! The HTTP client both senders share: one deadline over the request and its read, an error body
//! read to a fixed bound and no further, and no redirect followed (SPEC-343 R9).

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Bytes;
use hyper::header::RETRY_AFTER;
use hyper::{HeaderMap, Request, StatusCode};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::Client as Pooled;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

use crate::{BuildError, Unreached};

/// The most of an error answer's body a sender reads. A push service's error body is a short JSON
/// object; a longer one is not read past this, and its reason is taken as unknown.
pub(crate) const ERROR_BODY_LIMIT: usize = 4096;

/// How long a pooled connection may sit idle before it is closed.
const POOL_IDLE: Duration = Duration::from_secs(90);

/// Which HTTP versions a client speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Versions {
    /// HTTP/2 alone, as APNs requires: by ALPN over TLS, and by prior knowledge to a loopback fake.
    Http2Only,
    /// HTTP/1.1 or HTTP/2, as the push service offers.
    Any,
}

/// What a push service answered, read as far as a sender needs.
#[derive(Clone, Debug)]
pub(crate) struct Answer {
    /// The status.
    pub(crate) status: StatusCode,
    /// `Retry-After`, when it was sent as a number of seconds.
    pub(crate) retry_after: Option<Duration>,
    /// The body of an answer that is not a success, when it was read inside its bound; `None` for a
    /// success, and for a body past [`ERROR_BODY_LIMIT`].
    pub(crate) error_body: Option<Bytes>,
}

/// A pooled client with a deadline.
#[derive(Clone)]
pub(crate) struct Client {
    pooled: Pooled<HttpsConnector<HttpConnector>, Full<Bytes>>,
    deadline: Duration,
}

impl Client {
    /// A client speaking `versions`, whose every request and read ends at `deadline`.
    pub(crate) fn new(versions: Versions, deadline: Duration) -> Result<Self, BuildError> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let tls = hyper_rustls::HttpsConnectorBuilder::new()
            .with_provider_and_platform_verifier(provider)
            .map_err(|_| BuildError::Tls)?
            .https_or_http();
        let connector = match versions {
            Versions::Http2Only => tls.enable_http2().build(),
            Versions::Any => tls.enable_all_versions().build(),
        };
        let mut builder = Pooled::builder(TokioExecutor::new());
        builder.pool_idle_timeout(POOL_IDLE);
        if versions == Versions::Http2Only {
            builder.http2_only(true);
        }
        Ok(Self {
            pooled: builder.build(connector),
            deadline,
        })
    }

    /// Sends `request` and reads its answer, all inside the deadline.
    pub(crate) async fn post(&self, request: Request<Full<Bytes>>) -> Result<Answer, Unreached> {
        match tokio::time::timeout(self.deadline, self.exchange(request)).await {
            Ok(answer) => answer,
            Err(_) => Err(Unreached::Deadline),
        }
    }

    async fn exchange(&self, request: Request<Full<Bytes>>) -> Result<Answer, Unreached> {
        let response = self
            .pooled
            .request(request)
            .await
            .map_err(|_| Unreached::Connection)?;
        let status = response.status();
        let retry_after = retry_after(response.headers());
        if status.is_success() {
            return Ok(Answer {
                status,
                retry_after,
                error_body: None,
            });
        }
        let error_body = match Limited::new(response.into_body(), ERROR_BODY_LIMIT)
            .collect()
            .await
        {
            Ok(collected) => Some(collected.to_bytes()),
            Err(error) if error.downcast_ref::<LengthLimitError>().is_some() => None,
            Err(_) => return Err(Unreached::Unreadable),
        };
        Ok(Answer {
            status,
            retry_after,
            error_body,
        })
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Client")
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

/// `Retry-After` as a number of seconds; an HTTP date, or anything else, is none.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}
