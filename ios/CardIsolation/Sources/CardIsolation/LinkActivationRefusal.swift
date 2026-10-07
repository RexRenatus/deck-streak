import WebKit

/// Layer L10, the link-activation refusal (SPEC-361 R3, ADR-372): one user script, in every frame,
/// before the card's own script, in a content world the app owns, that cancels the default of every
/// click and middle click whose composed path holds a link, or whose target is an element that can
/// host a shadow root. WebKit opens an early connection to a clicked link's host before any
/// delegate is asked, so a link inside a card is refused before the engine follows it.
/// It does NOT see an activation that never reaches the frame's window: a detached link, or one in
/// a document whose listeners were erased. The page guard (L11) refuses those.
/// It never stops propagation, so a card's own listeners still run; it cancels only the default.
public enum LinkActivationRefusal {
    /// The script's source. A target that can host a shadow root is refused whatever it holds,
    /// because a closed root hides its link from the composed path a listener outside it reads.
    public static let source = """
        (() => {
          const html = 'http://www.w3.org/1999/xhtml';
          const svg = 'http://www.w3.org/2000/svg';
          const links = ['a', 'area'];
          const hosts = [
            'article', 'aside', 'blockquote', 'body', 'div', 'footer',
            'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'header', 'main', 'nav', 'p',
            'section', 'span',
          ];
          const element = (node) => node !== null && typeof node === 'object' && node.nodeType === 1;
          const isLink = (node) => element(node)
            && ((node.namespaceURI === html && links.includes(node.localName) && node.hasAttribute('href'))
              || (node.namespaceURI === svg && node.localName === 'a'));
          const canHost = (node) => element(node) && node.namespaceURI === html
            && (node.localName.includes('-') || hosts.includes(node.localName));
          const refuse = (event) => {
            if (event.composedPath().some(isLink) || canHost(event.target)) {
              event.preventDefault();
            }
          };
          for (const type of ['click', 'auxclick']) {
            window.addEventListener(type, refuse, { capture: true });
          }
        })();
        """

    /// The content world the refusal runs in: the app's own, so no card script can reach or
    /// remove its listeners.
    @MainActor public static let world = WKContentWorld.world(name: "card-link-activation-refusal")

    /// The user script the factory installs: at document start, in every frame, in `world`.
    @MainActor public static let userScript = WKUserScript(
        source: source, injectionTime: .atDocumentStart, forMainFrameOnly: false, in: world)
}
