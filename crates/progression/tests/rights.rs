//! Progression's data-rights port declares `xp_ledger` exported and erased, exports every grant
//! with every column, and an erase leaves the ledger empty (SPEC-040 A10, R9; SPEC-021; CHARTER
//! 13). Every grant here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used, clippy::too_many_lines)]

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
        [
            "xp_ledger",
            "xp_settlement",
            "buffs",
            "badges_earned",
            "records"
        ]
        .map(|table| TableRights {
            table,
            disposition: Disposition::ExportAndErase,
        }),
        "the XP ledger, its settlement and the day buffs are the owner's data: exported and \
         erased (CHARTER 13; SPEC-072 R6, R22)"
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
    sqlx::query(
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (20000, 'reviews', 'language', 70, 1, 3000)",
    )
    .execute(db.reader())
    .await
    .expect("a settlement row");
    sqlx::query(
        "INSERT INTO buffs (study_day, kind, created_at) VALUES (20001, 'ascendant', 4000)",
    )
    .execute(db.reader())
    .await
    .expect("a buff row");
    sqlx::query(
        "INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, created_at) \
         VALUES ('first_steps', 0, 'First Steps', 'x', 20000, NULL, 5000)",
    )
    .execute(db.reader())
    .await
    .expect("a badge row");
    sqlx::query(
        "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
         VALUES ('best_score', 80, 20001, 70, 7000, 6000)",
    )
    .execute(db.reader())
    .await
    .expect("a record row");
    let mut write = db.write().await.expect("a write");
    let exported = ProgressionDataRights
        .export(&mut write)
        .await
        .expect("the port exports");
    assert_eq!(
        exported,
        [
            ExportedTable {
                table: "xp_ledger",
                rows: vec![
                    json!({"id": 1, "study_day": 20_000, "source": "reading:read:r1",
                           "track": "language", "amount": 40, "scope": "per-day", "created_at": 1_000}),
                    json!({"id": 2, "study_day": 20_001, "source": "reading:studied:r1",
                           "track": "law", "amount": 60, "scope": "once", "created_at": 2_000}),
                ],
            },
            ExportedTable {
                table: "xp_settlement",
                rows: vec![json!({"id": 1, "study_day": 20_000, "source": "reviews",
                                  "track": "language", "amount": 70, "closed": 1, "created_at": 3_000})],
            },
            ExportedTable {
                table: "buffs",
                rows: vec![json!({"study_day": 20_001, "kind": "ascendant", "created_at": 4_000})],
            },
            ExportedTable {
                table: "badges_earned",
                rows: vec![
                    json!({"badge_key": "first_steps", "tier": 0, "name": "First Steps",
                                  "emoji": "x", "study_day": 20_000, "celebrated_at": null,
                                  "created_at": 5_000})
                ],
            },
            ExportedTable {
                table: "records",
                rows: vec![
                    json!({"kind": "best_score", "value": 80, "study_day": 20_001,
                                  "previous": 70, "celebrated_at": 7_000, "created_at": 6_000})
                ],
            },
        ]
    );

    // ...and an erase, inside the caller's transaction, leaves the ledger empty.
    ProgressionDataRights
        .erase(&mut write)
        .await
        .expect("the port erases");
    write.commit().await.expect("the erase commits");
    for (table, count) in [
        ("xp_ledger", "SELECT count(*) FROM xp_ledger"),
        ("xp_settlement", "SELECT count(*) FROM xp_settlement"),
        ("buffs", "SELECT count(*) FROM buffs"),
        ("badges_earned", "SELECT count(*) FROM badges_earned"),
        ("records", "SELECT count(*) FROM records"),
    ] {
        let remaining: i64 = sqlx::query_scalar(count)
            .fetch_one(db.reader())
            .await
            .expect("a count");
        assert_eq!(remaining, 0, "the erase left a row of {table}");
    }
    let total = ledger.total().await.expect("the total is read");
    assert_eq!(total.get(), 0, "an erased ledger sums to nothing");
    let mut after = db.write().await.expect("a write");
    let exported = ProgressionDataRights
        .export(&mut after)
        .await
        .expect("the port exports");
    assert_eq!(
        exported,
        [
            "xp_ledger",
            "xp_settlement",
            "buffs",
            "badges_earned",
            "records"
        ]
        .map(|table| ExportedTable {
            table,
            rows: Vec::new(),
        }),
        "the erased tables are still exported, and hold no row"
    );
}
