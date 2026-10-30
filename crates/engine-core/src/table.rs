//! The pairs a dispatcher admits on each transport, and the exempt writes it holds for an owner's
//! gesture (SPEC-345 R2, R3; ADR-356 D2, D6).
//!
//! The engine numbers its backend services and their methods when it is built, and its own clients
//! address a call by that pair. A backend service answers its own methods and, after them, the
//! methods of the collection service it fronts, so `Undo` is reached through the backend
//! collection service. Each row holds the engine's name for its pair, so a reader can check a
//! number against the engine's generated dispatch.

/// The client a dispatcher serves. Each adapter starts its dispatcher on its own transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// The native adapter (`deck-streak-ffi`), linked into the iPhone and iPad app.
    Native,
    /// The web engine (`deck-streak-web-engine`), in the web client's Worker.
    Web,
}

/// What the table decides for one pair on one transport, before the engine sees the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// An ordinary call this transport may make.
    Admit,
    /// An exempt write: never through `run`, only for an owner's gesture.
    NeedsGesture,
    /// The one answer: never through `run`, only for an owner's press (SPEC-365 R4).
    NeedsAnswer,
    /// Every other pair.
    NotAllowed,
}

/// The never-list's writes the owner may tap, each one engine method with one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExemptWrite {
    /// Forget one card: it returns to the new queue.
    Forget,
    /// Set one card's due date.
    SetDueDate,
    /// Delete one preset.
    DeletePreset,
    /// Change one note's note type.
    ChangeNoteType,
    /// Delete one card.
    DeleteCard,
    /// Delete one note.
    DeleteNote,
    /// The full-sync choice's one-way write, upload or download, of the open collection.
    OneWaySync,
    /// Undo the review's own last answer, on the card it answered (SPEC-371 R2).
    Undo,
}

/// The kind of the one target an exempt write takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A card, by id.
    Card,
    /// A note, by id.
    Note,
    /// A preset (a deck options group), by id.
    Preset,
    /// The open collection as a whole: the one-way sync's target.
    Collection,
}

/// One ordinary call, with the transports that may make it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ordinary {
    /// The backend service's index, as the engine numbers it.
    pub service: u32,
    /// The method's index within that service.
    pub method: u32,
    /// The engine's name for the call, `Service.Method`.
    pub name: &'static str,
    /// Whether the native adapter may make it.
    pub native: bool,
    /// Whether the web engine may make it.
    pub web: bool,
}

impl Ordinary {
    /// Whether `transport` may make this call.
    #[must_use]
    pub fn admits(&self, transport: Transport) -> bool {
        match transport {
            Transport::Native => self.native,
            Transport::Web => self.web,
        }
    }
}

/// One exempt write: its pair, the engine's name for it and the kind of target it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exempt {
    /// The write the owner taps.
    pub write: ExemptWrite,
    /// The backend service's index, as the engine numbers it.
    pub service: u32,
    /// The method's index within that service.
    pub method: u32,
    /// The engine's name for the call, `Service.Method`.
    pub name: &'static str,
    /// The kind of the one target the write takes.
    pub kind: TargetKind,
}

impl Exempt {
    /// Whether this row is the write at `service` and `method`.
    #[must_use]
    pub fn is(&self, service: u32, method: u32) -> bool {
        self.service == service && self.method == method
    }
}

/// The ordinary calls, each marked with the transports that may make it.
///
/// The native column is the native adapter's allow-list and the web column is the web engine's
/// study calls with its sync calls (SPEC-345 M1, M4; SPEC-364 R1); a parity test holds each
/// adapter's own table equal to its column.
/// A pair one transport may make is not thereby admitted on the other: the native client neither
/// closes the collection nor adds notes through this table (ADR-356 D2).
pub const ORDINARY: [Ordinary; 18] = [
    Ordinary {
        service: 1,
        method: 3,
        name: "BackendSyncService.SyncLogin",
        native: true,
        web: true,
    },
    Ordinary {
        service: 1,
        method: 5,
        name: "BackendSyncService.SyncCollection",
        native: false,
        web: true,
    },
    Ordinary {
        service: 3,
        method: 0,
        name: "BackendCollectionService.OpenCollection",
        native: true,
        web: true,
    },
    Ordinary {
        service: 3,
        method: 1,
        name: "BackendCollectionService.CloseCollection",
        native: false,
        web: true,
    },
    Ordinary {
        service: 3,
        method: 7,
        name: "CollectionService.GetUndoStatus",
        native: false,
        web: true,
    },
    Ordinary {
        service: 5,
        method: 4,
        name: "CardsService.SetFlag",
        native: true,
        web: true,
    },
    Ordinary {
        service: 7,
        method: 4,
        name: "DecksService.DeckTree",
        native: true,
        web: true,
    },
    Ordinary {
        service: 7,
        method: 13,
        name: "DecksService.GetDeckNames",
        native: true,
        web: false,
    },
    Ordinary {
        service: 7,
        method: 22,
        name: "DecksService.SetCurrentDeck",
        native: true,
        web: true,
    },
    Ordinary {
        service: 13,
        method: 3,
        name: "SchedulerService.GetQueuedCards",
        native: true,
        web: true,
    },
    Ordinary {
        service: 13,
        method: 14,
        name: "SchedulerService.BuryOrSuspendCards",
        native: true,
        web: true,
    },
    Ordinary {
        service: 13,
        method: 24,
        name: "SchedulerService.DescribeNextStates",
        native: true,
        web: true,
    },
    Ordinary {
        service: 23,
        method: 8,
        name: "NotetypesService.GetNotetypeNames",
        native: false,
        web: true,
    },
    Ordinary {
        service: 25,
        method: 0,
        name: "NotesService.NewNote",
        native: false,
        web: true,
    },
    Ordinary {
        service: 25,
        method: 2,
        name: "NotesService.AddNotes",
        native: false,
        web: true,
    },
    Ordinary {
        service: 27,
        method: 6,
        name: "CardRenderingService.RenderExistingCard",
        native: true,
        web: true,
    },
    Ordinary {
        service: 27,
        method: 9,
        name: "CardRenderingService.StripAvTags",
        native: false,
        web: true,
    },
    Ordinary {
        service: 27,
        method: 14,
        name: "CardRenderingService.HtmlToTextLine",
        native: false,
        web: true,
    },
];

