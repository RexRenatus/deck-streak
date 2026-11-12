//! The agent's data-rights port (SPEC-043 R14): `agent_runs` is the owner's data, exported and
//! erased through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

use crate::runs::AGENT_RUNS_TABLE;

/// The context this port speaks for.
pub const AGENT_CONTEXT: &str = "agent";

/// The agent's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct AgentDataRights;

impl DataRights for AgentDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            AGENT_CONTEXT,
            vec![TableRights {
                table: AGENT_RUNS_TABLE,
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let runs = sqlx::query!(
                r#"SELECT id AS "id!", duty, template, subject, verdict, cause, class, turns,
                          input_tokens, output_tokens, cost_micro_usd, duration_ms, created_at
                   FROM agent_runs ORDER BY id"#
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![ExportedTable {
                table: AGENT_RUNS_TABLE,
                rows: runs
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.id,
                            "duty": row.duty,
                            "template": row.template,
                            "subject": row.subject,
                            "verdict": row.verdict,
                            "cause": row.cause,
                            "class": row.class,
                            "turns": row.turns,
                            "input_tokens": row.input_tokens,
                            "output_tokens": row.output_tokens,
                            "cost_micro_usd": row.cost_micro_usd,
                            "duration_ms": row.duration_ms,
                            "created_at": row.created_at,
                        })
                    })
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM agent_runs")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
