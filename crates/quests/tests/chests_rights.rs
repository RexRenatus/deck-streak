//! An erase empties the chest tables and resets the two single rows (SPEC-081 A21, R21; SPEC-021):
//! the quests data-rights port, as `privacy` drives it on one write, leaves no chest and no token,
//! the pity counters at 0 and 0 and the chest settings at their defaults. Every row is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{DataRights, Db, Disposition, TableRights};
use deck_streak_quests::data_rights::{
    CHEST_SETTINGS_TABLE, CHESTS_TABLE, PITY_TABLE, QUESTS_CONTEXT, QuestsDataRights,
    XP_TOKENS_TABLE,
};
use serde_json::{Map, Value, json};

/// Rows the seed writes into `chests` and `xp_tokens`.
const SEEDED: usize = 101;

/// How many chests the table holds, read past the ports.
async fn chests(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM chests")
        .fetch_one(db.reader())
        .await
        .expect("the chests' count")
}

/// How many tokens the table holds, read past the ports.
async fn tokens(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM xp_tokens")
        .fetch_one(db.reader())
        .await
        .expect("the tokens' count")
}

/// `pity`'s row as the test reads it back: id, both counters and `created_at`.
async fn pity(db: &Db) -> Vec<(i64, i64, i64, i64)> {
    sqlx::query_as("SELECT id, since_epic, since_legendary, created_at FROM pity ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("the pity row")
}

/// `chest_settings`' row: id, the per-day maximum, the vault hour and `created_at`.
async fn settings(db: &Db) -> Vec<(i64, i64, i64, i64)> {
    sqlx::query_as("SELECT id, per_day_max, vault_hour, created_at FROM chest_settings ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("the settings row")
}

#[tokio::test]
async fn the_chest_tables_are_exported_and_erased() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");

    let mut write = db.write().await.expect("a write");
    for statement in [
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
         INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, choice, \
         announced, created_at) SELECT 20000 + i, 'session', 1000 * i, 'rare', i, 'opened', '', 1, \
         1000 * i FROM n",
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
         INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, consumed, \
         created_at) SELECT i, 1000 * i, 1000 * i, 1000 * i + 7200000, 1, 1000 * i FROM n",
        "UPDATE pity SET since_epic = 4, since_legendary = 9 WHERE id = 1",
        "UPDATE chest_settings SET per_day_max = 5, vault_hour = 18 WHERE id = 1",
    ] {
        sqlx::query(statement)
            .execute(&mut *write)
            .await
            .expect("a seed row");
    }
    write.commit().await.expect("the seed committed");
    let created_pity = pity(&db).await[0].3;
    let created_settings = settings(&db).await[0].3;

    // The export holds every row of the four tables, none paged away.
    let mut read = db.write().await.expect("a write for the read");
    let exported = QuestsDataRights
        .export(&mut read)
        .await
        .expect("the export");
    read.commit().await.expect("the read committed");
    let rows = |name: &str| {
        exported
            .iter()
            .find(|table| table.table == name)
            .map(|table| table.rows.len())
    };
    assert_eq!(rows(CHESTS_TABLE), Some(SEEDED));
    assert_eq!(rows(XP_TOKENS_TABLE), Some(SEEDED));
    assert_eq!(rows(PITY_TABLE), Some(1));
    assert_eq!(rows(CHEST_SETTINGS_TABLE), Some(1));
    let pity_row = exported
        .iter()
        .find(|table| table.table == PITY_TABLE)
        .expect("pity is exported")
        .rows[0]
        .clone();
    assert_eq!(
        pity_row,
        json!({"id": 1, "since_epic": 4, "since_legendary": 9, "created_at": created_pity})
    );

    // The erase runs on one write, as privacy drives it.
    let mut write = db.write().await.expect("a write");
    QuestsDataRights.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase committed");

    assert_eq!(chests(&db).await, 0, "the erase empties the chests");
    assert_eq!(tokens(&db).await, 0, "the erase empties the tokens");
    assert_eq!(
        pity(&db).await,
        [(1, 0, 0, created_pity)],
        "the pity row stays, both counters at 0"
    );
    assert_eq!(
        settings(&db).await,
        [(1, 3, 21, created_settings)],
        "the settings row stays, at its defaults"
    );
}

/// Mutation coverage (R21): the port declares its four tables, the two row tables erased and the
/// two single rows reset to exactly their reset values.
#[test]
fn the_quests_port_declares_its_tables_and_their_reset_rows() {
    let declaration = QuestsDataRights
        .declaration()
        .expect("the quests declaration");
    let mut pity_reset = Map::new();
    pity_reset.insert("since_epic".to_owned(), Value::from(0));
    pity_reset.insert("since_legendary".to_owned(), Value::from(0));
    let mut settings_reset = Map::new();
    settings_reset.insert("per_day_max".to_owned(), Value::from(3));
    settings_reset.insert("vault_hour".to_owned(), Value::from(21));
    assert_eq!(
        declaration.tables(),
        [
            TableRights {
                table: "chests",
                disposition: Disposition::ExportAndErase,
            },
            TableRights {
                table: "pity",
                disposition: Disposition::ResetInPlace { row: pity_reset },
            },
            TableRights {
                table: "xp_tokens",
                disposition: Disposition::ExportAndErase,
            },
            TableRights {
                table: "chest_settings",
                disposition: Disposition::ResetInPlace {
                    row: settings_reset
                },
            },
        ]
    );
    assert_eq!(declaration.context(), QUESTS_CONTEXT);
}
