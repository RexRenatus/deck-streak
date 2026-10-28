//! Schema v1: every type that crosses between the app and a scheduling companion.
//!
//! Every field is an integer, one of the three closed enums below, a fixed array of them, a `Vec`
//! of them, or one of the two named checkpoint byte fields ([`CheckpointReply::bytes`] and
//! [`Restore`]'s private field). Note text, tags, deck names and paths, guids, media and every
//! other field a learner writes never cross: no type here can hold them. The field order of each
//! type is its order on the wire ([`crate::codec`]).

use alloc::string::String;
use alloc::vec::Vec;

/// The schema's major version. A companion that answers another major is refused.
pub const MAJOR: u16 = 1;
/// The schema's minor version. A minor difference is admitted.
pub const MINOR: u16 = 0;
/// The frame header's length: `len u32 | major u16 | minor u16 | kind u8 | epoch u32`.
pub const HEADER_LEN: usize = 13;
/// The largest frame either side sends or reads: 1 MiB.
pub const FRAME_MAX: usize = 1_048_576;
/// The most review events one `Fold` frame carries.
pub const FOLD_MAX: usize = 4_096;
/// The longest interval a due date may set, in days.
pub const INTERVAL_DAYS_MAX: u32 = 36_500;
/// A review event's bytes on the wire.
pub const REVIEW_EVENT_LEN: usize = 74;

/// What a reply says about the companion's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Status {
    /// The request was answered.
    Ok = 0,
    /// No companion is present: the public default.
    Absent = 1,
    /// The companion holds a fixture model, which answers no due date.
    FixtureRefuses = 2,
    /// The companion cannot answer now.
    Busy = 3,
    /// The companion failed.
    Error = 4,
    /// A review key was sent again with other fields.
    Conflict = 5,
    /// The companion does not serve this request.
    Unsupported = 6,
}

/// Which model the companion holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Model {
    /// A trained model, the only one whose due dates are written.
    Trained = 0,
    /// A fixture, for tests; it answers no due date.
    Fixture = 1,
}

/// Whether the companion adapts to its one owner's own reviews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Adapter {
    /// No adaptation.
    Off = 0,
    /// Adaptation to the owner's own reviews.
    OwnerOnly = 1,
}

/// The companion's answer to `Hello`: who it is and which model it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HelloReply {
    /// Whether the companion is present and ready.
    pub status: Status,
    /// The schema major it speaks.
    pub major: u16,
    /// The schema minor it speaks.
    pub minor: u16,
    /// Its build, as an opaque 256-bit id.
    pub build_id: [u64; 4],
    /// Its model, as an opaque 256-bit id.
    pub model_id: [u64; 4],
    /// Whether that model is trained or a fixture.
    pub model: Model,
    /// Whether it adapts to its owner.
    pub adapter: Adapter,
}

/// One review, as the app's log holds it: 74 bytes on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReviewEvent {
    /// The review's id: the review log's id of the row.
    pub t_ms: i64,
    /// Where the review was recorded.
    pub origin: [u64; 2],
    /// The review's sequence number at its origin.
    pub seq: u64,
    /// The card's id.
    pub card: i64,
    /// The card's note id.
    pub note: i64,
    /// The card's home deck id: its original deck when it sits in a filtered deck.
    pub home_deck: i64,
    /// The home deck's preset id.
    pub preset: i64,
    /// The grade, 1 to 4: the engine's rating plus one.
    pub grade: u8,
    /// The review kind, 0 to 3: learn, review, relearn, filtered.
    pub kind: u8,
    /// The answer time in milliseconds, capped at one hour.
    pub duration_ms: u32,
    /// The study day the review counts on, computed by the app.
    pub study_day: i32,
}

/// One press, asked about before anything is written: what the due date would be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Preview {
    /// The review the press would record.
    pub event: ReviewEvent,
    /// The target retention, in parts per million.
    pub target_ppm: u32,
}

