import Foundation

/// The answer clock of the shown card (SPEC-507 R15): the time a grade sends is the time since the
/// card's question was shown, less every moment an overlay held the clock, one still open at the
/// grade included. The session holds one and hands it each instant from its own clock, so a test
/// decides every instant.
struct AnswerClock: Equatable, Sendable {
    /// When the shown card's question was shown.
    private(set) var shownAt: Date
    /// The time closed overlays held the clock since then.
    private(set) var held: TimeInterval = 0
    /// When the overlay now open opened, or nil while none is.
    private(set) var openedAt: Date?

    init(shownAt: Date) {
        self.shownAt = shownAt
    }

    /// A new card is shown at `instant`: its time starts there with nothing held, and an overlay
    /// still open holds the clock from `instant` on.
    mutating func shown(at instant: Date) {
        shownAt = instant
        held = 0
        openedAt = openedAt.map { _ in instant }
    }

    /// An overlay opens at `instant`; one already open keeps the instant it opened at.
    mutating func pause(at instant: Date) {
        openedAt = openedAt ?? instant
    }

    /// The open overlay closes at `instant` and its time is held; with none open, nothing changes.
    mutating func resume(at instant: Date) {
        held += holding(at: instant)
        openedAt = nil
    }

    /// The milliseconds a grade at `instant` sends: the time since the card was shown less every
    /// moment an overlay held the clock, never below 0.
    func taken(at instant: Date) -> UInt32 {
        let seconds = instant.timeIntervalSince(shownAt) - held - holding(at: instant)
        return UInt32(clamping: Int64(seconds * 1000))
    }

    /// How long the overlay now open has held the clock at `instant`; 0 while none is open.
    private func holding(at instant: Date) -> TimeInterval {
        openedAt.map { instant.timeIntervalSince($0) } ?? 0
    }
}
