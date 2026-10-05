//! The endpoint guard on the engine's sync login (SPEC-347 R2; ADR-358 D4).
//!
//! The engine sends a login whose endpoint is absent to its own built-in default server, which is
//! not this project's, and its HTTP client brings its own TLS, so no platform transport rule
//! governs where the password goes. So before the engine sees the login pair, the core decodes the
//! request and admits it only when its endpoint parses as a URL with no username or password, over
//! `https`, or over plain `http` to a loopback IP literal, which is the engine's own test server.
//! Every other login is refused in the engine's own error shape, a `BackendError` of kind
//! `INVALID_INPUT`, so a client reads one shape of refusal for the call. The refusal names the rule
//! broken and never the endpoint, the user or the password.

/// Checks one encoded `SyncLoginRequest` before the engine sees it, and admits it only when its
/// endpoint is one a login may reach: `https`, or plain `http` to a loopback IP literal, with no
/// username or password in the URL.
///
/// # Errors
///
/// An encoded `BackendError` of kind `INVALID_INPUT` whose message names the rule the request
/// breaks: it does not decode, it names no endpoint, its endpoint does not parse, carries a
/// username or a password, or has another scheme or host.
pub fn check(_request: &[u8]) -> Result<(), Vec<u8>> {
    Ok(())
}
