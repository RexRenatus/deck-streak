// SPEC-355 A3: the connection hold (L9's listener, #677) answers no byte, closes every connection
// it receives, and counts it. Plain TCP clients on the macOS host send it the CONNECT request a
// card view's store would, with no WebKit.
import Foundation
import Network
import XCTest

import CardIsolation

/// What one connection to the hold received before it ended.
struct HoldAnswer: Sendable {
    /// Bytes the hold sent back.
    let bytes: Int
    /// Whether the hold closed the connection before the client gave up waiting.
    let closed: Bool
}

final class ConnectionHoldTests: XCTestCase {
    /// Prints how many attempts were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    func test_the_hold_answers_no_byte_and_counts_every_connection() async throws {
        let hold = try ConnectionHold()
        try await hold.start()
        defer { hold.cancel() }
        XCTAssertTrue(hold.isReady, "the hold started but is not ready")
        let port = try XCTUnwrap(NWEndpoint.Port(rawValue: hold.port), "the hold holds no port")

        let attempts = examined("connection attempts", [1, 2, 3])
        var answers: [HoldAnswer] = []
        for attempt in attempts {
            let target = "planted-\(attempt).invalid:443"
            answers.append(await HoldClient.ask(
                port: port, request: "CONNECT \(target) HTTP/1.1\r\nHost: \(target)\r\n\r\n"))
        }

        // The behaviour first: every attempt read nothing and was closed by the hold.
        for (attempt, answer) in zip(attempts, answers) {
            XCTAssertEqual(answer.bytes, 0, "attempt \(attempt): the hold answered \(answer.bytes) byte(s)")
            XCTAssertTrue(answer.closed, "attempt \(attempt): the hold kept the connection open")
        }
        let deadline = Date().addingTimeInterval(5)
        while hold.count < attempts.count && Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        XCTAssertEqual(hold.count, attempts.count, "the hold counted \(hold.count) of \(attempts.count) connections")
    }
}

/// One TCP client of the hold: it sends a request and reads until the hold closes the connection,
/// or until it gives up waiting.
private final class HoldClient: @unchecked Sendable {
    // @unchecked: every mutable field is read and written only on `queue`, the queue the
    // connection's handlers and the give-up timer run on.
    private let queue = DispatchQueue(label: "connection-hold-tests.client")
    private let connection: NWConnection
    private var continuation: CheckedContinuation<HoldAnswer, Never>?
    private var bytes = 0

    private init(port: NWEndpoint.Port, continuation: CheckedContinuation<HoldAnswer, Never>) {
        connection = NWConnection(host: NWEndpoint.Host(ConnectionHold.host), port: port, using: .tcp)
        self.continuation = continuation
    }

    /// Sends `request` to the hold at `port` and returns what came back.
    static func ask(port: NWEndpoint.Port, request: String) async -> HoldAnswer {
        await withCheckedContinuation { (continuation: CheckedContinuation<HoldAnswer, Never>) in
            let client = HoldClient(port: port, continuation: continuation)
            client.run(request)
        }
    }

    private func run(_ request: String) {
        connection.stateUpdateHandler = { [self] state in
            switch state {
            case .ready:
                connection.send(content: Data(request.utf8), completion: .contentProcessed { _ in })
                read()
            case .failed, .cancelled:
                finish(closed: true)
            default:
                break
            }
        }
        connection.start(queue: queue)
        queue.asyncAfter(deadline: .now() + 3) { [self] in finish(closed: false) }
    }

    private func read() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) { [self] data, _, isComplete, error in
            bytes += data?.count ?? 0
            if isComplete || error != nil {
                finish(closed: true)
            } else {
                read()
            }
        }
    }

    private func finish(closed: Bool) {
        guard let continuation else { return }
        self.continuation = nil
        continuation.resume(returning: HoldAnswer(bytes: bytes, closed: closed))
        connection.cancel()
    }
}
