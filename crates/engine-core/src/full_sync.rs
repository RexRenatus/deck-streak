//! The full-sync choice: the one rule both clients share when the engine answers a normal sync
//! with a full sync (SPEC-357, ADR-368).
//!
//! The owner chooses a direction. An upload replaces the server's collection with the device's; a
//! download replaces the device's with the server's. The core counts what each side loses by id
//! (ADR-368 D2), takes a backup of the side the write replaces (D3), and before an upload finds the
//! offsite snapshot and re-reads the server (D4); at the write it re-reads the device (D9). The
//! model `formal/tla/FullSyncChoice` states the order these steps keep (D8).

use std::collections::BTreeSet;

use anki_proto::sync::SyncCollectionResponse;

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

/// A direction the owner can choose: which side's collection the write replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The device's collection replaces the server's.
    Upload,
    /// The server's collection replaces the device's.
    Download,
}

/// The directions the engine's answer to a normal sync offers (SPEC-357 R4, SPEC-342 F1): a full
/// sync offers both, a server with no cards the upload only, a device with no cards the download
/// only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    /// Whether the upload is offered.
    pub upload: bool,
    /// Whether the download is offered.
    pub download: bool,
}

impl Offer {
    /// The offer the engine's answer makes. It is read from the answer, never from a screen, so
    /// both clients offer the same directions on the same answer.
    #[must_use]
    pub fn from_answer(_answer: &SyncCollectionResponse) -> Self {
        Self::of(true, true)
    }

    /// An offer of the named directions.
    #[must_use]
    pub const fn of(upload: bool, download: bool) -> Self {
        Self { upload, download }
    }

    /// Whether `direction` is offered: a direction not offered is refused (R4).
    #[must_use]
    pub const fn admits(self, direction: Direction) -> bool {
        match direction {
            Direction::Upload => self.upload,
            Direction::Download => self.download,
        }
    }
}

/// What one direction loses, by id: the rows the replaced side holds and the kept side lacks
/// (SPEC-357 R5). A difference of ids, never a count of unsynced rows, because a review already
/// synced is lost by a download once the server was replaced without it (SPEC-342 F6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Losses {
    /// Review-log rows lost.
    pub reviews: usize,
    /// Cards lost.
    pub cards: usize,
    /// Notes lost.
    pub notes: usize,
}

impl Losses {
    /// What a write that replaces `replaced` with `kept` loses: each id `replaced` holds and
    /// `kept` lacks.
    #[must_use]
    pub fn between(_replaced: &IdSets, _kept: &IdSets) -> Self {
        Self::default()
    }
}

/// The counts the choice screen shows: what each offered direction loses, and `None` for a
/// direction the engine did not offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// What the upload loses: the server's rows the device lacks.
    pub upload: Option<Losses>,
    /// What the download loses: the device's rows the server lacks.
    pub download: Option<Losses>,
}

/// The choice's first state: the offer and both sides' ids, from which the counts are shown and a
/// direction is confirmed (SPEC-357 R3). A later refusal the owner must see returns here, with new
/// counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counted {
    offer: Offer,
    sides: Box<Sides>,
}

/// Both sides' ids the counts were taken from, boxed so each state stays small to move.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sides {
    device: IdSets,
    server: IdSets,
}

impl Counted {
    /// The choice over the device's ids and the server copy's, for the engine's offer.
    #[must_use]
    pub fn show(offer: Offer, device: IdSets, server: IdSets) -> Self {
        Self {
            offer,
            sides: Box::new(Sides { device, server }),
        }
    }

    /// What each offered direction loses: an upload replaces the server, a download the device.
    #[must_use]
    pub fn counts(&self) -> Counts {
        let Sides { device, server } = &*self.sides;
        Counts {
            upload: self.offer.upload.then(|| Losses::between(server, device)),
            download: self.offer.download.then(|| Losses::between(device, server)),
        }
    }

    /// The owner's tap on `direction`.
    ///
    /// # Errors
    ///
    /// The unchanged state, when the offer does not admit `direction`: the choice is kept.
    pub fn confirm(self, direction: Direction) -> Result<Confirmed, Self> {
        Ok(Confirmed {
            counted: self,
            direction,
        })
    }
}

/// A direction the owner confirmed, before its backup (SPEC-357 R6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmed {
    counted: Counted,
    direction: Direction,
}

