import WebKit

/// Layer L11, the page guard (SPEC-361 R4, ADR-372): one user script, in every frame, before the
/// card's own script, in the page's world, that refuses a card's activation of a node it has
/// detached and every rewrite of its own document. A detached link's activation reaches no window,
/// so the link-activation refusal (L10) never sees it; a rewritten document loses every listener
/// L10 placed on it.
/// It does NOT stop an activation of a connected node; L10 refuses a link's.
/// Each wrapper is installed neither writable nor reconfigurable, and calls only what the guard
/// captured when it ran, so a card can neither replace a wrapper nor reach around it through a
/// function it has replaced.
public enum PageGuard {
    /// The script's source.
    public static let source = """
        (() => {
          const apply = Reflect.apply;
          const define = Object.defineProperty;
          const connected = Object.getOwnPropertyDescriptor(Node.prototype, 'isConnected').get;
          const Refusal = DOMException;
          const refuse = (what) => {
            throw new Refusal(what + ' is refused inside a card', 'NotAllowedError');
          };
          const detached = (target) => {
            try {
              return apply(connected, target, []) === false;
            } catch (error) {
              return false;
            }
          };
          const guarded = (original, what) => function (...given) {
            if (detached(this)) {
              refuse(what);
            }
            return apply(original, this, given);
          };
          const install = (owner, name, value) => {
            define(owner, name, { value, writable: false, configurable: false });
          };
          install(HTMLElement.prototype, 'click', guarded(HTMLElement.prototype.click, 'click'));
          install(EventTarget.prototype, 'dispatchEvent', guarded(EventTarget.prototype.dispatchEvent, 'dispatchEvent'));
          for (const name of ['open', 'write', 'writeln']) {
            install(Document.prototype, name, function () {
              refuse(name);
            });
          }
        })();
        """

    /// The user script the factory installs: at document start, in every frame, in the page's
    /// world, where the card's own script meets the wrappers.
    @MainActor public static let userScript = WKUserScript(
        source: source, injectionTime: .atDocumentStart, forMainFrameOnly: false, in: .page)
}
