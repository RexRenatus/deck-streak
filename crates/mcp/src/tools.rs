//! The MCP server's tools (SPEC-119 R15 to R17; ADR-329).
//!
//! This part serves one tool, `get_law_track`, the one whose owner exists and whose scope is not
//! `core` (ADR-329 D1). A tool asks the guard for its scope before it reads anything (R17), and a
//! refusal is the tool error whose whole text is [`DENIED`], with no data (R12). The grant a tool
//! asks about is the one the guard's request layer admitted the request with: the layer puts it in
//! the HTTP request's extensions, and the transport hands a tool the request's parts.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::http::request::Parts;
use deck_streak_coordination::law::{LawBlock, law_block};
use deck_streak_kernel::{Clock, Db, KernelError, StudyDayRule};
use rmcp::handler::server::wrapper::Json;
use rmcp::model::{CallToolResult, ContentBlock, Extensions};
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Serialize;

use crate::grants::Scope;
use crate::guard::{DENIED, Granted, Guard};

/// The text of the tool error a law track that could not be read answers. It names no cause: the
/// cause is in the role's log.
const UNREADABLE: &str = "the law track could not be read";

/// What [`LawTrackSource::law_track`] answers.
pub type LawTrackFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LawBlock, KernelError>> + Send + 'a>>;

/// Where `get_law_track` reads the law track: the ledger, through its owner's use case.
pub trait LawTrackSource: Send + Sync {
    /// The law track's block for the study day now.
    fn law_track(&self) -> LawTrackFuture<'_>;
}

/// The law track as the ledger holds it: `coordination`'s `law::law_block` for the kernel's study
/// day now, with no leech count, as the API's law route reads it (R16).
pub struct LedgerLawTrack {
    db: Db,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
}

impl LedgerLawTrack {
    /// The law track over the ledger `db`, its study day by `rule` at `clock`'s now.
    #[must_use]
    pub fn new(db: Db, clock: Arc<dyn Clock>, rule: StudyDayRule) -> Self {
        Self { db, clock, rule }
    }
}

impl LawTrackSource for LedgerLawTrack {
    fn law_track(&self) -> LawTrackFuture<'_> {
        let today = self.rule.study_day(self.clock.now());
        Box::pin(law_block(&self.db, today, None))
    }
}

/// `get_law_track`'s answer: the roster golden's fields, in its order (R15). A number the ledger
/// cannot answer yet is null, never 0 (SPEC-077 R12; ADR-329 D7), and the output schema requires
/// each field, a pending one included. `required` alone would declare a pending field as its inner
/// number, refusing the null it answers, so each pending field also names `null` among its types.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct LawTrack {
    /// The law streak's current length in study days.
    pub streak: i64,
    /// The law track's XP on the study day now.
    pub xp_today: i64,
    /// The law track's lifetime XP.
    pub total_xp: i64,
    /// The level of `total_xp`.
    pub level: i64,
    /// The backlog plus the cards due today, or null before the first recompute stores them.
    #[schemars(required, extend("type" = ["integer", "null"]))]
    pub dues: Option<i64>,
    /// The active law leeches, or null while the leech port is not wired (#133).
    #[schemars(required, extend("type" = ["integer", "null"]))]
    pub leech_total: Option<i64>,
    /// The law mastery pillar rounded to 2 places, or null while the leeches are pending (#133).
    #[schemars(required, extend("type" = ["number", "null"]))]
    pub mastery: Option<f64>,
}

impl From<LawBlock> for LawTrack {
    fn from(block: LawBlock) -> Self {
        Self {
            streak: block.streak,
            xp_today: block.xp_today,
            total_xp: block.total_xp,
            level: block.level,
            dues: block.dues,
            leech_total: block.leech_active,
            mastery: block.mastery.map(rounded),
        }
    }
}

/// `mastery` rounded to 2 places as the predecessor's `server.py:_law_float` rounds it: Python's
/// `round`, half to even on the exact binary value, so 0.125 answers 0.12.
fn rounded(mastery: f64) -> f64 {
    deck_streak_kernel::pynum::round(mastery, 2)
}

/// The tool error every scope refusal answers: the one refusal word, and no data (R12).
fn denied() -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(DENIED)])
}

/// The MCP server's handler: its tools, each behind its scope (R15, R17).
#[derive(Clone)]
pub struct McpServer {
    law: Arc<dyn LawTrackSource>,
    guard: Arc<Guard>,
}

#[tool_router]
impl McpServer {
    /// The handler over the law track `law`, asking `guard` for each tool's scope.
    #[must_use]
    pub fn new(law: Arc<dyn LawTrackSource>, guard: Arc<Guard>) -> Self {
        Self { law, guard }
    }

    /// The law track's numbers, after the guard allows the request's grant `law_track` (R17). A
    /// request whose parts carry no grant is refused like a grant without the scope: the guard's
    /// layer admitted no such request, so nothing is read.
    #[tool(
        name = "get_law_track",
        description = "The law track's streak, XP today, lifetime XP and level, its dues, its \
                       active leeches and its mastery. A number not yet known is null.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn get_law_track(
        &self,
        mut extensions: Extensions,
    ) -> Result<Json<LawTrack>, CallToolResult> {
        let parts = extensions.remove::<Parts>().ok_or_else(denied)?;
        let granted = parts.extensions.get::<Granted>().ok_or_else(denied)?;
        if self.guard.authorize(granted, Scope::LawTrack).is_err() {
            return Err(denied());
        }
        match self.law.law_track().await {
            Ok(block) => Ok(Json(LawTrack::from(block))),
            Err(error) => {
                tracing::error!(%error, "the law track could not be read");
                Err(CallToolResult::error(vec![ContentBlock::text(UNREADABLE)]))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {}
