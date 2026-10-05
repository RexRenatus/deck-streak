/// Layer L8, the peer-connection removal (SPEC-355 R3, ADR-366 D2): the source of the one user
/// script the factory injects at document start, in every frame, in the page's own content world.
/// It deletes every global whose name matches `pattern`, and each name in `names`, before any card
/// script runs, so a card can construct no peer connection and open no transport outside the load
/// path. It adds no handler and names no host.
/// It does NOT stop a load, a navigation or a lookup; L3, L5 and L9 hold those.
public enum PeerConnectionRemoval {
    /// The names a peer connection is built from: every global that starts `RTC` or `webkitRTC`.
    public static let pattern = "^(webkit)?RTC"

    /// The names deleted beside the pattern: a transport that sends datagrams outside the load
    /// path.
    public static let names = ["WebTransport"]

    /// The script's source. The stub the red-first tests run against: empty, so every name
    /// survives.
    public static let source = ""
}
