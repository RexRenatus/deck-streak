import Observation

/// The undo's offer as the review's dialog shows it (SPEC-515 R3): the card and the step the
/// confirmation names, and the adapter's words, each shown as text as it is.
struct UndoDialog: Identifiable, Equatable, Sendable {
    let card: Int64
    let step: UInt32
    let title: String
    let lines: [String]
    let keep: String
    let confirm: String

    /// One offer per card and step, so a sheet shown for one is never reused for the next.
    var id: String { "\(card)-\(step)" }
}

/// The review's undo of its own last answer, bury or flag (SPEC-515 R6): the control's label, the
/// offer its dialog shows, and the adapter's sentence for an undo it did not offer or make. A press
/// asks the adapter for the offer and writes nothing; Keep drops it; only the confirmation runs the
/// undo, which the core judges at the write, and then the review shows the next queued card. Each
/// gesture writes its state before its first `await`, so a second confirmation finds no offer.
@MainActor
@Observable
final class ReviewUndo {
    /// The confirmation's title of the step the adapter would undo now, or empty: the control is
    /// enabled only while it names one, and reads it as its accessibility value.
    private(set) var label = ""
    /// The offer the dialog shows, until Keep or the confirmation; the dialog's dismissal drops it.
    var offer: UndoDialog?
    /// The adapter's sentence for an undo it did not offer or did not make, at most one.
    private(set) var said: [String] = []

    private let session: ReviewSession

    init(session: ReviewSession) {
        self.session = session
    }

    /// Reads what the adapter would undo now, after the review opens and after every answer, bury,
    /// flag and undo.
    func read(_ available: UndoDialog?) {
        label = available?.confirm ?? ""
    }

    /// The undo's three gestures, from the review model's one entry: the press asks for the offer
    /// from a card's question or answer, Keep drops it, and the confirmation of the offer shown
    /// runs the undo and then `reload` shows the next queued card. Every other gesture is not one.
    func perform(
        _ action: ReviewAction, from phase: ReviewPhase, reload: () async throws -> Void
    ) async throws {
        switch (action, offer) {
        case (.undo, nil) where phase == .question || phase == .answer:
            let asked = try await session.undoOffer()
            offer = asked.dialog
            say(asked.said)
        case (.keep, _):
            offer = nil
        case (.confirmUndo, let confirmed?):
            offer = nil
            let refused = try await session.undo(confirmed)
            try await reload()
            say(refused)
        default:
            break
        }
    }

    /// Shows the adapter's sentence, or nothing when it said none.
    private func say(_ sentence: String) {
        said = [sentence].filter { !$0.isEmpty }
    }
}
