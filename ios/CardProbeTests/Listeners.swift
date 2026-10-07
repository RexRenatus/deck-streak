// SPEC-349 R5: the planted suite's own listeners. SPEC-355's loopback risk (section 6) moves them
// off the loopback address to the simulator host's non-loopback address, read once at run time:
// the hold (L9) stays on loopback, and only the destinations move, so a connection the engine
// sends around the proxy for a loopback destination is measured again on one that is not. Every
// view the suite builds gets a fresh pair, so a late arrival from an earlier view reaches a
// cancelled listener and is never counted against the next one.
import Darwin
import Foundation
import Network

/// Why the listeners cannot start.
enum ListenersError: Error, CustomStringConvertible {
    /// No interface that is up, running and not loopback holds an IPv4 address. The listeners
    /// never fall back to loopback, which would repeat the first measurement and call it the
    /// second.
    case noNonLoopbackAddress

    var description: String {
        "no up, running, non-loopback interface holds an IPv4 address; the listeners never fall back to loopback"
    }
}

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
/// counts datagrams, both bound to the simulator host's non-loopback address on a port the system
/// picks.
final class Listeners: @unchecked Sendable {
    // @unchecked: every mutable field below is read and written only on `queue`, the queue every
    // listener and connection handler runs on.
    private let queue = DispatchQueue(label: "card-probe.listeners")
    private let tcp: NWListener
    private let udp: NWListener
    private var seen = Arrivals()
    private var open: [NWConnection] = []

    /// The simulator host's non-loopback address every planted card points at, read once at run
    /// time. It is never printed: the one line the read prints says only whether an address was
    /// found off loopback. Empty when there is none, and then `init` throws.
    static let host: String = {
        let found = hostAddress()
        let offLoopback = found.flatMap { IPv4Address($0.address) }.map { !$0.isLoopback } ?? false
        print(
            "listeners: the simulator host's non-loopback address found=\(found != nil)"
                + " off loopback=\(offLoopback) on an en interface=\(found?.name.hasPrefix("en") ?? false)")
        return offLoopback ? (found?.address ?? "") : ""
    }()

    /// The TCP listener's port, once `start()` returned.
    private(set) var tcpPort: UInt16 = 0
    /// The UDP listener's port, once `start()` returned.
    private(set) var udpPort: UInt16 = 0

    init() throws {
        guard !Self.host.isEmpty else { throw ListenersError.noNonLoopbackAddress }
        tcp = try NWListener(using: Self.bound(.tcp))
        udp = try NWListener(using: Self.bound(.udp))
    }

    private static func bound(_ parameters: NWParameters) -> NWParameters {
        parameters.requiredLocalEndpoint = .hostPort(host: NWEndpoint.Host(host), port: .any)
        return parameters
    }

    /// The IPv4 address of an interface that is up, running and not loopback, with its
    /// interface's name: the first on an `en` interface, else the first on any, else nil.
    static func hostAddress() -> (name: String, address: String)? {
        var list: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&list) == 0 else { return nil }
        defer { freeifaddrs(list) }
        var found: [(name: String, address: String)] = []
        var cursor = list
        while let entry = cursor?.pointee {
            cursor = entry.ifa_next
            let flags = entry.ifa_flags
            guard flags & UInt32(IFF_UP) != 0, flags & UInt32(IFF_RUNNING) != 0,
                  flags & UInt32(IFF_LOOPBACK) == 0,
                  let address = entry.ifa_addr, address.pointee.sa_family == sa_family_t(AF_INET)
            else { continue }
            var text = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            guard getnameinfo(
                address, socklen_t(address.pointee.sa_len), &text, socklen_t(text.count), nil, 0,
                NI_NUMERICHOST) == 0
            else { continue }
            let numeric = String(
                decoding: text.prefix(while: { $0 != 0 }).map { UInt8(bitPattern: $0) }, as: UTF8.self)
            found.append((name: String(cString: entry.ifa_name), address: numeric))
        }
        return found.first { $0.name.hasPrefix("en") } ?? found.first
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

    /// Stops both listeners and every connection they accepted that is still open, each once.
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
                connection.send(content: Data(answer.utf8), completion: .contentProcessed { [weak self] _ in
                    self?.close(connection)
                })
                return
            }
            if complete || error != nil {
                self.close(connection)
                return
            }
            self.read(connection, buffer: buffer)
        }
    }

    // Called on `queue`: cancels a connection this listener answered or lost, and forgets it, so
    // `cancel()` never cancels it a second time. A second cancel makes the Network framework log
    // the connection with both its endpoints, and the listeners' address is never printed.
    private func close(_ connection: NWConnection) {
        guard let index = open.firstIndex(where: { $0 === connection }) else { return }
        open.remove(at: index)
        connection.cancel()
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