/// A request from the app to the companion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Request {
    /// Opens a session: the schema the app speaks and its release.
    Hello {
        /// The schema major the app speaks.
        major: u16,
        /// The schema minor the app speaks.
        minor: u16,
        /// The app's release, major, minor and patch.
        release: [u16; 3],
    },
    /// Reviews the companion has not seen, at most [`FOLD_MAX`] per frame.
    Fold {
        /// The reviews, in log order.
        events: Vec<ReviewEvent>,
    },
    /// Asks the due date a press would get, and changes no state.
    Preview(Preview),
    /// Folds a review the app has written, read back from its log.
    Commit {
        /// The written review.
        event: ReviewEvent,
    },
    /// Removes an undone review by its key.
    Retract {
        /// The review's id.
        t_ms: i64,
        /// Where it was recorded.
        origin: [u64; 2],
        /// Its sequence number at its origin.
        seq: u64,
    },
    /// Asks for the companion's state as opaque bytes.
    Checkpoint,
    /// Hands back bytes a `Checkpoint` reply gave.
    Restore(Restore),
    /// Forgets every state the companion holds.
    Purge,
    /// Asks for the hash of the companion's state.
    StateHash,
}

/// A reply from the companion to the app.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reply {
    /// The answer to `Hello`.
    Hello(HelloReply),
    /// The answer to `Fold`, `Commit`, `Retract`, `Restore` and `Purge`.
    Ack {
        /// Whether the request was taken.
        status: Status,
        /// How many reviews the companion has folded.
        folded: u64,
        /// The id of the newest review it has folded.
        head_t_ms: i64,
    },
    /// The answer to `Preview`: the due date the press would get.
    Due {
        /// Whether a due date was computed.
        status: Status,
        /// The card it is for.
        card: i64,
        /// The previewed review's id.
        staged_t_ms: i64,
        /// The interval, in days.
        interval_days: u32,
        /// The study day the card falls due on.
        due_day: i32,
        /// The model that computed it.
        model_id: [u64; 4],
    },
    /// The answer to `Checkpoint`.
    Checkpoint(CheckpointReply),
    /// The answer to `StateHash`.
    Hash {
        /// Whether the hash was computed.
        status: Status,
        /// The 256-bit hash.
        words: [u64; 4],
    },
    /// A request the companion did not answer.
    Refused {
        /// Why.
        status: Status,
    },
}

impl Reply {
    /// The status every reply carries.
    #[must_use]
    pub fn status(&self) -> Status {
        match self {
            Self::Hello(hello) => hello.status,
            Self::Checkpoint(checkpoint) => checkpoint.status,
            Self::Ack { status, .. }
            | Self::Due { status, .. }
            | Self::Hash { status, .. }
            | Self::Refused { status } => *status,
        }
    }
}

/// The companion's state as opaque bytes, which the app stores and hands back verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CheckpointReply {
    /// Whether the state was written.
    pub status: Status,
    /// The model the state belongs to.
    pub model_id: [u64; 4],
    /// The adapter mode the state was folded under.
    pub adapter: Adapter,
    /// The state, derived from the review log and opaque to the app.
    pub bytes: Vec<u8>,
}

/// A `Restore` request's bytes. Its one field is private and its one constructor takes a
/// [`CheckpointReply`], so the app can hand back only bytes the companion gave it:
///
/// ```
/// use deck_streak_companion::wire::{Adapter, CheckpointReply, Restore, Status};
///
/// let reply = CheckpointReply {
///     status: Status::Ok,
///     model_id: [1, 2, 3, 4],
///     adapter: Adapter::Off,
///     bytes: vec![7, 8, 9],
/// };
/// let restore = Restore::from(reply);
/// assert_eq!(restore.bytes(), &[7, 8, 9]);
/// ```
///
/// Raw bytes do not build one:
///
/// ```compile_fail
/// use deck_streak_companion::wire::Restore;
///
/// let forged = Restore { bytes: vec![7, 8, 9] };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Restore {
    /// The bytes.
    pub bytes: Vec<u8>,
}

impl Restore {
    /// The bytes to restore.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The decoder's own door: a `Restore` frame read on the companion's side.
    pub(crate) fn decoded(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl From<CheckpointReply> for Restore {
    fn from(reply: CheckpointReply) -> Self {
        Self { bytes: reply.bytes }
    }
}

/// A note's text, as this stub of the schema carries it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteText {
    /// The text.
    pub text: String,
}
