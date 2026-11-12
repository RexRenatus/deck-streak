// SPEC-680 R5. Every failure the deck list and the account sheet show reads words of the app's own:
// a refusal's sentence, which the engine words, or the app's general sentence, never an error's
// description. Every error judged is counted, and a planted control, a mapper that prints the
// description as the app did, is refused for every error but the refusal.
import Foundation
import XCTest

@testable import DeckStreak

/// An error from elsewhere, which brings no words of its own.
private struct ElsewhereError: Error {}

@MainActor
final class ErrorWordsTests: XCTestCase {
    /// The errors judged: a refusal, a file error, a domain error and one from elsewhere.
    private let errors: [any Error] = [
        Refusal(sentence: "The collection could not be opened."),
        CocoaError(.fileNoSuchFile),
        NSError(domain: "DeckStreakTests", code: 7),
        ElsewhereError(),
    ]

    /// Every error `words` fails, with why: no words, words other than a refusal's own sentence or
    /// the general sentence, or words that print the error's description.
    private func problems(of words: (any Error) -> String) -> [String] {
        var problems: [String] = []
        for error in errors {
            let text = words(error)
            let description = String(describing: error)
            let refusal = error as? Refusal
            var reasons: [String] = []
            if text.isEmpty {
                reasons.append("no words")
            }
            if text != (refusal?.sentence ?? ErrorWords.general) {
                reasons.append("reads other words")
            }
            if refusal == nil, text.contains(description) {
                reasons.append("prints its description")
            }
            if !reasons.isEmpty {
                problems.append(description + ": " + reasons.joined(separator: ", "))
            }
        }
        return problems
    }

    func test_every_app_error_reads_words_of_its_own() {
        print("examined \(errors.count) errors")
        XCTAssertEqual(problems(of: ErrorWords.text(of:)), [])
        XCTAssertFalse(errors.isEmpty, "examined 0 errors: nothing was judged")

        // the control: a mapper that prints the description, as the app did, is refused for every
        // error but the refusal, whose description is its sentence
        XCTAssertEqual(
            problems(of: { error in String(describing: error) }).count, errors.count - 1)
    }
}
