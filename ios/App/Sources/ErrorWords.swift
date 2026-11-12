/// The words a failure reads in the deck list and the account sheet (SPEC-680 R5).
enum ErrorWords {
    /// The app's sentence for a failure that brings no words of its own.
    static let general = "DeckStreak could not do that. Please try again."

    /// The words `error` reads.
    static func text(of error: any Error) -> String {
        String(describing: error)
    }
}
