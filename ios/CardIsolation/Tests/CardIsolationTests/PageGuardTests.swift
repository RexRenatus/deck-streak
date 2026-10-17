// SPEC-361 A3: the page guard (L11) refuses a detached node's `click()` and `dispatchEvent`, refuses
// every `open`, `write` and `writeln` of a document, passes a connected node's calls through, calls
// only what it captured when it was installed, and a card can neither redefine nor reassign nor
// delete any wrapper. Its source runs in a JavaScriptCore context on the macOS host, over
// prototypes the test plants with recording methods: no WebKit, so nothing exists unless planted.
import Foundation
import JavaScriptCore
import XCTest

import CardIsolation

final class PageGuardTests: XCTestCase {
    /// Prints how many planted calls were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// The planted platform: `EventTarget`, `Node` (whose `isConnected` getter throws for a target
    /// that is no node, as the engine's does), `HTMLElement` and `Document`, each method recording
    /// that it ran, and a `DOMException` that carries its name. Then the planted targets.
    static let planted = """
        globalThis.calls = [];
        globalThis.DOMException = class DOMException extends Error {
          constructor(message, name) {
            super(message);
            this.name = name;
          }
        };
        globalThis.EventTarget = function EventTarget() {};
        EventTarget.prototype.dispatchEvent = function (event) {
          calls.push('dispatchEvent ' + this.label);
          return true;
        };
        globalThis.Node = function Node() {};
        Node.prototype = Object.create(EventTarget.prototype);
        Object.defineProperty(Node.prototype, 'isConnected', {
          get() {
            if (!Object.prototype.isPrototypeOf.call(Node.prototype, this)) {
              throw new TypeError('not a node');
            }
            return this.connected;
          },
          configurable: true,
        });
        globalThis.HTMLElement = function HTMLElement() {};
        HTMLElement.prototype = Object.create(Node.prototype);
        HTMLElement.prototype.click = function () {
          calls.push('click ' + this.label);
        };
        globalThis.Document = function Document() {};
        Document.prototype = Object.create(Node.prototype);
        for (const name of ['open', 'write', 'writeln']) {
          Document.prototype[name] = function () {
            calls.push(name + ' ' + this.label);
          };
        }
        const made = (prototype, label, connected) => {
          const target = Object.create(prototype);
          target.label = label;
          target.connected = connected;
          return target;
        };
        globalThis.connectedLink = made(HTMLElement.prototype, 'connected', true);
        globalThis.detachedLink = made(HTMLElement.prototype, 'detached', false);
        globalThis.cardDocument = made(Document.prototype, 'document', true);
        globalThis.cardWindow = made(EventTarget.prototype, 'window', undefined);
        globalThis.attempt = (body) => {
          calls.length = 0;
          let threw = null;
          try {
            body();
          } catch (error) {
            threw = error.name;
          }
          return JSON.stringify({ threw, reached: calls.slice() });
        };
        """

    /// One planted call a card makes after the guard is installed, what it must throw (nil for
    /// nothing) and which planted originals it must reach. Named here rather than read from the
    /// code under test (SPEC-361 R4). The tampering calls come last: each leaves the platform
    /// changed.
    struct Call {
        let name: String
        let body: String
        let threw: String?
        let reached: [String]
    }

    /// The five wrappers the guard installs, each as `[owner, name]`.
    static let wrappers: [[String]] = [
        ["HTMLElement.prototype", "click"], ["EventTarget.prototype", "dispatchEvent"],
        ["Document.prototype", "open"], ["Document.prototype", "write"], ["Document.prototype", "writeln"],
    ]

