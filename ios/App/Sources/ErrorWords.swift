/// The words a failure reads in the deck list and the account sheet (SPEC-680 R5).
enum ErrorWords {
    /// The app's sentence for a failure that brings no words of its own.
    static let general = "DeckStreak could not do that. Please try again."

    /// The words `error` reads: a refusal's own sentence, which the engine words, or the app's
    /// general sentence, never the error's description.
    static func text(of error: any Error) -> String {
        (error as? Refusal)?.sentence ?? general
    }
}
