//! The registry of every stateful context's data-rights port, and the one pair of use cases every
//! surface calls to export and erase the owner's data (SPEC-021 R1; ADR-002: the registry crosses
//! contexts, so it lives here; CHARTER 13).
//!
//! A delivery that creates a table adds it to its context's port, and a context with a port adds
//! it here, in the same change: `tests/data_rights_symmetry.rs` fails until it does.

use deck_streak_ingest::data_rights::IngestDataRights;
use deck_streak_kernel::{DataRights, Db, KernelDataRights};
use deck_streak_privacy::{Erasure, Export, PrivacyError};

use crate::data_rights::CoordinationDataRights;

/// The kernel's port: the settings generation reset in place, the schema version table exempt.
static KERNEL: KernelDataRights = KernelDataRights;
/// Ingest's port: the sync record exported and erased, its state reset in place.
static INGEST: IngestDataRights = IngestDataRights;
/// Coordination's own port: the cron-fire ledger exempt.
static COORDINATION: CoordinationDataRights = CoordinationDataRights;

/// Every stateful context's port, in the order an erase runs them. At W0 the stateful contexts are
/// the kernel, ingest and coordination; identity keeps its sessions in memory (ADR-024), so it has
/// no table and no port.
#[must_use]
pub fn ports() -> Vec<&'static dyn DataRights> {
    vec![&KERNEL, &INGEST, &COORDINATION]
}

/// The owner's export: every table a port exports or resets, as one JSON document
/// (`privacy::export` over [`ports`]).
///
/// # Errors
///
/// Every refusal of [`PrivacyError`].
pub async fn export_all(db: &Db) -> Result<Export, PrivacyError> {
    deck_streak_privacy::export(db, &ports()).await
}

/// The owner's erase: every table a port exports or resets, in one transaction
/// (`privacy::erase` over [`ports`]).
///
/// # Errors
///
/// Every refusal of [`PrivacyError`].
pub async fn erase_all(db: &Db) -> Result<Erasure, PrivacyError> {
    deck_streak_privacy::erase(db, &ports()).await
}
