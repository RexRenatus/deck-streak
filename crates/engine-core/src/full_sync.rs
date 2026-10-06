//! The full-sync choice: the one rule both clients share when the engine answers a normal sync
//! with a full sync (SPEC-357, ADR-368).
//!
//! The owner chooses a direction. An upload replaces the server's collection with the device's; a
//! download replaces the device's with the server's. The core counts what each side loses by id
//! (ADR-368 D2), takes a backup of the side the write replaces (D3), and before an upload finds the
//! offsite snapshot and re-reads the server (D4); at the write it re-reads the device (D9). The
//! model `formal/tla/FullSyncChoice` states the order these steps keep (D8).

use std::collections::BTreeSet;

/// The ids one side's collection holds, read by the core's fixed reads
/// ([`crate::dispatch::Dispatcher::id_sets`]): what the counts, the backup check, the re-check and
/// the write's last check compare. Cards and notes follow the same rule as reviews.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdSets {
    /// Every review-log id: one row per review, the thing a full sync can lose silently.
    pub reviews: BTreeSet<i64>,
    /// Every card id.
    pub cards: BTreeSet<i64>,
    /// Every note id.
    pub notes: BTreeSet<i64>,
    /// The collection's modified stamp, which an upload's re-check compares beside the ids: an
    /// edit to a row both sides hold moves it and no id.
    pub modified: i64,
}

/// What this device has not sent to the server, read offline by one fixed statement
/// ([`crate::dispatch::Dispatcher::unsynced`]), so a client can warn before the owner loses it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unsynced {
    /// The reviews whose sequence number marks them as not yet synced.
    pub reviews: u32,
    /// Whether the collection changed since its last sync.
    pub changed: bool,
    /// Whether its schema changed since its last sync, which forces the next sync to be full.
    pub schema: bool,
}

impl Unsynced {
    /// Whether a client warns: any unsynced review or any change since the last sync is something
    /// a full sync in the wrong direction would lose.
    #[must_use]
    pub fn warns(&self) -> bool {
        self.reviews > 0 || self.changed || self.schema
    }
}

/// The answer to an upload's question, whether a sealed offsite snapshot of the server's
/// collection is found (ADR-340). Where the answer comes from is the adapter's; the rule reads only
/// whether it was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotAnswer {
    /// True only when a sealed snapshot was found.
    pub found: bool,
}
