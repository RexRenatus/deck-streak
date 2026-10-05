// SPEC-349 R5, R6: the planted cards, one for every channel in the iOS table of
// docs/schematics/card-frame-channels.md (section 4), the layers the single-layer variants remove,
// the declared UNOBSERVABLE and DEPTH sets, and the render-proof card. Every card points only at
// the suite's own listeners on the loopback address, or at a file the suite wrote, and every
// request it makes carries the card's own path, so an arrival names the channel that opened.
import Foundation

@testable import CardIsolation

/// What shows that a planted card's channel opened.
enum Observable: Equatable, Sendable {
    /// A request under the card's own path at the TCP listener.
    case path
    /// A TCP connection, with or without a request (a preconnect sends none).
    case connection
    /// A UDP datagram (a peer connection's STUN request).
    case datagram
    /// The `ran` marker on the document element, set by the card's own script.
    case marker
    /// The body's text after the click: the destination of a `data:` navigation.
    case body(String)
    /// A window created through the view's UI delegate.
    case window
    /// A message the `bridge` handler received.
    case bridge
    /// A navigation the view allowed after its first load: a link no probe can follow out of the
    /// view (a `mailto:` handler launch), so the decision is all that can be seen.
    case allowed
    /// The file image's natural width above zero.
    case fileImage
}

/// Where a card's markup points: the listeners' addresses and the fixture file, from the test.
struct Address: Sendable {
    let tcpPort: UInt16
    let udpPort: UInt16
    /// The fixture image the suite wrote, as a file URL.
    let fixture: URL

    /// The TCP listener's origin.
    var origin: String { "http://\(Listeners.host):\(tcpPort)" }

    /// The origin under a card's own path, so each of its requests names it.
    func under(_ id: String) -> String { "\(origin)/\(id)" }
}

/// One channel's planted card.
struct Planted: Sendable {
    /// The schematic's channel id.
    let id: String
    /// The one layer whose removal alone opens the channel, or nil when two or more hold it.
    let alone: CardLayer?
    /// Whether the card acts only when its full-view link or button is clicked.
    let click: Bool
    /// What shows the channel open.
    let observable: Observable
    /// The card's head markup.
    let head: @Sendable (Address) -> String
    /// The card's body markup.
    let body: @Sendable (Address) -> String

    /// The whole card, as the view is handed it.
    func html(_ address: Address) -> String {
        Planted.document(id: id, head: head(address), body: body(address))
    }

    /// A card document: the `planted-card` meta names it, so the suite can tell its load finished.
    static func document(id: String, head: String, body: String) -> String {
        "<!doctype html><html><head><meta charset=\"utf-8\">"
            + "<meta name=\"planted-card\" content=\"\(id)\">"
            + "<style>\(cover)</style>\(head)</head><body>\(body)</body></html>"
    }
}

/// A link or button laid over the whole view, so a click lands on it.
let cover = ".cover { position: fixed; inset: 0; display: block; margin: 0; }"

/// A 1x1 transparent image, for markup that needs an image which reaches nothing.
let pixel = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"

/// The fixture image the `file` card points at: a 1x1 red GIF the suite writes to a temporary
/// directory.
let fixtureGIF = Data(base64Encoded: "R0lGODlhAQABAIABAP8AAP///ywAAAAAAQABAAACAkQBADs=") ?? Data()

/// The script that sets the `ran` marker, and records what card script sees of the bridge.
let ranMarker = "document.documentElement.dataset.ran = '1';"
let webkitRecord = "document.documentElement.dataset.webkit = typeof window.webkit;"

/// The text a `data:` navigation's destination shows.
let dataDestination = "planted data document"

/// A card that loads from the TCP listener under its own path, with no click.
func loads(
    _ id: String, _ alone: CardLayer?, head: @escaping @Sendable (String) -> String = { _ in "" },
    _ body: @escaping @Sendable (String) -> String
) -> Planted {
    Planted(
        id: id, alone: alone, click: false, observable: .path,
        head: { head($0.under(id)) }, body: { body($0.under(id)) })
}

/// A card whose full-view link or button acts when clicked.
func clicks(
    _ id: String, _ alone: CardLayer?, _ observable: Observable,
    _ body: @escaping @Sendable (Address) -> String
) -> Planted {
    Planted(id: id, alone: alone, click: true, observable: observable, head: { _ in "" }, body: body)
}

