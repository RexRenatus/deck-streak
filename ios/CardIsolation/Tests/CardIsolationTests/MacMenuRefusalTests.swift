// SPEC-679 section 9a: on macOS the card's web view holds every isolation layer the iOS view
// holds, its context menu included. A Mac web view opens a context menu of its own, which asks
// the UI delegate nothing, so the window refusal's macOS arm empties each menu before it opens.
// This runs on the macOS host, where the menu and the event that opens it are AppKit's.
#if os(macOS)
import AppKit
import WebKit
import XCTest

// Testable: the test reaches the factory's internal `make(layers:ruleList:)`, the probe's door.
@testable import CardIsolation

/// One view the factory makes, and how many items of an offered menu it lets open.
struct MenuRow {
    let name: String
    let layers: Set<CardLayer>
    let itemsLeft: Int
}

final class MacMenuRefusalTests: XCTestCase {
    /// Prints how many rows were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// The card view, every layer on, opens none of a menu's items; the same view with the window
    /// refusal off opens the menu as it was offered, so the refusal is the window refusal's.
    @MainActor
    func test_a_mac_card_view_empties_its_context_menu() throws {
        let rows = examined(
            "card views",
            [
                MenuRow(
                    name: "the card view, every layer on", layers: Set(CardLayer.allCases),
                    itemsLeft: 0),
                MenuRow(
                    name: "the card view with the window refusal off",
                    layers: Set(CardLayer.allCases).subtracting([.L6]), itemsLeft: 1),
            ])
        for row in rows {
            let view = CardWebViewFactory.make(layers: row.layers, ruleList: nil)
            let menu = NSMenu(title: "card")
            menu.addItem(NSMenuItem(title: "Open Link", action: nil, keyEquivalent: ""))
            let event = try XCTUnwrap(
                NSEvent.mouseEvent(
                    with: .rightMouseDown, location: .zero, modifierFlags: [], timestamp: 0,
                    windowNumber: 0, context: nil, eventNumber: 0, clickCount: 1, pressure: 1),
                "\(row.name): no right-click event was made")
            view.willOpenMenu(menu, with: event)
            XCTAssertEqual(
                menu.items.count, row.itemsLeft,
                "\(row.name) opens \(menu.items.count) of the menu's items")
        }
    }
}
#endif
