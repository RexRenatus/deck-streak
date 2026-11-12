// SPEC-505 R7, R8, A4; ADR-523 D5. Each refusal the card view's factory can give reads a
// DeckStreak sentence, never the error's Swift case name, and the card face announces the words it
// shows to VoiceOver once. The announcement is handed a recording poster, so a test reads it.
import UIKit
import XCTest

@testable import DeckStreak

@MainActor
final class CardRefusalWordsTests: XCTestCase {
    /// Every way `words` fails a refusal: no words, or words that print the case's name.
    private func problems(of words: (Error) -> String) -> [String] {
        CardRefusalWords.refusals.flatMap { refusal -> [String] in
            let text = words(refusal)
            let name = String(describing: refusal)
            return (text.isEmpty ? ["\(name): no words"] : [])
                + (text.contains(name) ? ["\(name): prints its case name"] : [])
        }
    }

    func testEveryRefusalReadsADeckStreakSentenceNeverItsCaseName() {
        let refusals = CardRefusalWords.refusals
        print("examined \(refusals.count) refusals")
        XCTAssertFalse(refusals.isEmpty, "examined 0 refusals: nothing was judged")
        XCTAssertEqual(problems(of: CardRefusalWords.text(of:)), [])

        // an error the factory never throws reads the general sentence, not its own description
        struct Elsewhere: Error {}
        let general = CardRefusalWords.text(of: Elsewhere())
        XCTAssertFalse(general.isEmpty)
        XCTAssertFalse(general.contains("Elsewhere"))

        // the control: a mapper that prints the case, as the card face did, is refused for every case
        XCTAssertEqual(problems(of: { String(describing: $0) }).count, refusals.count)
    }

    func testARefusalIsAnnouncedOnceInTheWordsShown() {
        for refusal in CardRefusalWords.refusals {
            let shown = CardRefusalWords.text(of: refusal)
            var posted: [(UIAccessibility.Notification, String?)] = []
            CardRefusalWords.announce(shown) { notification, argument in
                posted.append((notification, argument as? String))
            }
            XCTAssertEqual(posted.count, 1, "\(refusal)")
            XCTAssertEqual(posted.first?.0, UIAccessibility.Notification.announcement, "\(refusal)")
            XCTAssertEqual(posted.first?.1, shown, "\(refusal)")
        }
    }
}
