/// The words the review shows at its finish (SPEC-680 R3): how many answers the review sent, or
/// that nothing was due, so no two finishes read the same words.
enum FinishWords {
    /// The finish's words after the review sent `answered` answers.
    static func text(answered: Int) -> String {
        switch answered {
        case 0:
            return "Nothing was due in this deck. You are done for now."
        case 1:
            return "You answered 1 card. This deck is done for now."
        default:
            return "You answered \(answered) cards. This deck is done for now."
        }
    }
}