/// A card that runs script or connects with no click, observed by `observable`.
func acts(
    _ id: String, _ alone: CardLayer?, _ observable: Observable,
    _ body: @escaping @Sendable (Address) -> String
) -> Planted {
    Planted(id: id, alone: alone, click: false, observable: observable, head: { _ in "" }, body: body)
}

/// One card per channel of the schematic's iOS table. `alone` is the schematic's "layer alone"
/// column; where that column says "measured", the value here is the prediction the measurement
/// confirms or corrects (SPEC-349 Step 6), and its comment says so.
let PLANTED: [Planted] = plantedCards()

// One statement per card, so the compiler type-checks each card's markup on its own.
private func plantedCards() -> [Planted] {
    var cards: [Planted] = []
    cards.append(loads("img", .L3) { h in
        "<img alt=\"\" src=\"\(h)/1\"><img alt=\"\" srcset=\"\(h)/2 1x\">"
            + "<picture><source srcset=\"\(h)/3\"><img alt=\"\"></picture>"
            + "<input type=\"image\" alt=\"go\" src=\"\(h)/4\">"
            + "<svg width=\"10\" height=\"10\"><image href=\"\(h)/5\" width=\"10\" height=\"10\"/>"
            + "<use href=\"\(h)/6#a\"/></svg><video poster=\"\(h)/7\"></video>"
    })
    cards.append(loads(
        "css-url", .L3,
        head: { h in
            "<style>body { background-image: image-set(url('\(h)/2') 1x); cursor: url('\(h)/3'), auto; }"
                + " li { list-style-image: url('\(h)/4'); }"
                + " .framed { border: 10px solid; border-image: url('\(h)/5') 30 round; }</style>"
        }
    ) { h in
        "<div class=\"framed\" style=\"background-image: url('\(h)/1')\">styled</div><ul><li>item</li></ul>"
    })
    cards.append(loads("css-import", .L3, head: { h in "<style>@import url('\(h)/1');</style>" }) { _ in "<p>imported</p>" })
    cards.append(loads(
        "font", .L3,
        head: { h in
            "<style>@font-face { font-family: planted; src: url('\(h)/1'); } body { font-family: planted; }</style>"
        }
    ) { _ in "<p>a planted font</p>" })
    cards.append(loads("media", .L3) { h in
        "<audio preload=\"auto\" src=\"\(h)/1\"></audio><video preload=\"auto\" src=\"\(h)/2\"></video>"
            + "<video><track default kind=\"captions\" src=\"\(h)/3\"></video>"
    })
    cards.append(loads("stylesheet", .L3, head: { h in "<link rel=\"stylesheet\" href=\"\(h)/1\">" }) { _ in "<p>linked</p>" })
    cards.append(loads(
        "preload", .L3,
        head: { h in "<link rel=\"preload\" as=\"image\" href=\"\(h)/1\"><link rel=\"modulepreload\" href=\"\(h)/2\">" }
    ) { _ in "<p>preloaded</p>" })
    cards.append(loads("prefetch", .L3, head: { h in "<link rel=\"prefetch\" href=\"\(h)/1\">" }) { _ in "<p>prefetched</p>" })
    // Measured in the schematic: predicted closed by L3, the rule list covering a preconnect.
    cards.append(acts("preconnect", .L3, .connection) { a in "<link rel=\"preconnect\" href=\"\(a.origin)\"><p>connected</p>" })
    cards.append(acts("dns-prefetch", .L3, .connection) { a in "<link rel=\"dns-prefetch\" href=\"\(a.origin)\"><p>resolved</p>" })
    // Measured in the schematic, as `preconnect` is.
    cards.append(acts("shadow-link", .L3, .connection) { a in
        "<div><template shadowrootmode=\"open\"><link rel=\"preconnect\" href=\"\(a.origin)\">"
            + "<p>shadowed</p></template></div>"
    })
    cards.append(loads("meta-refresh", nil, head: { h in "<meta http-equiv=\"refresh\" content=\"0; url=\(h)/1\">" }) { _ in
        "<p>refreshing</p>"
    })
    cards.append(loads("base", .L3, head: { h in "<base href=\"\(h)/\">" }) { _ in "<img alt=\"\" src=\"1\">" })
    cards.append(clicks("nav-self", nil, .path) { a in
        "<a class=\"cover\" data-planted-click href=\"\(a.under("nav-self"))/1\">open</a>"
    })
    cards.append(clicks("nav-data", .L5, .body(dataDestination)) { _ in
        "<a class=\"cover\" data-planted-click href=\"data:text/html,%3Cp%3Eplanted%20data%20document%3C%2Fp%3E\">open</a>"
    })
    // The schematic gives L6 alone. Predicted L5 and L6 both: WebKit asks the navigation delegate
    // about a new-window action (its target frame is nil) before it asks the UI delegate for the
    // window, and the gate cancels every action after the first load.
    cards.append(clicks("nav-blank", nil, .window) { a in
        "<a class=\"cover\" data-planted-click target=\"_blank\" href=\"\(a.under("nav-blank"))/1\">open</a>"
    })
    cards.append(clicks("external-scheme", .L5, .allowed) { _ in
        "<a class=\"cover\" data-planted-click href=\"mailto:card@example.invalid\">mail</a>"
    })
    cards.append(clicks("form", nil, .path) { a in
        "<form method=\"post\" action=\"\(a.under("form"))/1\">"
            + "<button class=\"cover\" data-planted-click type=\"submit\">send</button></form>"
    })
    cards.append(loads("nested-frame", nil) { h in
        "<iframe title=\"nested\" src=\"\(h)/1\"></iframe>"
            + "<iframe title=\"nested document\" srcdoc=\"<img alt='' src='\(h)/2'>\"></iframe>"
    })
    cards.append(loads("object", .L3) { h in
        "<object type=\"text/html\" data=\"\(h)/1\"></object><embed type=\"text/html\" src=\"\(h)/2\">"
    })
    cards.append(acts("script-inline", .L2, .marker) { _ in "<script>\(ranMarker)</script>" })
    cards.append(loads("script-src", nil) { h in "<script src=\"\(h)/1\"></script>" })
    cards.append(acts("event-handler", .L2, .marker) { _ in "<img alt=\"\" src=\"\(pixel)\" onload=\"\(ranMarker)\">" })
    cards.append(clicks("javascript-url", .L2, .marker) { _ in
        "<a class=\"cover\" data-planted-click href=\"javascript:\(ranMarker) void 0\">run</a>"
    })
    cards.append(acts("webrtc", .L2, .datagram) { a in
        "<script>\(webkitRecord) const peer = new RTCPeerConnection({ iceServers: [{ urls: "
            + "'stun:\(Listeners.host):\(a.udpPort)' }] }); peer.createDataChannel('card');"
            + " peer.createOffer().then((offer) => peer.setLocalDescription(offer));</script>"
    })
    cards.append(acts("bridge", nil, .bridge) { _ in
        "<script>\(webkitRecord) try { window.webkit.messageHandlers.bridge.postMessage('card'); }"
            + " catch (error) { document.documentElement.dataset.bridge = String(error); }</script>"
    })
    // Measured in the schematic: predicted L7 and L3 both, the rule list blocking a file load.
    cards.append(acts("file", nil, .fileImage) { a in
        "<img id=\"planted-file\" alt=\"\" src=\"\(a.fixture.absoluteString)\">"
    })
    return cards
}

