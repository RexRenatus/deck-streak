// SPEC-349 R5: the planted suite's own listeners on the loopback address. Every view the suite
// builds gets a fresh pair, so a late arrival from an earlier view reaches a cancelled listener
// and is never counted against the next one.
import Foundation
import Network

/// What the listeners saw while one view was measured.
struct Arrivals: Equatable, Sendable {
    /// The path of every request line the TCP listener read, in order.
    var paths: [String] = []
    /// Every TCP connection accepted, with or without a request (a preconnect carries none).
    var connections = 0
    /// Every UDP datagram received (a peer connection's STUN request).
    var datagrams = 0
}

/// A TCP listener that reads each request line's path and answers 204, and a UDP listener that
/// counts datagrams, both bound to the loopback address on a port the system picks.
final class Listeners: @unchecked Sendable {
    // @unchecked: every mutable field below is read and written only on `queue`, the queue every
    // listener and connection handler runs on.
    private let queue = DispatchQueue(label: "card-probe.listeners")
    private let tcp: NWListener
    private let udp: NWListener
    private var seen = Arrivals()
    private var open: [NWConnection] = []

    /// The loopback address every planted card points at.
    static let host = "127.0.0.1"

    /// The TCP listener's port, once `start()` returned.
    private(set) var tcpPort: UInt16 = 0
    /// The UDP listener's port, once `start()` returned.
    private(set) var udpPort: UInt16 = 0

    init() throws {
        tcp = try NWListener(using: Self.loopback(.tcp))
        udp = try NWListener(using: Self.loopback(.udp))
    }

    private static func loopback(_ parameters: NWParameters) -> NWParameters {
        parameters.requiredLocalEndpoint = .hostPort(host: NWEndpoint.Host(host), port: .any)
        return parameters
    }

    /// Starts both listeners and waits until each has a port.
    func start() async throws {
        tcp.newConnectionHandler = { [weak self] connection in self?.accept(connection) }
        udp.newConnectionHandler = { [weak self] connection in self?.receiveDatagrams(connection) }
        try await ready(tcp)
        try await ready(udp)
        tcpPort = tcp.port?.rawValue ?? 0
        udpPort = udp.port?.rawValue ?? 0
    }

    /// What has arrived so far.
    func arrivals() -> Arrivals {
        queue.sync { seen }
    }

    /// Forgets everything that has arrived.
    func reset() {
        queue.sync { seen = Arrivals() }
    }

    /// Stops both listeners and every connection they accepted.
    func cancel() {
        queue.sync {
            tcp.cancel()
            udp.cancel()
            for connection in open {
                connection.cancel()
            }
            open = []
        }
    }

    private func ready(_ listener: NWListener) async throws {
        let once = Once()
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    if once.claim() { continuation.resume() }
                case .failed(let error):
                    if once.claim() { continuation.resume(throwing: error) }
                case .cancelled:
                    if once.claim() { continuation.resume(throwing: CancellationError()) }
                default:
                    break
                }
            }
            listener.start(queue: queue)
        }
    }

    // Called on `queue`.
    private func accept(_ connection: NWConnection) {
        seen.connections += 1
        open.append(connection)
        connection.start(queue: queue)
        read(connection, buffer: Data())
    }

    // Called on `queue`: reads until the end of the request head, records its path, answers 204.
    private func read(_ connection: NWConnection, buffer: Data) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) { [weak self] data, _, complete, error in
            guard let self else { return }
            var buffer = buffer
            if let data {
                buffer.append(data)
            }
            if let end = buffer.range(of: Data("\r\n\r\n".utf8)) {
                let head = String(decoding: buffer[buffer.startIndex..<end.lowerBound], as: UTF8.self)
                let line = head.components(separatedBy: "\r\n").first ?? ""
                let parts = line.split(separator: " ")
                if parts.count >= 2 {
                    self.seen.paths.append(String(parts[1]))
                }
                let answer = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                connection.send(content: Data(answer.utf8), completion: .contentProcessed { _ in
                    connection.cancel()
                })
                return
            }
            if complete || error != nil {
                connection.cancel()
                return
            }
            self.read(connection, buffer: buffer)
        }
    }

    // Called on `queue`.
    private func receiveDatagrams(_ connection: NWConnection) {
        open.append(connection)
        connection.start(queue: queue)
        receiveNext(connection)
    }

    private func receiveNext(_ connection: NWConnection) {
        connection.receiveMessage { [weak self] data, _, _, error in
            guard let self else { return }
            if data != nil {
                self.seen.datagrams += 1
            }
            if error == nil {
                self.receiveNext(connection)
            }
        }
    }
}

/// SPEC-355 R10: the multicast DNS witness. It joins the multicast DNS group and keeps every
/// datagram it receives, so a lookup a card asks for is counted by a label only that card's view
/// names. A lookup reaches no listener a test owns otherwise: the name is resolved before any
/// connection, and an address literal is not looked up at all.
final class Witness: @unchecked Sendable {
    // @unchecked: every mutable field below is read and written only on `queue`, the queue the
    // group's handlers run on.
    private let queue = DispatchQueue(label: "card-probe.witness")
    private let group: NWConnectionGroup
    private var datagrams: [Data] = []

    /// The multicast DNS group's address and port.
    static let address = "224.0.0.251"
    static let port: NWEndpoint.Port = 5353

    init() throws {
        let multicast = try NWMulticastGroup(
            for: [.hostPort(host: NWEndpoint.Host(Self.address), port: Self.port)])
        let parameters = NWParameters.udp
        parameters.allowLocalEndpointReuse = true
        group = NWConnectionGroup(with: multicast, using: parameters)
    }

    /// Joins the group and waits until it is ready.
    func start() async throws {
        group.setReceiveHandler(maximumMessageSize: 65_535, rejectOversizedMessages: false) {
            [weak self] _, content, _ in
            guard let self, let content else { return }
            self.datagrams.append(content)
        }
        let once = Once()
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            group.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    if once.claim() { continuation.resume() }
                case .failed(let error):
                    if once.claim() { continuation.resume(throwing: error) }
                case .cancelled:
                    if once.claim() { continuation.resume(throwing: CancellationError()) }
                default:
                    break
                }
            }
            group.start(queue: queue)
        }
    }

    /// How many datagrams so far carry `label`, as a DNS name carries it: its length, then its
    /// bytes.
    func count(_ label: String) -> Int {
        let bytes = Data(label.utf8)
        guard !bytes.isEmpty, bytes.count < 64 else { return 0 }
        let wire = Data([UInt8(bytes.count)]) + bytes
        return queue.sync { datagrams.filter { $0.range(of: wire) != nil }.count }
    }

    /// Every datagram so far, for the run's log.
    var total: Int {
        queue.sync { datagrams.count }
    }

    /// Leaves the group.
    func cancel() {
        queue.sync { group.cancel() }
    }
}

/// A flag that is claimed once, so a continuation resumes exactly once.
private final class Once: @unchecked Sendable {
    // @unchecked: `claim()` is called only from the listener's state handler, on one queue.
    private var claimed = false

    func claim() -> Bool {
        if claimed { return false }
        claimed = true
        return true
    }
}
