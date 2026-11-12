/// The words the review shows at its finish (SPEC-680 R3).
enum FinishWords {
    /// The finish's words after the review sent `answered` answers.
    static func text(answered: Int) -> String {
        "Congratulations! You have finished this deck for now."
    }
}