/// The one call that records a grade, held for an owner's press (SPEC-365 R4; ADR-376 D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answered {
    /// The backend service's index, as the engine numbers it.
    pub service: u32,
    /// The method's index within that service.
    pub method: u32,
    /// The engine's name for the call, `Service.Method`.
    pub name: &'static str,
}

impl Answered {
    /// Whether this row is the call at `service` and `method`.
    #[must_use]
    pub fn is(&self, service: u32, method: u32) -> bool {
        self.service == service && self.method == method
    }
}

/// The calls that record a grade: `AnswerCard` alone. Neither transport makes it through `run`; the
/// dispatcher runs it only for an owner's answer, which names the card and the grade a press
/// named (SPEC-365 R3, R4).
pub const ANSWERED: [Answered; 1] = [Answered {
    service: 13,
    method: 4,
    name: "SchedulerService.AnswerCard",
}];

/// The exempt writes: the never-list's entries 2 (Forget), 6 (set due date), 3 (delete a preset),
/// 7 (change note type) and 8 (delete a card or a note), each one method with one target (SPEC-345
/// M8), the full-sync choice's one-way sync, whose one target is the open collection and which
/// runs only through the choice's own write, never through `run_exempt` (SPEC-364 R1, R3; ADR-375
/// D5), and the undo of the review's own last answer, on its card (SPEC-371 R2). The scheduler
/// switch and every other never-list method stay unlisted, so `run` refuses them as not allowed
/// (ADR-356 D6).
pub const EXEMPT: [Exempt; 8] = [
    Exempt {
        write: ExemptWrite::Forget,
        service: 13,
        method: 17,
        name: "SchedulerService.ScheduleCardsAsNew",
        kind: TargetKind::Card,
    },
    Exempt {
        write: ExemptWrite::SetDueDate,
        service: 13,
        method: 19,
        name: "SchedulerService.SetDueDate",
        kind: TargetKind::Card,
    },
    Exempt {
        write: ExemptWrite::DeletePreset,
        service: 11,
        method: 5,
        name: "DeckConfigService.RemoveDeckConfig",
        kind: TargetKind::Preset,
    },
    Exempt {
        write: ExemptWrite::ChangeNoteType,
        service: 23,
        method: 15,
        name: "NotetypesService.ChangeNotetype",
        kind: TargetKind::Note,
    },
    Exempt {
        write: ExemptWrite::DeleteCard,
        service: 5,
        method: 2,
        name: "CardsService.RemoveCards",
        kind: TargetKind::Card,
    },
    Exempt {
        write: ExemptWrite::DeleteNote,
        service: 25,
        method: 7,
        name: "NotesService.RemoveNotes",
        kind: TargetKind::Note,
    },
    Exempt {
        write: ExemptWrite::OneWaySync,
        service: 1,
        method: 6,
        name: "BackendSyncService.FullUploadOrDownload",
        kind: TargetKind::Collection,
    },
    Exempt {
        write: ExemptWrite::Undo,
        service: 3,
        method: 8,
        name: "CollectionService.Undo",
        kind: TargetKind::Card,
    },
];

/// What the table decides for `service` and `method` on `transport`: admitted when an ordinary
/// row holds the pair and marks the transport, held for an owner's answer when the answered row
/// holds it, held for a gesture when an exempt row holds it, and refused otherwise.
#[must_use]
pub fn decide(transport: Transport, service: u32, method: u32) -> Decision {
    let ordinary = ORDINARY
        .iter()
        .any(|row| (row.service, row.method) == (service, method) && row.admits(transport));
    if ordinary {
        Decision::Admit
    } else if ANSWERED.iter().any(|row| row.is(service, method)) {
        Decision::NeedsAnswer
    } else if EXEMPT.iter().any(|row| row.is(service, method)) {
        Decision::NeedsGesture
    } else {
        Decision::NotAllowed
    }
}
