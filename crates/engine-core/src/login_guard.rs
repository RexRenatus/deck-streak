//! The endpoint guard on the engine's sync calls (SPEC-347 R2; ADR-358 D4; SPEC-364 R2).
//!
//! The engine sends a login whose endpoint is absent to its own built-in default server, which is
//! not this project's, and its HTTP client brings its own TLS, so no platform transport rule
//! governs where the password goes. So before the engine sees the login pair, the core decodes the
//! request and admits it only when its endpoint parses as a URL with no username or password, over
//! `https`, or over plain `http` to a loopback IP literal, which is the engine's own test server.
//! The same one rule reads the endpoint of every normal sync and every one-way write, and a normal
//! sync may not ask for media, which a later part brings (#631).
//! Every other call is refused in the engine's own error shape, a `BackendError` of kind
//! `INVALID_INPUT`, so a client reads one shape of refusal for the call. The refusal names the rule
//! broken and never the endpoint, the user or the password.

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::sync::{SyncAuth, SyncCollectionRequest, SyncLoginRequest};
use prost::Message;
use url::{Host, Url};

/// The rule a request that does not decode as a login breaks.
const UNDECODABLE: &str = "the sync login request does not decode";
/// The rule a login that names no endpoint, or an empty one, breaks.
const ABSENT: &str = "the sync login names no endpoint";
/// The rule an endpoint that does not parse as a URL breaks.
const UNPARSEABLE: &str = "the sync login's endpoint is not a URL";
/// The rule an endpoint carrying a username or a password breaks.
const CREDENTIALS: &str = "the sync login's endpoint carries a username or a password";
/// The rule every other scheme, and plain `http` to anything but a loopback IP literal, breaks.
const SCHEME: &str =
    "the sync login's endpoint is neither https nor plain http to a loopback address";

/// The rule a request that does not decode as a normal sync breaks.
const SYNC_UNDECODABLE: &str = "the normal sync request does not decode";
/// The rule a normal sync that asks for media breaks: media is a later part's (#631).
const MEDIA: &str = "a normal sync may not sync media";

/// Checks one encoded `SyncLoginRequest` before the engine sees it, and admits it only when its
/// endpoint is one a login may reach: `https`, or plain `http` to a loopback IP literal, with no
/// username or password in the URL.
///
/// # Errors
///
/// An encoded `BackendError` of kind `INVALID_INPUT` whose message names the rule the request
/// breaks: it does not decode, it names no endpoint, its endpoint does not parse, carries a
/// username or a password, or has another scheme or host.
pub fn check(request: &[u8]) -> Result<(), Vec<u8>> {
    refused(broken_rule(request))
}

/// Checks the `SyncAuth` a normal sync or a one-way write carries, by the login's one endpoint
/// rule, before the engine sees it (SPEC-364 R2).
///
/// # Errors
///
/// An encoded `BackendError` of kind `INVALID_INPUT` whose message names the rule the endpoint
/// breaks.
pub fn check_auth(auth: &SyncAuth) -> Result<(), Vec<u8>> {
    refused(endpoint_rule(auth.endpoint.clone()))
}

/// Checks one encoded `SyncCollectionRequest`, the normal sync, before the engine sees it: it asks
/// for no media, and its auth meets the endpoint rule (SPEC-364 R2).
///
/// # Errors
///
/// An encoded `BackendError` of kind `INVALID_INPUT` whose message names the rule the request
/// breaks: it does not decode, it asks for media, or its endpoint breaks the rule.
pub fn check_sync(request: &[u8]) -> Result<(), Vec<u8>> {
    let Ok(request) = SyncCollectionRequest::decode(request) else {
        return refused(Some(SYNC_UNDECODABLE));
    };
    if request.sync_media {
        return refused(Some(MEDIA));
    }
    check_auth(&request.auth.unwrap_or_default())
}

/// The engine-shaped refusal for `rule`, or `Ok` when no rule is broken.
fn refused(rule: Option<&'static str>) -> Result<(), Vec<u8>> {
    match rule {
        None => Ok(()),
        Some(rule) => Err(BackendError {
            message: rule.to_owned(),
            kind: Kind::InvalidInput.into(),
            ..BackendError::default()
        }
        .encode_to_vec()),
    }
}

/// The rule `request` breaks, or `None` when the login may reach its endpoint.
fn broken_rule(request: &[u8]) -> Option<&'static str> {
    let Ok(login) = SyncLoginRequest::decode(request) else {
        return Some(UNDECODABLE);
    };
    endpoint_rule(login.endpoint)
}

/// The rule `endpoint` breaks, or `None` when a sync call may reach it: the one rule every sync
/// call's endpoint meets.
fn endpoint_rule(endpoint: Option<String>) -> Option<&'static str> {
    let Some(endpoint) = endpoint.filter(|endpoint| !endpoint.is_empty()) else {
        return Some(ABSENT);
    };
    let Ok(url) = Url::parse(&endpoint) else {
        return Some(UNPARSEABLE);
    };
    if !url.username().is_empty() || url.password().is_some() {
        return Some(CREDENTIALS);
    }
    if url.scheme() == "https" || (url.scheme() == "http" && is_loopback(&url)) {
        None
    } else {
        Some(SCHEME)
    }
}

/// Whether `url`'s host is a loopback IP literal: a name, `localhost` included, is never one.
fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(_)) | None => false,
    }
}
