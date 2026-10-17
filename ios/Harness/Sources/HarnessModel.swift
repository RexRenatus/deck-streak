import HarnessWire
import Observation

/// The review session's state, held in a model and never in a view's state, so a size-class change
/// or a scene reconnect keeps it (SPEC-339 R8). It lives on the main actor and awaits the session,
/// which makes every engine call off it.
@MainActor
@Observable
final class HarnessModel {
    /// `opened` once the collection is open, or the open's refusal sentence.
    var openStatus = ""
    var deckNames: [String] = []
    /// The deck the learner chose; the study pane studies the collection's current deck.
    var chosenDeck: String?
    /// The question's text nodes, joined, as the card's HTML.
    var questionHTML = ""
    /// The deck's new count after the last study step.
    var newCount = ""
    /// The last refusal, as its own sentence.
    var refusal: String?

    private let session = EngineSession()
    private var started = false

    /// Opens the collection and lists its decks, once per model (R3, R4).
    func start() async {
        guard !started else { return }
        started = true
        do {
            try await session.open()
            openStatus = "opened"
            deckNames = try await session.deckNames()
        } catch {
            openStatus = Refusal.sentence(for: error)
        }
    }

    /// Shows the current deck's next card (R5).
    func study() async {
        do {
            show(try await session.next())
        } catch {
            refusal = Refusal.sentence(for: error)
        }
    }

    /// Answers the shown card Good and shows the queue after it (R7).
    func answerGood() async {
        do {
            show(try await session.answerGood())
        } catch {
            refusal = Refusal.sentence(for: error)
        }
    }

    private func show(_ studied: Studied) {
        newCount = String(studied.newCount)
        refusal = nil
        var text: [String] = []
        for node in studied.question {
            switch node {
            case .text(let part):
                text.append(part)
            case .replacement(let field):
                // A replacement is a field the engine left for the client to fill. The harness
                // fills none, so the card is refused in a sentence, never shown as raw text.
                questionHTML = ""
                refusal = "The card's question needs the client to fill the field \(field), which the harness does not do."
                return
            }
        }
        questionHTML = text.joined()
    }
}
