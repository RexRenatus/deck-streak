//! The native client's sync messages as the core builds them (SPEC-503 A3; R2, R3, R5): the
//! normal sync names its endpoint and never asks for media, the reply's requirement reads as the
//! engine numbers it, and the statement is read at the endpoint's origin only when the endpoint
//! keeps the login's rule. Every expected value is written here, never read from the core.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

use anki_proto::sync::{SyncCollectionRequest, SyncCollectionResponse};
use deck_streak_engine_core::native_sync::{auth, normal_sync, required, statement_url};
use prost::Message;

#[test]
fn the_native_normal_sync_request_names_its_endpoint_and_never_asks_for_media() {
    let request = SyncCollectionRequest::decode(
        normal_sync(&auth(
            String::from("planted-host-key"),
            String::from("https://sync.example.invalid/"),
        ))
        .as_slice(),
    )
    .expect("the core's request decodes");
    let auth = request.auth.expect("the request carries its auth");
    assert_eq!(
        (request.sync_media, auth.hkey, auth.endpoint),
        (
            false,
            String::from("planted-host-key"),
            Some(String::from("https://sync.example.invalid/"))
        ),
        "the native normal sync names its key and endpoint and asks for no media"
    );
}

#[test]
fn the_reply_reads_as_the_engines_number_and_an_undecodable_one_as_none() {
    let reply = SyncCollectionResponse {
        required: 3,
        ..SyncCollectionResponse::default()
    }
    .encode_to_vec();
    assert_eq!(
        (required(&reply), required(&[0xff, 0xff])),
        (Some(3), None),
        "a reply's requirement, and none for bytes that do not decode"
    );
}

#[test]
fn the_statement_is_read_at_the_origin_of_an_endpoint_the_rule_keeps() {
    let read = [
        "https://sync.example.invalid/sync/",
        "http://127.0.0.1:8080/",
        "http://example.invalid/",
        "not a url",
    ]
    .map(statement_url);
    assert_eq!(
        read,
        [
            Some(String::from(
                "https://sync.example.invalid/api/sync/minimum-client"
            )),
            Some(String::from(
                "http://127.0.0.1:8080/api/sync/minimum-client"
            )),
            None,
            None,
        ],
        "the statement's URL at the origin, and none for an endpoint the rule refuses"
    );
}
