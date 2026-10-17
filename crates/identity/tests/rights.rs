//! Identity's data-rights port exports `passkeys` whole and an erase empties it (SPEC-359 R12;
//! A28).
//!
//! The oracle is this test's own read of the table, keyed by column, so the export is compared
//! with rows the port did not build. Every row comes from the software authenticator in `support`.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_identity::IdentityDataRights;
use deck_streak_identity::data_rights::{IDENTITY_CONTEXT, PASSKEYS_TABLE};
use deck_streak_kernel::{DataRights, Disposition, TableRights};
use serde_json::{Value, json};
use sqlx::Row as _;

use support::{Authenticator, fixture};

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[tokio::test]
async fn passkeys_export_and_erase_are_symmetric() {
    let port = IdentityDataRights;
    let declaration = port.declaration().expect("the declaration holds");
    assert_eq!(
        declaration.tables(),
        [TableRights {
            table: PASSKEYS_TABLE,
            disposition: Disposition::ExportAndErase,
        }],
        "the owner's passkeys are the owner's data, exported whole and erased"
    );
    assert_eq!(declaration.context(), IDENTITY_CONTEXT);

    let fixture = fixture().await;
    let session = fixture.link_session();
    for seed in [7, 9] {
        fixture.register(&session, &Authenticator::new(seed)).await;
    }

    let oracle: Vec<Value> = sqlx::query(
        "SELECT id, telegram_user_id, hex(credential_id) AS credential_id, \
         hex(user_handle) AS user_handle, credential, counter, backup_state, created_at, \
         last_used_at FROM passkeys ORDER BY id",
    )
    .fetch_all(fixture.db.reader())
    .await
    .expect("the oracle reads")
    .iter()
    .map(|row| {
        json!({
            "id": row.get::<i64, _>("id"),
            "telegram_user_id": row.get::<i64, _>("telegram_user_id"),
            "credential_id": row.get::<String, _>("credential_id"),
            "user_handle": row.get::<String, _>("user_handle"),
            "credential": row.get::<String, _>("credential"),
            "counter": row.get::<i64, _>("counter"),
            "backup_state": row.get::<i64, _>("backup_state"),
            "created_at": row.get::<i64, _>("created_at"),
            "last_used_at": row.get::<Option<i64>, _>("last_used_at"),
        })
    })
    .collect();
    let oracle = examined("passkeys rows", oracle);
    assert_eq!(oracle.len(), 2, "both registrations wrote a row");

    let mut write = fixture.db.write().await.expect("a write");
    let exported = port.export(&mut write).await.expect("the export reads");
    let tables: Vec<&str> = exported.iter().map(|table| table.table).collect();
    assert_eq!(tables, [PASSKEYS_TABLE]);
    assert_eq!(
        exported[0].rows, oracle,
        "the export carries every row and every column"
    );
    port.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");
    assert_eq!(fixture.rows().await, 0, "the erase left a passkey behind");
}
