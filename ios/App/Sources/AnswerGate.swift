import Observation

/// Holds a shown answer's two grades closed for a settle interval, so a press that lands as Show
/// Answer turns into Again and Good is not sent (SPEC-680 R4). An answer shown closes the gate and
/// a wait of the interval opens it, unless another answer has shown since; a zero interval opens
/// it at once. The bar shows the grades disabled while it is closed.
@MainActor
@Observable
final class AnswerGate {
    /// The app's settle interval after an answer shows.
    static let settle = Duration.milliseconds(350)

    /// Whether a grade may be sent.
    private(set) var isOpen = true

    /// The answers shown so far; a wait opens the gate only for the answer it was started for.
    private var shownCount = 0
    private let interval: Duration

    init(interval: Duration) {
        self.interval = interval
    }

    /// Closes the grades as an answer shows, and opens them once the interval has passed.
    func answerShown() {
        shownCount += 1
        isOpen = interval == .zero
        let shown = shownCount
        let interval = interval
        Task { [weak self] in
            try? await Task.sleep(for: interval)
            self?.opened(after: shown)
        }
    }

    /// Whether the bar offers the grades at `phase`: only on a shown answer, once the gate opens.
    func allows(_ phase: ReviewPhase) -> Bool {
        phase == .answer && isOpen
    }

    private func opened(after shown: Int) {
        if shownCount == shown {
            isOpen = true
        }
    }
}
