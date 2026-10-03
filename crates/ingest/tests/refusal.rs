//! SPEC-128 A1-A4: a refused owner request is recorded on `ingest_state`, beside the pending flag.

mod support;

use deck_streak_ingest::state::{Refusal, RefusalReason, SqliteIngestState};
use deck_streak_kernel::UtcMillis;

const CODES: [(&str, RefusalReason); 8] = [
    ("rescore_unrecorded", RefusalReason::RescoreUnrecorded),
    ("sync_settings_refused", RefusalReason::SyncSettingsRefused),
    (
        "credentials_directory_refused",
        RefusalReason::CredentialsDirectoryRefused,
    ),
    (
        "scope_settings_refused",
        RefusalReason::ScopeSettingsRefused,
    ),
    ("recompute_refused", RefusalReason::RecomputeRefused),
    ("sync_record_failed", RefusalReason::SyncRecordFailed),
    (
        "obligations_unreadable",
        RefusalReason::ObligationsUnreadable,
    ),
    ("recompute_failed", RefusalReason::RecomputeFailed),
];

#[tokio::test]
async fn the_migration_refuses_a_reason_outside_the_closed_set() {
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let mut write = db.write().await.expect("a write");
    let outside = sqlx::query(
        "UPDATE ingest_state SET refused_at = 5, refused_reason = 'not_a_code' WHERE id = 1",
    )
    .execute(&mut *write)
    .await;
    assert!(outside.is_err(), "a code outside the closed set is refused");
    let unpaired =
        sqlx::query("UPDATE ingest_state SET refused_reason = 'recompute_failed' WHERE id = 1")
            .execute(&mut *write)
            .await;
    assert!(unpaired.is_err(), "a reason without its instant is refused");
    let instant_only =
        sqlx::query("UPDATE ingest_state SET refused_at = 5, refused_reason = NULL WHERE id = 1")
            .execute(&mut *write)
            .await;
    assert!(
        instant_only.is_err(),
        "an instant without its reason is refused"
    );
    let inside = sqlx::query(
        "UPDATE ingest_state SET refused_at = 5, refused_reason = 'recompute_failed' WHERE id = 1",
    )
    .execute(&mut *write)
    .await;
    assert!(
        inside.is_ok(),
        "a code of the set, paired, is accepted: {inside:?}"
    );
}

#[tokio::test]
async fn a_refusal_stores_its_reason_and_instant_and_clears_the_flag() {
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db);
    let asked = UtcMillis::from_epoch_millis(1_000);
    let refused = UtcMillis::from_epoch_millis(2_000);
    state.request_rescore(asked).await.expect("requested");
    assert!(state.load().await.expect("read").rescore_pending);
    state
        .record_refusal(RefusalReason::ScopeSettingsRefused, refused)
        .await
        .expect("recorded");
    let after = state.load().await.expect("read");
    assert!(!after.rescore_pending, "the refusal clears the flag");
    assert_eq!(
        after.refusal,
        Some(Refusal {
            reason: RefusalReason::ScopeSettingsRefused,
            at: refused
        })
    );
}

#[tokio::test]
async fn a_new_request_clears_the_refused_record() {
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db);
    state
        .record_refusal(
            RefusalReason::RecomputeRefused,
            UtcMillis::from_epoch_millis(2_000),
        )
        .await
        .expect("recorded");
    assert!(state.load().await.expect("read").refusal.is_some());
    state
        .request_rescore(UtcMillis::from_epoch_millis(3_000))
        .await
        .expect("requested");
    let after = state.load().await.expect("read");
    assert!(after.rescore_pending);
    assert_eq!(
        after.refusal, None,
        "an old refusal never answers a new request"
    );
}

#[tokio::test]
async fn every_reason_round_trips_through_its_code() {
    assert_eq!(RefusalReason::ALL.len(), CODES.len());
    for (code, reason) in CODES {
        assert_eq!(reason.as_str(), code);
        assert_eq!(RefusalReason::parse(code), Some(reason), "{code}");
    }
    assert_eq!(RefusalReason::parse("not_a_code"), None);
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db);
    for (_, reason) in CODES {
        state
            .record_refusal(reason, UtcMillis::from_epoch_millis(9))
            .await
            .expect("recorded");
        assert_eq!(
            state.load().await.expect("read").refusal.map(|r| r.reason),
            Some(reason)
        );
    }
}
