//! The calls a native client may make (SPEC-336 R2).
//!
//! The engine numbers its backend services and their methods when it is built, and its own
//! clients address a call by that pair. A backend service answers its own methods and, after them,
//! the methods of the collection service it fronts, so `Undo` is reached through the backend
//! collection service and `RenderExistingCard` through the backend card rendering service. The
//! table below holds the pairs this adapter lets through, each with the engine's name for it, so a
//! reader can check a number against the engine's generated dispatch.
//! A pair the engine renumbers fails the round-trip and render tests, which call each entry and
//! decode its answer.

/// One engine call a native client may make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    /// The backend service's index, as the engine numbers it.
    pub service: u32,
    /// The method's index within that service.
    pub method: u32,
    /// The engine's name for the call, `Service.Method`, for a reader and a refusal's text.
    pub name: &'static str,
}

/// The allow-list: open a collection, list its decks, get the next card, answer it, undo,
/// render a card (SPEC-339 R10), a call that reads the card and writes nothing, and log in to a
/// sync server (SPEC-347 R1), whose endpoint the core guards before the engine sees it.
pub const ALLOW_LIST: [Call; 7] = [
    Call {
        service: 3,
        method: 0,
        name: "BackendCollectionService.OpenCollection",
    },
    Call {
        service: 7,
        method: 13,
        name: "DecksService.GetDeckNames",
    },
    Call {
        service: 13,
        method: 3,
        name: "SchedulerService.GetQueuedCards",
    },
    Call {
        service: 13,
        method: 4,
        name: "SchedulerService.AnswerCard",
    },
    Call {
        service: 3,
        method: 8,
        name: "CollectionService.Undo",
    },
    Call {
        service: 27,
        method: 6,
        name: "CardRenderingService.RenderExistingCard",
    },
    Call {
        service: 1,
        method: 3,
        name: "BackendSyncService.SyncLogin",
    },
];

/// The allow-list's entry for a call, or `None` when the call is not on it.
#[must_use]
pub fn allowed(service: u32, method: u32) -> Option<&'static Call> {
    ALLOW_LIST
        .iter()
        .find(|call| call.service == service && call.method == method)
}
