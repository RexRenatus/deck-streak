import Observation

/// The card info panel's state, held in a model as the review's is (SPEC-507 R14, R15): whether
/// the sheet is shown, and the shown card's sections as the engine's adapter words them. Opening
/// the sheet stops the answer clock before the card is read; closing it, by its button or by a
/// swipe, starts the clock again, and a second close finds it running and changes nothing.
@MainActor
@Observable
final class CardHealthModel {
    /// Whether the sheet is shown.
    var showing = false
    /// The shown card's sections, in the adapter's words; empty before a read.
    private(set) var sections: [CardInfoSection] = []

    private let session: ReviewSession

    init(session: ReviewSession) {
        self.session = session
    }

    /// Opens the sheet over the shown card: the answer clock stops first, then the card is read. A
    /// refusal shows its sentence as the sheet's one line and leaves the review where it is.
    func open() async {
        await session.pause()
        sections = []
        showing = true
        do {
            sections = try await session.health()
        } catch {
            sections = [CardInfoSection(title: "Card info", lines: [String(describing: error)])]
        }
    }

    /// Closes the sheet; the answer clock runs again.
    func close() async {
        showing = false
        await session.resume()
    }
}
