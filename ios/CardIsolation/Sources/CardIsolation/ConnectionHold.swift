import Foundation
import Network

/// Layer L9's listener, the connection hold (SPEC-355 R4, ADR-366 D3, #677): a TCP listener on the
/// loopback address, on a port the system picks, that the card view's store names as its one HTTP
/// CONNECT proxy. Every connection the card view starts reaches only this listener, which answers
/// no byte, closes the connection and counts it.
/// It does NOT stop a peer connection, whose UDP traffic passes no HTTP proxy; L8 does.
public final class ConnectionHold: @unchecked Sendable {
    // @unchecked: every mutable field below is read and written only on `queue`, the queue the
    // listener and its handlers run on.
    private let queue = DispatchQueue(label: "card-isolation.connection-hold")
    private let listener: NWListener
    private var held = 0
    private var ready = false
    private var open: [NWConnection] = []
    private var boundPort: UInt16 = 0

    /// The loopback address the hold listens on.
    public static let host = "127.0.0.1"

    /// A hold that is not yet listening; `start()` makes it ready.
    public init() throws {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: NWEndpoint.Host(Self.host), port: .any)
        listener = try NWListener(using: parameters)
    }

    /// Starts the listener and waits until it has a port.
    public func start() async throws {
        listener.newConnectionHandler = { [weak self] connection in self?.receive(connection) }
        let once = HoldOnce()
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            listener.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    self?.ready = true
                    self?.boundPort = self?.listener.port?.rawValue ?? 0
                    if once.claim() { continuation.resume() }
                case .failed(let error):
                    self?.ready = false
                    if once.claim() { continuation.resume(throwing: error) }
                case .cancelled:
                    self?.ready = false
                    if once.claim() { continuation.resume(throwing: CancellationError()) }
                default:
                    break
                }
            }
            listener.start(queue: queue)
        }
    }

    /// Whether the listener is ready and holds a port.
    public var isReady: Bool {
        queue.sync { ready && boundPort != 0 }
    }

    /// The port the listener holds, or 0 before it is ready.
    public var port: UInt16 {
        queue.sync { boundPort }
    }

    /// The hold's address as a proxy endpoint, or nil while it is not ready.
    public var endpoint: NWEndpoint? {
        queue.sync { () -> NWEndpoint? in
            guard ready, boundPort != 0, let port = NWEndpoint.Port(rawValue: boundPort) else {
                return nil
            }
            return NWEndpoint.hostPort(host: NWEndpoint.Host(Self.host), port: port)
        }
    }

    /// How many connections the hold has received.
    public var count: Int {
        queue.sync { held }
    }

    /// Stops the listener and every connection it kept.
    public func cancel() {
        queue.sync {
            listener.cancel()
            for connection in open {
                connection.cancel()
            }
            open = []
            ready = false
        }
    }

    // Called on `queue`. The stub the red-first tests run against: it answers one byte and
    // counts nothing.
    private func receive(_ connection: NWConnection) {
        open.append(connection)
        connection.start(queue: queue)
        connection.send(content: Data([0x48]), completion: .contentProcessed { _ in
            connection.cancel()
        })
    }
}

/// A flag that is claimed once, so a continuation resumes exactly once.
private final class HoldOnce: @unchecked Sendable {
    // @unchecked: `claim()` is called only from the listener's state handler, on one queue.
    private var claimed = false

    func claim() -> Bool {
        if claimed { return false }
        claimed = true
        return true
    }
}
