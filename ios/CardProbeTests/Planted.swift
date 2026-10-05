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
    /// A multicast DNS query for the card's own `.local` name reached the witness (SPEC-355 R10).
    case query
    /// A request a learner would be asked about (a script dialog, a media capture) was answered
    /// yes: by the recorder, or as the card's own record reads it (SPEC-355 R5).
    case granted
}

/// Where a card's markup points: the listeners' addresses and the fixture file, from the test.
struct Address: Sendable {
    let tcpPort: UInt16
    let udpPort: UInt16
    /// The fixture image the suite wrote, as a file URL.
    let fixture: URL
    /// A label only this view's card names, for the multicast DNS witness (SPEC-355 R10).
    var label = "unlabelled"

    /// The TCP listener's origin.
    var origin: String { "http://\(Listeners.host):\(tcpPort)" }

    /// The origin under a card's own path, so each of its requests names it.
    func under(_ id: String) -> String { "\(origin)/\(id)" }

    /// The `.local` name a lookup card asks for: the card's id, then this view's label.
    func local(_ id: String) -> String { "\(id).\(label).local" }
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
    // The schematic gives L3 alone. Measured on both simulators: an `<object>` and an `<embed>` of
    // type text/html each load as a subframe, which the gate cancels as it does `nested-frame`'s,
    // so L3 and L5 both hold this card and no layer alone opens it.
    cards.append(loads("object", nil) { h in
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

/// SPEC-349's seven layers, named one by one: the scripts-off view as SPEC-349 measured it, with
/// L8 and L9 off. SPEC-355 R12 keeps SPEC-349's single-layer variants on this base, so their
/// declarations stand as measured.
let BASE: [CardLayer] = [.L1, .L2, .L3, .L4, .L5, .L6, .L7]

/// The layers the single-layer variants remove, one at a time: SPEC-349's base, by name.
let LAYERS: [CardLayer] = BASE

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

// SPEC-355 R7 to R11: the scripted cards of the schematic's section 7, the controls a scripted
// view must carry, and the declared sets the scripted variants measure against.

/// The controls a scripted card view must carry (SPEC-355 R1), named one by one: every layer but
/// L2, which is the verdict itself.
let CONTROLS: [CardLayer] = [.L1, .L3, .L4, .L5, .L6, .L7, .L8, .L9]

/// One scripted card: the planted card, and the controls that hold its channel in the scripted
/// view, which its reference removes. Its card's `alone` is the control whose removal alone, every
/// other control on, opens the channel (the section's "control alone" column, predicted).
struct Scripted: Sendable {
    let card: Planted
    let held: Set<CardLayer>

    var id: String { card.id }
}

/// A card whose own script acts with no click. The script sets the `ran` marker first, so an
/// absence in the scripted view is never read off a script that did not run.
func scripted(
    _ id: String, held: Set<CardLayer>, alone: CardLayer?, _ observable: Observable,
    _ script: @escaping @Sendable (Address) -> String
) -> Scripted {
    Scripted(
        card: Planted(
            id: id, alone: alone, click: false, observable: observable, head: { _ in "" },
            body: { a in "<p>\(id)</p><script>\(ranMarker) \(script(a))</script>" }),
        held: held)
}

/// A peer connection opened from `scope`'s globals with `server` as its one ICE server. Every
/// quote in it is a single quote, so a frame's `srcdoc` can carry it in double quotes.
func peer(_ scope: String, _ server: String) -> String {
    "const Peer = \(scope).RTCPeerConnection; const peer = new Peer({ iceServers: [\(server)] });"
        + " peer.createDataChannel('card'); peer.createOffer().then((offer) => peer.setLocalDescription(offer));"
}

/// A STUN server at the UDP listener.
func stun(_ a: Address) -> String {
    "{ urls: 'stun:\(Listeners.host):\(a.udpPort)' }"
}

/// A TURN server over TCP at the TCP listener: the peer connection's twin reading, so the ICE
/// agent skipping the loopback interface for UDP reads red, never green (SPEC-355 R9).
func turnOverTCP(_ a: Address) -> String {
    "{ urls: 'turn:\(Listeners.host):\(a.tcpPort)?transport=tcp', username: 'card', credential: 'card' }"
}

/// One card per script-driven channel of the section's table, but `lookup`, `app-state` and
/// `render-script`, which have their own criteria (A11, A9, A12).
let SCRIPTED: [Scripted] = scriptedCards()

// One statement per card, so the compiler type-checks each card's markup on its own.
private func scriptedCards() -> [Scripted] {
    var cards: [Scripted] = []
    cards.append(scripted("script-fetch", held: [.L3, .L9], alone: nil, .path) { a in
        "fetch('\(a.under("script-fetch"))/1').catch(() => {});"
    })
    cards.append(scripted("script-xhr", held: [.L3, .L9], alone: nil, .path) { a in
        "const request = new XMLHttpRequest(); request.open('GET', '\(a.under("script-xhr"))/1'); request.send();"
    })
    cards.append(scripted("script-websocket", held: [.L3, .L9], alone: nil, .connection) { a in
        "try { new WebSocket('ws://\(Listeners.host):\(a.tcpPort)/script-websocket/1'); } catch (error) {}"
    })
    cards.append(scripted("script-eventsource", held: [.L3, .L9], alone: nil, .path) { a in
        "new EventSource('\(a.under("script-eventsource"))/1');"
    })
    cards.append(scripted("script-beacon", held: [.L3, .L9], alone: nil, .path) { a in
        "navigator.sendBeacon('\(a.under("script-beacon"))/1', 'card');"
    })
    cards.append(scripted("script-image", held: [.L3, .L9], alone: nil, .path) { a in
        "const image = new Image(); image.src = '\(a.under("script-image"))/1';"
    })
    cards.append(scripted("script-worker", held: [.L3, .L9], alone: nil, .path) { a in
        "const source = \"fetch('\(a.under("script-worker"))/1').catch(() => {});\";"
            + " new Worker(URL.createObjectURL(new Blob([source], { type: 'text/javascript' })));"
    })
    cards.append(scripted("script-link", held: [.L3, .L9], alone: nil, .connection) { a in
        "const early = document.createElement('link'); early.rel = 'preconnect'; early.href = '\(a.origin)';"
            + " document.head.appendChild(early); const sheet = document.createElement('link');"
            + " sheet.rel = 'stylesheet'; sheet.href = '\(a.under("script-link"))/1'; document.head.appendChild(sheet);"
    })
    // L3 holds the two navigations as well as L5 and L9, measured on both simulators: their
    // reference without L5 and L9 allowed every navigation and no load arrived, as SPEC-349's
    // `nav-self` reads without L5 (the rule list refuses the document load).
    cards.append(scripted("script-nav", held: [.L3, .L5, .L9], alone: nil, .connection) { a in
        "let tries = 0; const go = () => { tries += 1; location.assign('\(a.under("script-nav"))/1');"
            + " if (tries < 5) { setTimeout(go, 200); } }; setTimeout(go, 0);"
    })
    cards.append(scripted("script-form", held: [.L3, .L5, .L9], alone: nil, .path) { a in
        "const form = document.createElement('form'); form.method = 'post';"
            + " form.action = '\(a.under("script-form"))/1'; document.body.appendChild(form);"
            + " setTimeout(() => form.submit(), 0);"
    })
    // L5 holds the window as well as L6, measured on both simulators: its reference without L6
    // created no window, removing L6 alone opened nothing, and SPEC-349's `nav-blank` creates one
    // only with both removed.
    cards.append(scripted("script-open", held: [.L5, .L6], alone: nil, .window) { a in
        "setTimeout(() => window.open('\(a.under("script-open"))/1'), 0);"
    })
    cards.append(scripted("webrtc-stun", held: [.L8], alone: .L8, .datagram) { a in
        peer("window", stun(a))
    })
    cards.append(scripted("webrtc-turn-tcp", held: [.L8], alone: .L8, .connection) { a in
        peer("window", turnOverTCP(a))
    })
    cards.append(scripted("webrtc-blank-frame", held: [.L8], alone: .L8, .datagram) { a in
        "const frame = document.createElement('iframe'); frame.title = 'blank'; document.body.appendChild(frame);"
            + " const inner = frame.contentWindow; \(peer("inner", stun(a)))"
    })
    // L5 holds the `srcdoc` frame as well as L8, measured on both simulators: its reference
    // without L8 sent nothing with its marker set, and removing L5 alone, with no L8 built, sent a
    // datagram. The gate cancels the frame's own navigation, so its script never runs.
    cards.append(scripted("webrtc-srcdoc-frame", held: [.L5, .L8], alone: nil, .datagram) { a in
        "const frame = document.createElement('iframe'); frame.title = 'srcdoc';"
            + " frame.srcdoc = \"<script>\(peer("window", stun(a)))<\\/script>\"; document.body.appendChild(frame);"
    })
    cards.append(scripted("webrtc-written-frame", held: [.L8], alone: .L8, .datagram) { a in
        "const frame = document.createElement('iframe'); frame.title = 'written'; document.body.appendChild(frame);"
            + " frame.contentDocument.open(); frame.contentDocument.write(\"<script>\(peer("window", stun(a)))<\\/script>\");"
            + " frame.contentDocument.close();"
    })
    // UNOBSERVABLE when the engine has no WebTransport: the reference's record then reads
    // `absent` (ABSENT_ALLOWED).
    cards.append(scripted("webtransport", held: [.L8], alone: .L8, .datagram) { a in
        "const present = typeof WebTransport !== 'undefined';"
            + " document.documentElement.dataset.record = present ? 'present' : 'absent';"
            + " if (present) { try { new WebTransport('https://\(Listeners.host):\(a.udpPort)/webtransport'); }"
            + " catch (error) {} }"
    })
    // Depth: the engine's own answer with no delegate is already a refusal.
    cards.append(scripted("dialog", held: [.L6], alone: nil, .granted) { _ in
        "const answers = {}; try { alert('card'); answers.alert = 'returned'; } catch (error) { answers.alert = error.name; }"
            + " try { answers.confirm = String(confirm('card')); } catch (error) { answers.confirm = error.name; }"
            + " try { answers.prompt = String(prompt('card', '')); } catch (error) { answers.prompt = error.name; }"
            + " document.documentElement.dataset.record = JSON.stringify(answers);"
    })
    // Depth, as `dialog` is. Its reference is blind, measured on both simulators: the card's
    // document has no `navigator.mediaDevices`, so no capture request is made (BLIND_SCRIPTED).
    cards.append(scripted("capture", held: [.L6], alone: nil, .granted) { _ in
        "const note = (value) => { document.documentElement.dataset.record = JSON.stringify({ capture: value }); };"
            + " note('pending'); if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {"
            + " navigator.mediaDevices.getUserMedia({ video: true, audio: true })"
            + ".then(() => note('granted'), (error) => note(error.name)); } else { note('absent'); }"
    })
    return cards
}

/// The cards whose reference may read `absent`: the engine has no such interface, so the card is
/// UNOBSERVABLE there, and the run prints it.
let ABSENT_ALLOWED: Set<String> = ["webtransport"]

/// The scripted cards whose reference is blind, counted and pinned from the measurement on both
/// simulators: the reference reached nothing, and no held set can open it. A7 asserts its blind
/// set equals this one and prints the count, so a new blind reference, or this one opening, reads
/// red. `capture`: the card's document has no `navigator.mediaDevices`, so L6's capture arm is
/// never asked in the probe.
let BLIND_SCRIPTED: Set<String> = ["capture"]

/// The declared set of controls with no scripted channel of their own: removing one alone, every
/// other control on, opens nothing (A10 measures it). L8 opens every peer connection but the
/// `srcdoc` frame's, which L5 also holds; L3 and L9 share every load, L3, L5 and L9 every
/// navigation, and L5 and L6 the window, as measured on both simulators.
let DEPTH_SCRIPTED: Set<CardLayer> = [.L1, .L3, .L4, .L5, .L6, .L7, .L9]

/// SPEC-355 R10: the lookup card. A static and a script-added `dns-prefetch`, and a script's
/// fetch, each of a `.local` name whose first label is the card's id and whose second only this
/// view names.
let LOOKUP = Planted(
    id: "lookup", alone: nil, click: false, observable: .query,
    head: { a in "<link rel=\"dns-prefetch\" href=\"http://\(a.local("lookup"))/\">" },
    body: { a in
        "<p>lookup</p><script>\(ranMarker) const hint = document.createElement('link'); hint.rel = 'dns-prefetch';"
            + " hint.href = 'http://\(a.local("lookup"))/'; document.head.appendChild(hint);"
            + " fetch('http://\(a.local("lookup"))/1').catch(() => {});</script>"
    })

/// SPEC-355 R6: the card that tries to read the app's state: the default store's cookie and
/// storage, a file the app wrote, `window.webkit`, and a stored credential. It writes what it read
/// into its own record after each step.
func appState(file: URL) -> String {
    let script: [String] = [
        ranMarker,
        "const read = {};",
        "const done = () => { document.documentElement.dataset.record = JSON.stringify(read); };",
        "try { read.cookie = document.cookie; } catch (error) { read.cookie = error.name; }",
        "try { read.storage = String(localStorage.getItem('planted')); } catch (error) { read.storage = error.name; }",
        "read.webkit = typeof window.webkit; done();",
        "fetch('\(file.absoluteString)').then((response) => response.text())",
        ".then((text) => { read.file = text; }, (error) => { read.file = error.name; })",
        ".then(() => (navigator.credentials && navigator.credentials.get)",
        "? navigator.credentials.get({ password: true, mediation: 'silent' }) : null)",
        ".then((credential) => { read.credential = credential ? JSON.stringify(credential) : 'none'; },",
        "(error) => { read.credential = error.name; })",
        ".then(done, done);",
    ]
    return Planted.document(
        id: "app-state", head: "",
        body: "<p>app state</p><script>\(script.joined(separator: " "))</script>")
}

/// SPEC-355 R11: a benign scripted card, a hint toggle. Its script shows the hint and sets the
/// marker, so its body text matches the reference's only when the script ran.
let RENDER_SCRIPT = Planted.document(
    id: "render-script",
    head: "<style>body { font: 16px sans-serif; color: #123; background: #fefae0; }</style>",
    body: "<p>Der Hund</p><p id=\"hint\" hidden>the dog</p><button id=\"toggle\" type=\"button\">hint</button>"
        + "<script>\(ranMarker) const hint = document.getElementById('hint');"
        + " document.getElementById('toggle').addEventListener('click', () => { hint.hidden = !hint.hidden; });"
        + " hint.hidden = false;</script>")
