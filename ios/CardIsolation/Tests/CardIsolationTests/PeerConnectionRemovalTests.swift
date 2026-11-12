// SPEC-355 A2: the peer-connection removal (L8) deletes every global a peer connection or a
// datagram transport is built from, and nothing else. Its source runs in a JavaScriptCore context
// on the macOS host, over globals the test plants: no WebKit, so no name exists unless planted.
import Foundation
import JavaScriptCore
import XCTest

import CardIsolation

final class PeerConnectionRemovalTests: XCTestCase {
    /// Prints how many names were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// Names the removal must delete: every global starting `RTC` or `webkitRTC`, and
    /// `WebTransport`. Named here rather than read from the code under test.
    static let removed = [
        "RTCPeerConnection", "webkitRTCPeerConnection", "RTCDataChannel", "RTCSessionDescription",
        "RTCIceCandidate", "RTCRtpSender", "RTCRtpReceiver", "RTCRtpTransceiver", "RTCDtlsTransport",
        "RTCIceTransport", "RTCSctpTransport", "RTCCertificate", "RTCDTMFSender",
        "RTCPeerConnectionIceEvent", "RTCDataChannelEvent", "RTCTrackEvent", "RTCError",
        "RTCErrorEvent", "RTCRtpScriptTransform", "webkitRTCSessionDescription", "WebTransport",
    ]

    /// Names the removal must keep: other globals, and names that only resemble the ones above.
    static let kept = [
        "fetch", "XMLHttpRequest", "WebSocket", "EventSource", "Worker", "Image", "MediaStream",
        "webkitURL", "webkitMediaStream", "xRTCPeerConnection", "rtcPeerConnection",
        "WebTransportError",
    ]

    func test_the_removal_deletes_every_peer_connection_name_and_nothing_else() throws {
        let planted = examined("planted global names", Self.removed + Self.kept)
        let encoded = try JSONSerialization.data(withJSONObject: planted)
        let list = try XCTUnwrap(String(data: encoded, encoding: .utf8), "the planted names were not encoded")
        let context = try XCTUnwrap(JSContext(), "no JavaScript context")
        context.evaluateScript(
            "for (const name of \(list)) { Object.defineProperty(globalThis, name, "
                + "{ value: function () {}, writable: true, enumerable: false, configurable: true }); }")
        XCTAssertNil(context.exception, "planting the names threw")

        context.evaluateScript(PeerConnectionRemoval.source)
        XCTAssertNil(context.exception, "the removal threw: \(String(describing: context.exception))")

        let read = context.evaluateScript(
            "JSON.stringify(\(list).filter((name) => Object.prototype.hasOwnProperty.call(globalThis, name)))")
        let text = try XCTUnwrap(read?.toString(), "the surviving names were not read")
        let decoded = try JSONSerialization.jsonObject(with: Data(text.utf8))
        let survivors = try XCTUnwrap(decoded as? [String], "the surviving names are not a list of names: \(text)")

        // The behaviour first: no name the removal owns survives it.
        XCTAssertEqual(
            survivors.filter { Self.removed.contains($0) }, [],
            "a peer-connection name survived the removal")
        XCTAssertEqual(
            survivors.filter { Self.kept.contains($0) }, Self.kept,
            "the removal deleted a name it does not own")
    }
}
