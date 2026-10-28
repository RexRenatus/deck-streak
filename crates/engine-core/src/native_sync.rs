//! The normal sync's request and answer as plain values, for the native adapter, which links no
//! protocol crate of its own (SPEC-503 R2, R3, R5; ADR-521 D2).
//!
//! The web engine builds these messages itself. The native adapter takes a host key and an
//! endpoint from the app and hands the core's messages to the dispatcher, so the request a native
//! normal sync sends is built here alone: its auth names the endpoint, and it never asks for media.

use anki_proto::sync::{SyncAuth, SyncCollectionRequest, SyncCollectionResponse, SyncLoginRequest};
use prost::Message;

use crate::handshake;

/// The engine's sync auth for `host_key` at `endpoint`, with the engine's own timeout.
pub fn auth(host_key: String, endpoint: String) -> SyncAuth {
    SyncAuth {
        hkey: host_key,
        endpoint: Some(endpoint),
        io_timeout_secs: None,
    }
}

/// The encoded normal sync for `auth`. Media is never asked for: a native normal sync runs with
/// media off, as the web's does (R2).
#[must_use]
pub fn normal_sync(auth: &SyncAuth) -> Vec<u8> {
    SyncCollectionRequest {
        auth: Some(auth.clone()),
        sync_media: false,
    }
    .encode_to_vec()
}

/// What the server requires next, from a normal sync's encoded reply, as the engine numbers it, or
/// `None` when the reply does not decode or names a negative number (R5).
#[must_use]
pub fn required(reply: &[u8]) -> Option<u32> {
    let reply = SyncCollectionResponse::decode(reply).ok()?;
    u32::try_from(reply.required).ok()
}

/// The normal sync's answer the choice's offer is read from, given what it required (R4).
pub fn answer(required: u32) -> SyncCollectionResponse {
    SyncCollectionResponse {
        required: i32::try_from(required).unwrap_or(i32::MAX),
        ..SyncCollectionResponse::default()
    }
}

/// The URL of the service's statement for a normal sync at `endpoint`: the statement the login
/// reads, at the same origin, and `None` when the login guard refuses the endpoint, so an endpoint
/// the rule refuses is never read (R3; SPEC-374 R8).
#[must_use]
pub fn statement_url(endpoint: &str) -> Option<String> {
    let login = SyncLoginRequest {
        endpoint: Some(endpoint.to_owned()),
        ..SyncLoginRequest::default()
    };
    handshake::statement_url(&login.encode_to_vec())
}
