//! The recorder's own proof (SPEC-083 R33, A33): the recording layer sees a write before any proof
//! rests on it.
//!
//! Another synthetic client, playing the owner's other device, uploads a collection whole, syncs one
//! review and changes one setting, each through the layer in front of the engine's own sync server.
//! The layer must record an `upload`, a chunk carrying the card and its review-log row, and a
//! change carrying the setting's new value, and the shared classifier must mark all three. A layer
//! that recorded nothing fails here first.

mod support;

use std::fs;

use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use support::recording::{Recording, local_change};
use support::synthetic::{self, Shape, Side};
use support::{Fixture, SyncServer};

/// An instant in a study day, one hour past its 04:00 rollover in UTC (the default rule).
const IN_A_STUDY_DAY: i64 = 20_000 * 86_400_000 + 5 * 3_600_000;
/// The setting another client changes, and the value it gives it.
const PLANTED_KEY: &str = "deckstreakRecorderControl";
const PLANTED_VALUE: i64 = 7_483;

#[test]
fn the_recorder_sees_a_planted_upload_and_a_planted_local_change() {
    const TEST: &str = "the_recorder_sees_a_planted_upload_and_a_planted_local_change";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let base = scratch.path().join("server");
    synthetic::build_shaped(
        &support::server_collection(&base),
        Side::Server,
        Shape::SMALL,
    );
    let server = SyncServer::start(TEST, &base);
    let recording = Recording::start(server.endpoint());

    // The other client starts from the owner's collection, as a second device does.
    let fixture = Fixture::new(recording.endpoint());
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db),
        support::clock_at(IN_A_STUDY_DAY),
    );
    runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    let other = scratch.path().join("other.anki2");
    fs::copy(fixture.copy(), &other).expect("the other client starts from the same collection");
    recording.clear();

    // A full upload, planted through the layer.
    support::upload_from_another_client(&runtime, &other, recording.endpoint());
    let uploads: Vec<_> = recording
        .requests()
        .into_iter()
        .filter(|request| request.method == "upload")
        .collect();
    assert_eq!(uploads.len(), 1, "the layer recorded the full upload");
    assert!(
        local_change(&uploads[0]).is_some(),
        "the classifier marks an upload"
    );
    recording.clear();

    // One review, synced normally: a chunk carrying the card and its review-log row.
    support::review_on_another_client(&runtime, &other, recording.endpoint(), 1);
    let chunks: Vec<_> = recording
        .requests()
        .into_iter()
        .filter(|request| request.method == "applyChunk" && !request.cards().is_empty())
        .collect();
    assert_eq!(chunks.len(), 1, "the layer recorded the review's chunk");
    let chunk = &chunks[0];
    assert_eq!(chunk.cards().len(), 1, "the chunk carries the one card");
    assert_eq!(
        chunk.revlog().len(),
        1,
        "the chunk carries its review-log row"
    );
    assert_eq!(
        chunk.revlog()[0].cid,
        chunk.cards()[0].id,
        "the row belongs to the card"
    );
    assert!(
        local_change(chunk).is_some(),
        "the classifier marks a chunk that carries a card"
    );
    recording.clear();

    // One setting, synced normally: a change carrying the setting's new value.
    support::change_setting_on_another_client(
        &runtime,
        &other,
        recording.endpoint(),
        PLANTED_KEY,
        PLANTED_VALUE,
    );
    let changes: Vec<_> = recording
        .requests()
        .into_iter()
        .filter(|request| request.method == "applyChanges" && request.settings().is_some())
        .collect();
    assert_eq!(changes.len(), 1, "the layer recorded the setting's change");
    let settings = changes[0].settings().expect("the change carries settings");
    assert_eq!(
        settings
            .get(PLANTED_KEY)
            .and_then(serde_json::Value::as_i64),
        Some(PLANTED_VALUE),
        "the change carries the setting's new value"
    );
    assert!(
        local_change(&changes[0]).is_some(),
        "the classifier marks a change that carries a setting"
    );
}