    static let calls: [Call] = {
        var calls: [Call] = [
            Call(name: "a detached click", body: "detachedLink.click()", threw: "NotAllowedError", reached: []),
            Call(
                name: "a detached dispatch", body: "detachedLink.dispatchEvent({ type: 'click' })",
                threw: "NotAllowedError", reached: []),
            Call(
                name: "a connected click", body: "connectedLink.click()", threw: nil,
                reached: ["click connected"]),
            Call(
                name: "a connected dispatch", body: "connectedLink.dispatchEvent({ type: 'click' })",
                threw: nil, reached: ["dispatchEvent connected"]),
            Call(
                name: "a dispatch on a target that is no node",
                body: "cardWindow.dispatchEvent({ type: 'load' })", threw: nil,
                reached: ["dispatchEvent window"]),
            Call(name: "document.open", body: "cardDocument.open()", threw: "NotAllowedError", reached: []),
            Call(
                name: "document.write", body: "cardDocument.write('<a>')", threw: "NotAllowedError",
                reached: []),
            Call(
                name: "document.writeln", body: "cardDocument.writeln('<a>')", threw: "NotAllowedError",
                reached: []),
            Call(
                name: "a connected click after the card replaced Reflect.apply",
                body: "Reflect.apply = () => calls.push('replaced'); connectedLink.click()",
                threw: nil, reached: ["click connected"]),
            Call(
                name: "a detached click after the card replaced the isConnected getter",
                body: "Object.defineProperty(Node.prototype, 'isConnected', { get: () => true }); "
                    + "detachedLink.click()",
                threw: "NotAllowedError", reached: []),
        ]
        for wrapper in PageGuardTests.wrappers {
            let owner = wrapper[0]
            let method = "\(owner)['\(wrapper[1])']"
            calls.append(
                Call(
                    name: "\(owner).\(wrapper[1]) redefined",
                    body: "Object.defineProperty(\(owner), '\(wrapper[1])', { value: () => calls.push('replaced') })",
                    threw: "TypeError", reached: []))
            calls.append(
                Call(
                    name: "\(owner).\(wrapper[1]) reassigned",
                    body: "const planted = () => {}; \(method) = planted; "
                        + "if (\(method) === planted) { calls.push('reassigned'); }",
                    threw: nil, reached: []))
            calls.append(
                Call(
                    name: "\(owner).\(wrapper[1]) deleted",
                    body: "delete \(method); "
                        + "if (!Object.prototype.hasOwnProperty.call(\(owner), '\(wrapper[1])')) { calls.push('deleted'); }",
                    threw: nil, reached: []))
        }
        return calls
    }()

    func test_the_guard_refuses_detached_activation_and_every_document_rewrite() throws {
        let context = try XCTUnwrap(JSContext(), "no JavaScript context")
        context.evaluateScript(Self.planted)
        XCTAssertNil(context.exception, "planting the platform threw")

        context.evaluateScript(PageGuard.source)
        XCTAssertNil(context.exception, "the guard threw: \(String(describing: context.exception))")

        let attempt = try XCTUnwrap(context.objectForKeyedSubscript("attempt"), "no planted attempt")
        var wrong: [String] = []
        for call in examined("planted calls", Self.calls) {
            let body = try XCTUnwrap(
                context.evaluateScript("(() => { \(call.body); })"), "\(call.name): the call did not parse")
            XCTAssertNil(context.exception, "\(call.name): the call did not parse")
            let read = attempt.call(withArguments: [body])
            let text = try XCTUnwrap(read?.toString(), "\(call.name): nothing was read")
            let seen = try XCTUnwrap(
                try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any],
                "\(call.name): not a reading: \(text)")
            let threw = seen["threw"] as? String
            let reached = seen["reached"] as? [String] ?? ["unreadable"]
            if threw != call.threw || reached != call.reached {
                wrong.append(
                    "\(call.name): threw \(threw ?? "nothing"), reached \(reached); "
                        + "wanted \(call.threw ?? "nothing"), \(call.reached)")
            }
        }

        // The behaviour first: every planted call threw what it must and reached only what it
        // must. A detached activation and every rewrite never reach the planted original.
        XCTAssertEqual(wrong, [], "the guard let a planted call through, or refused one it must pass")
    }
}
