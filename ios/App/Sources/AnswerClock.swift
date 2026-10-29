import Foundation

/// The answer clock of the shown card (SPEC-507 R15). A stub: it counts from the card's show and
/// holds no overlay's time.
struct AnswerClock: Equatable, Sendable {
    /// When the shown card's question was shown.
    private(set) var shownAt: Date

    init(shownAt: Date) {
        self.shownAt = shownAt
    }

    /// A new card is shown at `instant`.
    mutating func shown(at instant: Date) {
        shownAt = instant
    }

    /// An overlay opens at `instant`: the stub holds nothing.
    mutating func pause(at instant: Date) {}

    /// The open overlay closes at `instant`: the stub holds nothing.
    mutating func resume(at instant: Date) {}

    /// The milliseconds a grade at `instant` sends: the stub's time since the card was shown.
    func taken(at instant: Date) -> UInt32 {
        UInt32(clamping: Int64(instant.timeIntervalSince(shownAt) * 1000))
    }
}
