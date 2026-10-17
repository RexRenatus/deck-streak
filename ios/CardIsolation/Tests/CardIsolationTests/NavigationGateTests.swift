// SPEC-349 A1: the navigation gate allows only the first main-frame load, and seals. The gate's
// input is a pure value, so this runs on the macOS host with no WebKit.
import XCTest

import CardIsolation

/// One row of the gate's table: the actions in order, the decisions the gate must give them, and
/// the state it must end in.
struct GateRow {
    let name: String
    let actions: [NavigationRequest]
    let wanted: [NavigationDecision]
    let state: NavigationGate.State
}

final class NavigationGateTests: XCTestCase {
    /// Prints how many rows were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    func test_only_the_first_main_frame_load_is_allowed() {
        let first = NavigationRequest(isMainFrame: true, kind: NavigationKind.other)
        var rows: [GateRow] = [
            GateRow(
                name: "the first main-frame load",
                actions: [first],
                wanted: [NavigationDecision.allow],
                state: NavigationGate.State.sealed),
            GateRow(
                name: "ten more loads after the first",
                actions: [first] + Array(repeating: first, count: 10),
                wanted: [NavigationDecision.allow] + Array(repeating: NavigationDecision.cancel, count: 10),
                state: NavigationGate.State.sealed),
        ]
        for kind in NavigationKind.allCases {
            for isMainFrame in [true, false] {
                let frame = isMainFrame ? "the main frame" : "a subframe"
                rows.append(GateRow(
                    name: "\(kind) in \(frame) after the first load",
                    actions: [first, NavigationRequest(isMainFrame: isMainFrame, kind: kind)],
                    wanted: [NavigationDecision.allow, NavigationDecision.cancel],
                    state: NavigationGate.State.sealed))
            }
            rows.append(GateRow(
                name: "\(kind) in a subframe before the first load",
                actions: [NavigationRequest(isMainFrame: false, kind: kind), first],
                wanted: [NavigationDecision.cancel, NavigationDecision.allow],
                state: NavigationGate.State.sealed))
            if kind != NavigationKind.other {
                rows.append(GateRow(
                    name: "\(kind) in the main frame before the first load",
                    actions: [NavigationRequest(isMainFrame: true, kind: kind)],
                    wanted: [NavigationDecision.cancel],
                    state: NavigationGate.State.awaitingFirstLoad))
            }
        }
        for row in examined("gate rows", rows) {
            var gate = NavigationGate()
            var decided: [NavigationDecision] = []
            for action in row.actions {
                decided.append(gate.decide(action))
            }
            XCTAssertEqual(decided, row.wanted, row.name)
            XCTAssertEqual(gate.state, row.state, row.name)
        }
    }
}
