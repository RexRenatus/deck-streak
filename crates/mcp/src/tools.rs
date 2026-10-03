//! The MCP server's tools (SPEC-119 R15 to R17; ADR-329).

use std::future::Future;
use std::pin::Pin;

use deck_streak_coordination::law::LawBlock;
use deck_streak_kernel::KernelError;

/// What [`LawTrackSource::law_track`] answers.
pub type LawTrackFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LawBlock, KernelError>> + Send + 'a>>;

/// Where `get_law_track` reads the law track: the ledger, through its owner's use case.
pub trait LawTrackSource: Send + Sync {
    /// The law track's block for the study day now.
    fn law_track(&self) -> LawTrackFuture<'_>;
}
