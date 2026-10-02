//! The registry of every stateful context's data-rights port, and the one pair of use cases every
//! surface calls to export and erase the owner's data (SPEC-021 R1; ADR-002: the registry crosses
//! contexts, so it lives here; CHARTER 13).
//!
//! A delivery that creates a table adds it to its context's port, and a context with a port adds
//! it here, in the same change: `tests/data_rights_symmetry.rs` fails until it does.

use deck_streak_agent::data_rights::AgentDataRights;
use deck_streak_analytics::data_rights::AnalyticsDataRights;
use deck_streak_economy::data_rights::EconomyDataRights;
use deck_streak_ingest::data_rights::IngestDataRights;
use deck_streak_kernel::{DataRights, Db, KernelDataRights};
use deck_streak_notifications::data_rights::NotificationsDataRights;
use deck_streak_privacy::{Erasure, Export, PrivacyError};
use deck_streak_progression::data_rights::ProgressionDataRights;
use deck_streak_readings::data_rights::ReadingsDataRights;
use deck_streak_streaks::data_rights::StreaksDataRights;
use deck_streak_vault::data_rights::VaultDataRights;

use crate::data_rights::CoordinationDataRights;

/// The kernel's port: the settings generation reset in place, the schema version table exempt.
static KERNEL: KernelDataRights = KernelDataRights;
/// Ingest's port: the sync record exported and erased, its state reset in place.
static INGEST: IngestDataRights = IngestDataRights;
/// Analytics' port: the daily rollups and per-course statistics exported and erased (SPEC-071).
static ANALYTICS: AnalyticsDataRights = AnalyticsDataRights;
/// Progression's port: the XP ledger, the settled XP, the day buffs, the badges and the records exported and erased (SPEC-040, SPEC-072).
static PROGRESSION: ProgressionDataRights = ProgressionDataRights;
/// Notifications' port: the router's decisions, deliveries, queue, feed and settings exported and
/// erased (SPEC-041).
static NOTIFICATIONS: NotificationsDataRights = NotificationsDataRights;
/// Readings' port: the topic days and the runs exported and erased (SPEC-045).
static READINGS: ReadingsDataRights = ReadingsDataRights;
/// The agent's port: the duty runs exported and erased (SPEC-043).
static AGENT: AgentDataRights = AgentDataRights;
/// The economy's port: the coin ledger exported and erased, the shop's row reset in place
/// (SPEC-082).
static ECONOMY: EconomyDataRights = EconomyDataRights;
/// The vault's port: the law drill answers and grades exported and erased, and never a note
/// (SPEC-110, ADR-118).
static VAULT: VaultDataRights = VaultDataRights;
/// Coordination's own port: the cron-fire ledger exempt.
static STREAKS: StreaksDataRights = StreaksDataRights;
/// Coordination's own port: the cron-fire ledger exempt, the instrument reports exported and
/// erased (SPEC-094).
static COORDINATION: CoordinationDataRights = CoordinationDataRights;

/// Every stateful context's port, in the order an erase runs them: the kernel, ingest, analytics
/// (SPEC-071), progression (SPEC-040), notifications (SPEC-041), readings (SPEC-045), the agent
/// (SPEC-043), streaks (SPEC-076), the economy (SPEC-082), the vault (SPEC-110) and coordination.
/// Identity keeps its sessions in memory (ADR-024), so it has no table and no port.
#[must_use]
pub fn ports() -> Vec<&'static dyn DataRights> {
    vec![
        &KERNEL,
        &INGEST,
        &ANALYTICS,
        &PROGRESSION,
        &NOTIFICATIONS,
        &READINGS,
        &AGENT,
        &STREAKS,
        &ECONOMY,
        &VAULT,
        &COORDINATION,
    ]
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
