//! The cron-fire ledger is declared exempt from export and erase, with its reason, and survives an
//! erase (SPEC-027 A14; R10; CHARTER 13).

// An integration test is test code: its helpers panic on a failed database.
#![allow(clippy::expect_used)]

use deck_streak_coordination::data_rights::{
    COORDINATION_CONTEXT, CRON_FIRES_TABLE, CoordinationDataRights,
};
use deck_streak_coordination::jobs::FireDate;
use deck_streak_coordination::ledger::{CronLedger, Outcome, SqliteCronLedger};
use deck_streak_kernel::{DataRights, Db, Disposition, UtcMillis};

#[tokio::test]
async fn the_cron_fire_ledger_is_declared_exempt_with_its_reason() {
    let declaration = CoordinationDataRights
        .declaration()
        .expect("the declaration is accepted");
    assert_eq!(declaration.context(), COORDINATION_CONTEXT);
    let tables: Vec<&str> = declaration
        .tables()
        .iter()
        .map(|rights| rights.table)
        .collect();
    assert_eq!(
        tables,
        [CRON_FIRES_TABLE],
        "coordination owns the one table it declares"
    );
    let Some(Disposition::Exempt { reason }) = declaration.disposition(CRON_FIRES_TABLE) else {
        panic!(
            "cron_fires must be exempt from export and erase: {:?}",
            declaration.disposition(CRON_FIRES_TABLE)
        );
    };
    assert!(
        reason.contains("re-arm the catch-up double-send guard"),
        "the exemption says why: {reason}"
    );

    // An erase leaves the ledger as it was, and an export carries none of it.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteCronLedger::new(db.clone());
    let date = FireDate::from_epoch_day(20_000);
    let at = UtcMillis::from_epoch_millis(20_000 * 86_400_000);
    assert!(
        ledger
            .claim("synthetic_daily", date, at)
            .await
            .expect("the claim"),
        "the first claim of a fire succeeds"
    );
    let mut write = db.write().await.expect("a write");
    CoordinationDataRights
        .erase(&mut write)
        .await
        .expect("the erase runs");
    let exported = CoordinationDataRights
        .export(&mut write)
        .await
        .expect("the export runs");
    write.commit().await.expect("the erase commits");
    let kept = ledger
        .row("synthetic_daily", date)
        .await
        .expect("the ledger reads")
        .expect("the claim survives the erase");
    assert_eq!(
        (kept.catchup_count, kept.last_outcome),
        (1, Outcome::Catchup)
    );
    let named: Vec<&str> = exported.iter().map(|table| table.table).collect();
    assert_eq!(named.len(), 0, "an export carries no ledger row: {named:?}");
    // The guard still holds after the erase: a second claim of the fire is refused.
    assert_eq!(
        ledger.claim("synthetic_daily", date, at).await.ok(),
        Some(false)
    );
    db.close().await;
}
