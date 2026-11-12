//! Curriculum's data-rights port (SPEC-077 R18; SPEC-021): `language_progress`, `band_milestones`
//! and `law_dues` are the owner's data, so each is exported whole and erased, through the kernel's
//! port that `privacy` drives (CHARTER 13). An erase leaves no law dues row, so the dues read
//! pending again, never 0.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const CURRICULUM_CONTEXT: &str = "curriculum";
/// Each course's stored progress (`migrations/007701_curriculum_road_to_c2_and_law.sql`).
pub const LANGUAGE_PROGRESS_TABLE: &str = "language_progress";
/// Each band a course reached, its silent baseline included (the same migration).
pub const BAND_MILESTONES_TABLE: &str = "band_milestones";
/// The law dues the last recompute counted (the same migration).
pub const LAW_DUES_TABLE: &str = "law_dues";

/// Curriculum's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumDataRights;

impl DataRights for CurriculumDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            CURRICULUM_CONTEXT,
            vec![
                TableRights {
                    table: LANGUAGE_PROGRESS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: BAND_MILESTONES_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: LAW_DUES_TABLE,
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
            // Every column of every row, as stored: the bands stay the JSON text the store wrote.
            let progress = sqlx::query!(
                "SELECT course, name, flag, mastery_pct, current_band, mature_cards, total_cards, \
                     current_unit, bands, updated_at, created_at \
                 FROM language_progress ORDER BY course"
            )
            .fetch_all(&mut *connection)
            .await?;
            let milestones = sqlx::query!(
                "SELECT course, band, study_day, baseline, celebrated_at, created_at \
                 FROM band_milestones ORDER BY course, band"
            )
            .fetch_all(&mut *connection)
            .await?;
            let dues = sqlx::query!(
                "SELECT id, study_day, backlog, due_today, updated_at, created_at \
                 FROM law_dues ORDER BY id"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: LANGUAGE_PROGRESS_TABLE,
                    rows: progress
                        .into_iter()
                        .map(|row| {
                            json!({
                                "course": row.course,
                                "name": row.name,
                                "flag": row.flag,
                                "mastery_pct": row.mastery_pct,
                                "current_band": row.current_band,
                                "mature_cards": row.mature_cards,
                                "total_cards": row.total_cards,
                                "current_unit": row.current_unit,
                                "bands": row.bands,
                                "updated_at": row.updated_at,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: BAND_MILESTONES_TABLE,
                    rows: milestones
                        .into_iter()
                        .map(|row| {
                            json!({
                                "course": row.course,
                                "band": row.band,
                                "study_day": row.study_day,
                                "baseline": row.baseline,
                                "celebrated_at": row.celebrated_at,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: LAW_DUES_TABLE,
                    rows: dues
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "backlog": row.backlog,
                                "due_today": row.due_today,
                                "updated_at": row.updated_at,
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
            sqlx::query!("DELETE FROM language_progress")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM band_milestones")
                .execute(&mut *connection)
                .await?;
            // No law dues row reads as pending again, never 0 (R10).
            sqlx::query!("DELETE FROM law_dues")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
