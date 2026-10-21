// SPEC-361 A2: the link-activation refusal (L10) cancels the default of every click and auxclick
// whose composed path holds a link or whose target can host a shadow root, registers one listener
// per event in the capture phase, never stops propagation, and leaves every other click alone. Its
// source runs in a JavaScriptCore context on the macOS host, over a window, elements and events the
// test plants: no WebKit, so nothing exists unless planted.
import Foundation
import JavaScriptCore
import XCTest

import CardIsolation

final class LinkActivationRefusalTests: XCTestCase {
    /// Prints how many planted clicks were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    static let html = "http://www.w3.org/1999/xhtml"
    static let svg = "http://www.w3.org/2000/svg"
    static let mathml = "http://www.w3.org/1998/Math/MathML"

    /// One planted click: the target first, then the elements above it up to the card's own
    /// elements (each `[namespace, local name, attribute...]`), and whether the refusal must
    /// cancel its default. Named here rather than read from the code under test (SPEC-361 R3).
    struct Click {
        let name: String
        let path: [[String]]
        let refused: Bool
    }

    static let clicks: [Click] = [
        Click(name: "a link with an href", path: [[html, "a", "href"]], refused: true),
        Click(
            name: "an inline element inside a link",
            path: [[html, "b"], [html, "a", "href"]], refused: true),
        Click(name: "an area with an href", path: [[html, "area", "href"]], refused: true),
        Click(name: "an SVG link", path: [[svg, "a"], [svg, "svg"]], refused: true),
        Click(
            name: "a custom element hosting a closed shadow root",
            path: [[html, "card-face"]], refused: true),
        Click(name: "a span", path: [[html, "span"], [html, "label"]], refused: true),
        Click(name: "a div", path: [[html, "div"]], refused: true),
        Click(name: "a section", path: [[html, "section"]], refused: true),
        Click(name: "a link without an href", path: [[html, "a"]], refused: false),
        Click(name: "a button", path: [[html, "button"]], refused: false),
        Click(name: "a summary", path: [[html, "summary"], [html, "details"]], refused: false),
        Click(name: "an input", path: [[html, "input"], [html, "label"]], refused: false),
        Click(
            name: "a hyphenated element outside HTML",
            path: [[mathml, "annotation-xml"], [mathml, "math"]], refused: false),
    ]

    /// The planted window and the dispatcher. The window records every listener added to it;
    /// `click` builds one event over a planted path, runs every listener registered for its type,
    /// and reports how often the event's default was cancelled and its propagation stopped.
    static let harness = """
        globalThis.registered = [];
        globalThis.window = {
          addEventListener(type, listener, options) {
            const capture = options === true
              || (typeof options === 'object' && options !== null && options.capture === true);
            registered.push({ type, listener, capture });
          },
        };
        globalThis.click = (type, path) => {
          const element = ([namespace, name, ...attributes]) => ({
            nodeType: 1,
            namespaceURI: namespace,
            localName: name,
            hasAttribute: (key) => attributes.includes(key),
          });
          const html = 'http://www.w3.org/1999/xhtml';
          const above = [element([html, 'body']), element([html, 'html']), { nodeType: 9 }, {}];
          const nodes = path.map(element).concat(above);
          const seen = { prevented: 0, stopped: 0 };
          const event = {
            type,
            target: nodes[0],
            composedPath: () => nodes.slice(),
            preventDefault: () => { seen.prevented += 1; },
            stopPropagation: () => { seen.stopped += 1; },
            stopImmediatePropagation: () => { seen.stopped += 1; },
          };
          for (const entry of registered) {
            if (entry.type === type) {
              entry.listener.call(window, event);
            }
          }
          return JSON.stringify(seen);
        };
        """

    func test_the_refusal_cancels_every_link_activation_and_nothing_else() throws {
        let context = try XCTUnwrap(JSContext(), "no JavaScript context")
        context.evaluateScript(Self.harness)
        XCTAssertNil(context.exception, "planting the window threw")

        context.evaluateScript(LinkActivationRefusal.source)
        XCTAssertNil(context.exception, "the refusal threw: \(String(describing: context.exception))")

        let dispatch = try XCTUnwrap(context.objectForKeyedSubscript("click"), "no planted dispatcher")
        var cancelled: [String: Set<String>] = ["click": [], "auxclick": []]
        var stopped: [String] = []
        for click in examined("planted clicks", Self.clicks) {
            for type in ["click", "auxclick"] {
                let read = dispatch.call(withArguments: [type, click.path])
                XCTAssertNil(context.exception, "\(click.name), \(type): the listener threw")
                let text = try XCTUnwrap(read?.toString(), "\(click.name), \(type): nothing was read")
                let seen = try XCTUnwrap(
                    try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Int],
                    "\(click.name), \(type): not a reading: \(text)")
                if (seen["prevented"] ?? 0) > 0 {
                    cancelled[type, default: []].insert(click.name)
                }
                if (seen["stopped"] ?? 0) > 0 {
                    stopped.append("\(click.name), \(type)")
                }
            }
        }

        // The behaviour first: the default is cancelled exactly for the planted links and the
        // targets that can host a shadow root, for both events, and propagation is never stopped.
        let wanted = Set(Self.clicks.filter(\.refused).map(\.name))
        XCTAssertEqual(cancelled["click"], wanted, "the clicks cancelled are not the links and the hosts")
        XCTAssertEqual(cancelled["auxclick"], wanted, "the auxclicks cancelled are not the links and the hosts")
        XCTAssertEqual(stopped, [], "the refusal stopped a card's own listeners from running")

        // One listener per event, each in the capture phase.
        let registered = context.evaluateScript(
            "JSON.stringify(registered.map((entry) => [entry.type, entry.capture ? 'capture' : 'bubble']))")
        let text = try XCTUnwrap(registered?.toString(), "the registrations were not read")
        let pairs = try XCTUnwrap(
            try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [[String]],
            "the registrations are not a list: \(text)")
        XCTAssertEqual(
            pairs.sorted { $0.joined() < $1.joined() }, [["auxclick", "capture"], ["click", "capture"]],
            "the refusal is not one capture-phase listener for click and one for auxclick")
    }
}
