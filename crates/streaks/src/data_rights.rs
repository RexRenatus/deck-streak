//! Streaks' data-rights port (SPEC-076 R20, R27; CHARTER 13): the streak rows, the freeze ledger,
//! the habit strength and the relight's due list are the owner's data, exported whole and erased;
//! the governor's one row is reset in place, so an erase returns both tracks and the governor to
//! their start state.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, KernelError, PortFuture,
    TableRights,
};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const STREAKS_CONTEXT: &str = "streaks";

/// The table holding one row per track (`migrations/007601_streaks_state_and_governor.sql`).
pub const STREAK_STATE_TABLE: &str = "streak_state";
/// The table holding the freeze ledger.
pub const FREEZE_EVENTS_TABLE: &str = "freeze_events";
/// The table holding the habit strength of each study day.
pub const HABIT_STRENGTH_TABLE: &str = "habit_strength";
/// The table holding the governor's one row.
pub const GOVERNOR_STATE_TABLE: &str = "governor_state";
/// The table holding the relight's due list (`migrations/007602_streaks_relight_due.sql`).
pub const RELIGHT_DUE_TABLE: &str = "relight_due";

/// The relight's due list, one exported row per day (R27).
async fn relight_due_rows(connection: &mut SqliteConnection) -> Result<Vec<Value>, KernelError> {
    let rows = sqlx::query!("SELECT study_day, created_at FROM relight_due ORDER BY study_day")
        .fetch_all(connection)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "study_day": row.study_day,
                "created_at": row.created_at,
            })
        })
        .collect())
}

/// Streaks' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct StreaksDataRights;

impl DataRights for StreaksDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        let mut start = Map::new();
        start.insert("lapse_since".to_owned(), Value::Null);
        start.insert("standby".to_owned(), Value::from(0));
        start.insert("notified_day".to_owned(), Value::Null);
        let mut tables: Vec<TableRights> = [
            STREAK_STATE_TABLE,
            FREEZE_EVENTS_TABLE,
            HABIT_STRENGTH_TABLE,
            RELIGHT_DUE_TABLE,
        ]
        .into_iter()
        .map(|table| TableRights {
            table,
            disposition: Disposition::ExportAndErase,
        })
        .collect();
        tables.push(TableRights {
            table: GOVERNOR_STATE_TABLE,
            disposition: Disposition::ResetInPlace { row: start },
        });
        Declaration::new(STREAKS_CONTEXT, tables)
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let streaks = sqlx::query!(
                "SELECT track, current_days, longest_days, freezes, last_study_day, comeback_armed, \
                        created_at FROM streak_state ORDER BY track"
            )
            .fetch_all(&mut *connection)
            .await?;
            let events = sqlx::query!(
                r#"SELECT id AS "id!", study_day, delta, reason, created_at
                   FROM freeze_events ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let strength = sqlx::query!(
                "SELECT study_day, strength, created_at FROM habit_strength ORDER BY study_day"
            )
            .fetch_all(&mut *connection)
            .await?;
            let due = relight_due_rows(&mut *connection).await?;
            let governor = sqlx::query!(
                "SELECT id, lapse_since, standby, notified_day, created_at FROM governor_state"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: STREAK_STATE_TABLE,
                    rows: streaks
                        .into_iter()
                        .map(|row| {
                            json!({
                                "track": row.track,
                                "current_days": row.current_days,
                                "longest_days": row.longest_days,
                                "freezes": row.freezes,
                                "last_study_day": row.last_study_day,
                                "comeback_armed": row.comeback_armed,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: FREEZE_EVENTS_TABLE,
                    rows: events
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "delta": row.delta,
                                "reason": row.reason,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: HABIT_STRENGTH_TABLE,
                    rows: strength
                        .into_iter()
                        .map(|row| {
                            json!({
                                "study_day": row.study_day,
                                "strength": row.strength,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: RELIGHT_DUE_TABLE,
                    rows: due,
                },
                ExportedTable {
                    table: GOVERNOR_STATE_TABLE,
                    rows: governor
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "lapse_since": row.lapse_since,
                                "standby": row.standby,
                                "notified_day": row.notified_day,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM streak_state")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM freeze_events")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM habit_strength")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM relight_due")
                .execute(&mut *connection)
                .await?;
            // The governor's one row stays, at its start state, so the fold never finds it absent.
            sqlx::query!(
                "UPDATE governor_state SET lapse_since = NULL, standby = 0, notified_day = NULL"
            )
            .execute(connection)
            .await?;
            Ok(())
        })
    }
}
