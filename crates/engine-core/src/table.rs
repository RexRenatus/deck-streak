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
pub const ORDINARY: [Ordinary; 0] = [];

/// The exempt writes.
pub const EXEMPT: [Exempt; 0] = [];

/// What the table decides for `service` and `method` on `transport`.
#[must_use]
pub fn decide(_transport: Transport, _service: u32, _method: u32) -> Decision {
    Decision::NotAllowed
}
