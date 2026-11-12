// SPEC-680 R3. Each finish of a review reads words that name what was done: the answers the review
// sent, or that nothing was due, and no two finishes read the same words. Every finish judged is
// counted, and a planted control, one sentence for every finish, is refused for each.
import XCTest

@testable import DeckStreak

@MainActor
final class FinishWordsTests: XCTestCase {
    /// The finishes judged: no answer sent, one, two and a whole deck's.
    private let finishes = [0, 1, 2, 7]

    /// Every finish `words` fails, with why: no words, words that do not name the answers sent, or
    /// words another finish reads too.
    private func problems(of words: (Int) -> String) -> [String] {
        let texts = finishes.map(words)
        var problems: [String] = []
        for (answered, text) in zip(finishes, texts) {
            var reasons: [String] = []
            if text.isEmpty {
                reasons.append("no words")
            }
            if answered > 0, !text.contains(String(answered)) {
                reasons.append("names no count")
            }
            if texts.filter({ other in other == text }).count > 1 {
                reasons.append("reads another finish's words")
            }
            if !reasons.isEmpty {
                problems.append("\(answered): " + reasons.joined(separator: ", "))
            }
        }
        return problems
    }

    func test_each_finish_reads_words_that_name_what_was_done() {
        print("examined \(finishes.count) finishes")
        XCTAssertEqual(problems(of: FinishWords.text(answered:)), [])
        XCTAssertFalse(finishes.isEmpty, "examined 0 finishes: nothing was judged")

        // the control: one sentence for every finish, as the review showed, is refused for each
        XCTAssertEqual(
            problems(of: { _ in "Congratulations! You have finished this deck for now." }).count,
            finishes.count)
    }
}
