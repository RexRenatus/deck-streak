//! Progression's data-rights port declares `xp_ledger` exported and erased, exports every grant
//! with every column, and an erase leaves the ledger empty (SPEC-040 A10, R9; SPEC-021; CHARTER
//! 13). Every grant here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{
    DataRights, Db, Disposition, ExportedTable, StudyDay, TableRights, Track, UtcMillis,
};
use deck_streak_progression::data_rights::ProgressionDataRights;
use deck_streak_progression::grant::{
    GrantAnswer, GrantPort, GrantRequest, GrantScope, GrantSource,
};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::XpAmount;
use serde_json::json;

#[tokio::test]
async fn the_xp_ledger_is_exported_and_erased() {
    let declaration = ProgressionDataRights
        .declaration()
        .expect("progression's declaration is well formed");
    assert_eq!(declaration.context(), "progression");
    assert_eq!(
        declaration.tables(),
        [TableRights {
            table: "xp_ledger",
            disposition: Disposition::ExportAndErase,
        }],
        "the XP ledger is the owner's data: exported and erased (CHARTER 13)"
    );

    // What the declaration says, the port does: every grant is exported with every column...
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteXpLedger::new(db.clone());
    let grants = [
        (
            20_000,
            "reading:read:r1",
            Track::Language,
            40,
            GrantScope::PerDay,
            1_000,
        ),
        (
            20_001,
            "reading:studied:r1",
            Track::Law,
            60,
            GrantScope::Once,
            2_000,
        ),
    ];
    for (day, source, track, amount, scope, at) in grants {
        let request = GrantRequest {
            study_day: StudyDay::from_epoch_day(day),
            source: GrantSource::new(source).expect("a source of the grammar"),
            track,
            amount: XpAmount::new(amount),
            scope,
        };
        let answer = ledger
            .grant(&request, UtcMillis::from_epoch_millis(at))
            .await
            .expect("the grant runs");
        assert_eq!(answer, GrantAnswer::Granted(XpAmount::new(amount)));
    }
    let mut write = db.write().await.expect("a write");
    let exported = ProgressionDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    assert_eq!(
        exported,
        [ExportedTable {
            table: "xp_ledger",
            rows: vec![
                json!({"id": 1, "study_day": 20_000, "source": "reading:read:r1",
                       "track": "language", "amount": 40, "scope": "per-day", "created_at": 1_000}),
                json!({"id": 2, "study_day": 20_001, "source": "reading:studied:r1",
                       "track": "law", "amount": 60, "scope": "once", "created_at": 2_000}),
            ],
        }]
    );

    // ...and an erase, inside the caller's transaction, leaves the ledger empty.
    ProgressionDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM xp_ledger")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(remaining, 0, "the erase left a grant");
    let total = ledger.total().await.expect("the total is read");
    assert_eq!(total.get(), 0, "an erased ledger sums to nothing");
    let mut after = db.write().await.expect("a write");
    let exported = ProgressionDataRights
        .export(&mut after)
        .await
        .expect("the port exports");
    assert_eq!(
        exported,
        [ExportedTable {
            table: "xp_ledger",
            rows: Vec::new(),
        }],
        "the erased ledger is still exported, and holds no row"
    );
}
