import Observation

/// What the device has not synced, as the sync section shows it before a full sync (SPEC-503 R7).
struct Unsynced: Equatable, Sendable {
    let reviews: UInt32
    let changed: Bool
    let schema: Bool
}

/// What one direction of a full sync loses, by id (SPEC-503 R4).
struct Loss: Equatable, Sendable {
    let reviews: UInt64
    let cards: UInt64
    let notes: UInt64
}

/// The full-sync choice's counts: nil for a direction not offered.
struct FullSyncCounts: Equatable, Sendable {
    let upload: Loss?
    let download: Loss?
}

/// One step of the full-sync choice: whether the write ran, the adapter's sentence, and the
/// counts to choose from next.
struct FullSyncStep: Equatable, Sendable {
    let written: Bool
    let said: String
    let counts: FullSyncCounts?
}

/// The calls the sync section makes, each through the adapter's sync facade (SPEC-503 R2 to R7).
/// The engine session makes them; a test hands the model its own.
protocol SyncEngine: Sendable {
    func sync(hostKey: String, endpoint: String) async throws -> UInt32
    func unsynced() async throws -> Unsynced
    func countChoice(hostKey: String, endpoint: String, required: UInt32) async throws
        -> FullSyncStep
    func download(hostKey: String, endpoint: String) async throws -> FullSyncStep
    func cancelChoice() async
}

/// The sync section's state (SPEC-503 R4 to R7). It reads the host key from the one Keychain item
/// before each sync and holds no password. A normal sync that needs a full sync reads what the
/// device has not synced first, then opens the choice with its counts; the download runs only from
/// the owner's tap, and a normal sync follows it.
@MainActor
@Observable
final class SyncModel {
    /// What the device has not synced, read before the choice's counts are shown.
    private(set) var unsynced: Unsynced?
    /// The choice's counts while it is open.
    private(set) var counts: FullSyncCounts?
    /// The last sentence: the outcome, the adapter's, or the engine's refusal.
    private(set) var message = ""
    /// Whether a sync or a write is running; the section's buttons are disabled while one does.
    private(set) var syncing = false

    /// The engine's number for a full sync: a normal sync's answer below it needs no choice.
    static let fullSync: UInt32 = 2

    private let engine: any SyncEngine
    private let store: SyncCredentialStore
    private let endpoint: String

    init(engine: any SyncEngine, store: SyncCredentialStore, endpoint: String) {
        self.engine = engine
        self.store = store
        self.endpoint = endpoint
    }

    /// One normal sync with the stored host key (R5, R6). When the server needs a full sync, what
    /// the device has not synced is read, and then the choice's counts (R4, R7).
    func sync() async {
        syncing = true
        defer { syncing = false }
        do {
            guard let hostKey = try store.hostKey() else {
                message = "Sign in to sync."
                return
            }
            let required = try await engine.sync(hostKey: hostKey, endpoint: endpoint)
            guard required >= Self.fullSync else {
                message = "Synced."
                return
            }
            unsynced = try await engine.unsynced()
            let step = try await engine.countChoice(
                hostKey: hostKey, endpoint: endpoint, required: required)
            counts = step.counts
            message = step.said
        } catch {
            message = String(describing: error)
        }
    }

    /// The owner's tap on the download (R4): the adapter backs up this device's collection and
    /// downloads the server's; once written, the choice closes and a normal sync follows (R5).
    func download() async {
        let written = await write()
        counts = written.counts
        message = written.said
        guard written.written else { return }
        await sync()
    }

    /// The owner's cancel: the choice closes and nothing is written.
    func cancel() async {
        await engine.cancelChoice()
        counts = nil
        message = "Nothing was written."
    }

    /// The download, with the stored host key, as its step.
    private func write() async -> FullSyncStep {
        syncing = true
        defer { syncing = false }
        do {
            let hostKey = try store.hostKey() ?? ""
            return try await engine.download(hostKey: hostKey, endpoint: endpoint)
        } catch {
            return FullSyncStep(written: false, said: String(describing: error), counts: counts)
        }
    }
}
