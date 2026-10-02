//! The vault's data-rights port (SPEC-110 R17; SPEC-118 R12; SPEC-021): `drill_answers`,
//! `drill_grades` and `inbox_captures` are the owner's data, so each is exported and erased
//! (CHARTER 13) through the kernel's port that `privacy` drives. An erase empties the tables and
//! touches no note and no capture: the drill notes, the captures and their stubs are the owner's
//! own files in the owner's vault (ADR-118).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

use crate::capture_store::INBOX_CAPTURES_TABLE;
use crate::drill_store::{DRILL_ANSWERS_TABLE, DRILL_GRADES_TABLE};

/// The context this port speaks for.
pub const VAULT_CONTEXT: &str = "vault";

/// The vault's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct VaultDataRights;

impl DataRights for VaultDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            VAULT_CONTEXT,
            vec![
                TableRights {
                    table: DRILL_ANSWERS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: DRILL_GRADES_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: INBOX_CAPTURES_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let answers = sqlx::query!(
                r#"SELECT drill_id AS "drill_id!", study_day, surface, created_at
                   FROM drill_answers ORDER BY drill_id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let grades = sqlx::query!(
                r#"SELECT drill_id AS "drill_id!", drill_type, subject, xp, study_day, created_at
                   FROM drill_grades ORDER BY drill_id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let captures = sqlx::query!(
                r#"SELECT stem AS "stem!", capture_key, kind, source, attachment, captured_at,
                          state, destination, filed_day, created_at
                   FROM inbox_captures ORDER BY stem"#
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: DRILL_ANSWERS_TABLE,
                    rows: answers
                        .into_iter()
                        .map(|row| {
                            json!({
                                "drill_id": row.drill_id,
                                "study_day": row.study_day,
                                "surface": row.surface,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: DRILL_GRADES_TABLE,
                    rows: grades
                        .into_iter()
                        .map(|row| {
                            json!({
                                "drill_id": row.drill_id,
                                "drill_type": row.drill_type,
                                "subject": row.subject,
                                "xp": row.xp,
                                "study_day": row.study_day,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: INBOX_CAPTURES_TABLE,
                    rows: captures
                        .into_iter()
                        .map(|row| {
                            json!({
                                "stem": row.stem,
                                "capture_key": row.capture_key,
                                "kind": row.kind,
                                "source": row.source,
                                "attachment": row.attachment,
                                "captured_at": row.captured_at,
                                "state": row.state,
                                "destination": row.destination,
                                "filed_day": row.filed_day,
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
            sqlx::query!("DELETE FROM drill_answers")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM drill_grades")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM inbox_captures")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
