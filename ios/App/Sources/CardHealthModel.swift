import Observation

/// The card info panel's state (SPEC-507 R14, R15). A stub: it shows and hides the sheet, reads
/// nothing and leaves the answer clock as it is.
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

    /// Opens the sheet: the stub reads nothing.
    func open() async {
        showing = true
    }

    /// Closes the sheet.
    func close() async {
        showing = false
    }
}
