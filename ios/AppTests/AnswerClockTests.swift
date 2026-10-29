// The answer clock's arithmetic over sequences of shows, opens, closes and grades (SPEC-507 R15).
// It is pure, with every instant handed in, so these tests need no fixture and no engine, and run
// in the app's own test step.
import Foundation
import XCTest

@testable import DeckStreak

final class AnswerClockTests: XCTestCase {
    /// One step of a sequence, at its second after the sequence's start.
    private enum Step {
        case show(TimeInterval)
        case open(TimeInterval)
        case close(TimeInterval)
        case grade(TimeInterval)
    }

    /// SPEC-507 B6 (R15): each grade sends the time since its card was shown less every moment an
    /// overlay held the clock, one still open at the grade included. A second open keeps the first
    /// one's instant, a close with none open changes nothing, an overlay open as a new card shows
    /// holds from that show on, the held time ends with its card, and the time never reads below 0.
    func test_the_answer_clock_excludes_every_overlays_time() {
        let sequences: [(name: String, steps: [Step], sent: [UInt32])] = [
            ("no overlay", [.show(0), .grade(12)], [12_000]),
            ("one overlay", [.show(0), .open(2), .close(7), .grade(10)], [5_000]),
            ("a second open", [.show(0), .open(2), .open(4), .close(7), .grade(10)], [5_000]),
            ("a close with none open", [.show(0), .close(3), .grade(10)], [10_000]),
            (
                "two overlays", [.show(0), .open(2), .close(4), .open(6), .close(9), .grade(12)],
                [7_000]
            ),
            ("open at the grade", [.show(0), .open(4), .grade(10)], [4_000]),
            (
                "open as a new card shows",
                [.show(0), .open(4), .show(20), .close(25), .grade(30)], [5_000]
            ),
            (
                "held time ends with its card",
                [.show(0), .open(1), .close(3), .grade(5), .show(10), .grade(15)], [3_000, 5_000]
            ),
            ("a clock that went back", [.show(10), .grade(5)], [0]),
        ]
        let start = Date(timeIntervalSince1970: 1_800_000_000)
        var read: [String] = []
        for sequence in sequences {
            var clock = AnswerClock(shownAt: start)
            var sent: [UInt32] = []
            for step in sequence.steps {
                switch step {
                case .show(let at): clock.shown(at: start.addingTimeInterval(at))
                case .open(let at): clock.pause(at: start.addingTimeInterval(at))
                case .close(let at): clock.resume(at: start.addingTimeInterval(at))
                case .grade(let at): sent.append(clock.taken(at: start.addingTimeInterval(at)))
                }
            }
            read.append("\(sequence.name): \(sent)")
        }
        XCTAssertEqual(
            read, sequences.map { "\($0.name): \($0.sent)" },
            "B6: the milliseconds each grade sends, over \(sequences.count) sequences")
    }
}
