import UIKit

/// Says each step of the review once to VoiceOver (SPEC-680 R6): the answer as it shows, the
/// grade sent, and after each next step the next card with the queue's counts, or the finish in
/// its words. It posts each sentence as an announcement through the poster it is handed, the
/// system's announcement unless a test hands in its own, the form of the card face's one door.
@MainActor
struct ReviewAnnouncer {
    let post: @MainActor (UIAccessibility.Notification, Any?) -> Void

    /// After Show Answer, once the answer shows.
    func answerShown() {
        say("Answer shown.")
    }

    /// After a grade is sent.
    func graded(_ rating: Rating) {
        say(rating == .again ? "Again." : "Good.")
    }

    /// After each next step: the next card with the queue's counts, or the finish's words after
    /// `answered` answers.
    func shown(_ step: ReviewStep, answered: Int) {
        switch step.phase {
        case .finished:
            say(FinishWords.text(answered: answered))
        default:
            say(
                "Next card: \(step.counts.new) new, \(step.counts.learning) learning, "
                    + "\(step.counts.review) to review.")
        }
    }

    private func say(_ sentence: String) {
        post(.announcement, sentence)
    }
}
