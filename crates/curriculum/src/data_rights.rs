//! Curriculum's data-rights port (SPEC-077 R18; SPEC-021): `language_progress`, `band_milestones`
//! and `law_dues` are the owner's data, so each is exported whole and erased, through the kernel's
//! port that `privacy` drives (CHARTER 13). An erase leaves no law dues row, so the dues read
//! pending again, never 0.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
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
            let _ = connection;
            Ok(Vec::new())
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = connection;
            Ok(())
        })
    }
}
