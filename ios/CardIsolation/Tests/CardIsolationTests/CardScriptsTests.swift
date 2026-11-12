// SPEC-361 A1 (SPEC-355 A1's test, kept): the card-script switch's decision, which is pure, over
// every case: the switch on or off, times every subset of the eleven controls a scripted card view
// must carry. It runs on the macOS host with no WebKit.
import XCTest

import CardIsolation

final class CardScriptsTests: XCTestCase {
    /// Prints how many cases were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// The controls a scripted card view must carry (SPEC-361 R2), named here rather than read
    /// from the code under test: every layer but L2, the verdict itself, and L14, no control. L9 is retired.
    static let controls: [CardLayer] = [.L1, .L3, .L4, .L5, .L6, .L7, .L8, .L10, .L11, .L12, .L13]

    func test_scripts_run_only_when_switched_on_with_every_control_present() {
        let controls = Self.controls
        var cases: [(switchedOn: Bool, present: Set<CardLayer>)] = []
        for switchedOn in [false, true] {
            for mask in 0..<(1 << controls.count) {
                let present = Set(controls.indices.filter { mask & (1 << $0) != 0 }.map { controls[$0] })
                cases.append((switchedOn: switchedOn, present: present))
            }
        }
        // The behaviour first: scripts run in exactly one case, and every other case names the
        // controls it lacks.
        for row in examined("cases", cases) {
            let missing = Set(controls).subtracting(row.present)
            let wanted: CardScripts.Verdict =
                row.switchedOn && missing.isEmpty ? .run : .off(missing: missing)
            XCTAssertEqual(
                CardScripts.decide(switchedOn: row.switchedOn, present: row.present), wanted,
                "switch \(row.switchedOn ? "on" : "off"), present \(row.present.map(\.rawValue).sorted())")
        }
        XCTAssertEqual(cases.count, 4096, "the cases are not the switch times every subset of eleven")
        XCTAssertEqual(
            CardScripts.required, Set(controls), "the required controls are not L1, L3 to L8 and L10 to L13")
    }
}