/// The layers the single-layer variants remove, one at a time.
let LAYERS: [CardLayer] = CardLayer.allCases

/// The declared set of layers with no channel of their own: removing one alone opens nothing.
/// The schematic declares L1 and L4. L6 and L7 are added as predicted by the table's own
/// columns: `nav-blank` is held by L5 and L6 (above), and `file` by L7 and L3, so neither L6 nor
/// L7 is any channel's only layer. The measurement confirms or corrects all four.
let DEPTH: Set<CardLayer> = [.L1, .L4, .L6, .L7]

/// The cards whose reference view reaches nothing a probe can see, each with why.
let UNOBSERVABLE: [String: String] = [
    "dns-prefetch": "a DNS lookup of an address literal reaches no listener a test owns",
    "prefetch": "predicted as WebKit measured on the web: it sends no prefetch request",
    "nav-data": "measured on both simulators: WebKit refuses a page's own main-frame navigation "
        + "to a data: URL after the delegate allows it, so the document never changes",
]

/// The render-proof card: text, a `data:` image and a table, and no channel.
let RENDER = Planted.document(
    id: "render",
    head: "<style>body { font: 16px sans-serif; color: #123; background: #fefae0; }"
        + " td { border: 1px solid #333; padding: 4px; }</style>",
    body: "<p>Der Hund: the dog</p>"
        + "<img id=\"render-image\" alt=\"a red square\" width=\"40\" height=\"40\" "
        + "src=\"data:image/gif;base64,R0lGODlhAQABAIABAP8AAP///ywAAAAAAQABAAACAkQBADs=\">"
        + "<table><tbody><tr><td>der</td><td>Hund</td></tr><tr><td>die</td><td>Katze</td></tr></tbody></table>")