impl Confirmed {
    /// The backup of the side the write replaces: the device's collection for a download, the
    /// server copy for an upload. It is accepted only when its ids hold every review, card and
    /// note id of that side (ADR-368 D3).
    ///
    /// # Errors
    ///
    /// The unchanged state, when the backup lacks an id of the replaced side.
    pub fn backed_up(self, backup: &IdSets) -> Result<BackedUp, Self> {
        let sides = &self.counted.sides;
        let replaced = match self.direction {
            Direction::Upload => &sides.device,
            Direction::Download => &sides.server,
        };
        let holds = backup.reviews.is_superset(&replaced.reviews)
            && backup.cards.is_superset(&replaced.cards)
            && backup.notes.is_superset(&replaced.notes);
        if holds {
            Ok(BackedUp {
                confirmed: self,
                backup: backup.clone(),
            })
        } else {
            Err(self)
        }
    }
}

/// A confirmed direction whose backup holds the replaced side: a download is ready from here, and
/// an upload goes on to the snapshot check (SPEC-357 R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackedUp {
    confirmed: Confirmed,
    backup: IdSets,
}

impl BackedUp {
    /// A download's next step: its backup is all it needs before the write.
    ///
    /// # Errors
    ///
    /// The unchanged state for an upload, which needs the snapshot and the re-check first.
    pub fn download_ready(self) -> Result<Ready, Self> {
        if self.confirmed.direction == Direction::Upload {
            Ok(Ready {
                confirmed: self.confirmed,
                backup: self.backup,
            })
        } else {
            Err(self)
        }
    }

    /// An upload's next step: a sealed offsite snapshot of the server's collection is found
    /// before that collection is replaced (ADR-368 D4).
    ///
    /// # Errors
    ///
    /// The unchanged state for a download, which has no snapshot step, and for an answer that
    /// found no snapshot.
    pub fn snapshot_found(self, answer: &SnapshotAnswer) -> Result<Checked, Self> {
        let upload = self.confirmed.direction == Direction::Upload;
        if upload && answer.found {
            Ok(Checked { backed_up: self })
        } else {
            Err(self)
        }
    }
}

/// An upload whose snapshot was found, before the server is read again (SPEC-357 R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    backed_up: BackedUp,
}

impl Checked {
    /// The re-check: a fresh server copy, read just before the upload, must equal the copy the
    /// counts were taken from, by ids and by modified stamp, so a change another client synced
    /// since the counts is shown before it is overwritten.
    ///
    /// # Errors
    ///
    /// New counts over the fresh copy, when it differs: the owner sees them and confirms again.
    pub fn rechecked(self, fresh: IdSets) -> Result<Ready, Counted> {
        let copy = &self.backed_up.confirmed.counted.sides.server;
        let same_ids =
            fresh.reviews == copy.reviews && fresh.cards == copy.cards && fresh.notes == copy.notes;
        let same_stamp = fresh.modified == copy.modified;
        if same_ids && same_stamp {
            let BackedUp { confirmed, backup } = self.backed_up;
            Ok(Ready { confirmed, backup })
        } else {
            let Counted { offer, sides } = self.backed_up.confirmed.counted;
            Err(Counted::show(offer, sides.device, fresh))
        }
    }
}

/// The choice at its write: the last state, which only [`Ready::at_write`] turns into the
/// [`Write`] a client's one-way call consumes. It is not `Clone`, so one tap makes one write.
#[derive(Debug, PartialEq, Eq)]
pub struct Ready {
    confirmed: Confirmed,
    backup: IdSets,
}

impl Ready {
    /// The write's last check, at the write (ADR-368 D9): the device is read again, because study
    /// is not held during the choice. A download refuses a device row its backup lacks; an upload
    /// admits a device gain, which it sends.
    ///
    /// # Errors
    ///
    /// New counts over the device read now, when a download's backup lacks one of its rows.
    pub fn at_write(self, _device_now: &IdSets) -> Result<Write, Counted> {
        Ok(made(self.confirmed.direction))
    }
}

/// A write of `direction`.
fn made(direction: Direction) -> Write {
    Write { direction }
}

/// The one write a confirmed and checked choice makes: the direction a client's one-way call runs.
/// Its field is private and only [`Ready::at_write`] makes one (SPEC-357 R3).
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub struct Write {
    direction: Direction,
}

impl Write {
    /// The direction the one-way call runs.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
}
