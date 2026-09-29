//! Notifications' data-rights port (SPEC-041 R11; SPEC-021): the router's five tables are the
//! owner's data, each exported whole and erased, and the owner's latest message is exported and
//! reset in place to no message (SPEC-084 R14), through the kernel's port that `privacy` drives
//! (CHARTER 13).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, KernelError, PortFuture,
    TableRights,
};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

use crate::ledger::{
    DECISIONS_TABLE, DELIVERIES_TABLE, FEED_TABLE, OWNER_MESSAGE_TABLE, QUEUE_TABLE, SETTINGS_TABLE,
};

/// The context this port speaks for.
pub const NOTIFICATIONS_CONTEXT: &str = "notifications";

/// Notifications' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct NotificationsDataRights;

impl DataRights for NotificationsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        let mut no_message = Map::new();
        no_message.insert("message_id".to_owned(), Value::Null);
        no_message.insert("arrived_at".to_owned(), Value::Null);
        let reset = TableRights {
            table: OWNER_MESSAGE_TABLE,
            disposition: Disposition::ResetInPlace { row: no_message },
        };
        Declaration::new(
            NOTIFICATIONS_CONTEXT,
            [
                DECISIONS_TABLE,
                DELIVERIES_TABLE,
                QUEUE_TABLE,
                FEED_TABLE,
                SETTINGS_TABLE,
            ]
            .into_iter()
            .map(|table| TableRights {
                table,
                disposition: Disposition::ExportAndErase,
            })
            .chain([reset])
            .collect(),
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            Ok(vec![
                decisions(connection).await?,
                deliveries(connection).await?,
                queue(connection).await?,
                feed(connection).await?,
                settings(connection).await?,
                owner_message(connection).await?,
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            // Every row of every table, inside the caller's transaction.
            sqlx::query!("DELETE FROM notification_decisions")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM notification_deliveries")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM notification_queue")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM in_app_feed")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM notification_settings")
                .execute(&mut *connection)
                .await?;
            sqlx::query!(
                "UPDATE owner_last_message SET message_id = NULL, arrived_at = NULL WHERE id = 1"
            )
            .execute(&mut *connection)
            .await?;
            Ok(())
        })
    }
}

/// Every decision, in the order recorded.
async fn decisions(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", dedupe_key, kind, surface, arm, reason, tier_requested,
                  tier_rendered, study_day, created_at
           FROM notification_decisions ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: DECISIONS_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "dedupe_key": row.dedupe_key,
                    "kind": row.kind,
                    "surface": row.surface,
                    "arm": row.arm,
                    "reason": row.reason,
                    "tier_requested": row.tier_requested,
                    "tier_rendered": row.tier_rendered,
                    "study_day": row.study_day,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// Every delivery's claim.
async fn deliveries(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", kind, dedupe_key, scope, surface, study_day, lapse_id, created_at
           FROM notification_deliveries ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: DELIVERIES_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "kind": row.kind,
                    "dedupe_key": row.dedupe_key,
                    "scope": row.scope,
                    "surface": row.surface,
                    "study_day": row.study_day,
                    "lapse_id": row.lapse_id,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// Every held or abandoned celebration on the queue.
async fn queue(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", kind, dedupe_key, surface, tier_requested, tier_pending, text, hold,
                  tries, state, deferred_at, study_day, created_at
           FROM notification_queue ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: QUEUE_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "kind": row.kind,
                    "dedupe_key": row.dedupe_key,
                    "surface": row.surface,
                    "tier_requested": row.tier_requested,
                    "tier_pending": row.tier_pending,
                    "text": row.text,
                    "hold": row.hold,
                    "tries": row.tries,
                    "state": row.state,
                    "deferred_at": row.deferred_at,
                    "study_day": row.study_day,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// Every item of the in-app feed, seen or not.
async fn feed(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", dedupe_key, kind, tier, text, seen_at, created_at
           FROM in_app_feed ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: FEED_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "dedupe_key": row.dedupe_key,
                    "kind": row.kind,
                    "tier": row.tier,
                    "text": row.text,
                    "seen_at": row.seen_at,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// Every setting the owner changed.
async fn settings(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows =
        sqlx::query!("SELECT key, value, created_at FROM notification_settings ORDER BY key")
            .fetch_all(connection)
            .await?;
    Ok(ExportedTable {
        table: SETTINGS_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "key": row.key,
                    "value": row.value,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// The owner's latest message: its one row.
async fn owner_message(connection: &mut SqliteConnection) -> Result<ExportedTable, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", message_id, arrived_at, created_at
           FROM owner_last_message ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: OWNER_MESSAGE_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "message_id": row.message_id,
                    "arrived_at": row.arrived_at,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}
