// The sync section's model against a recording engine and the simulator's own Keychain (SPEC-503
// A5; R4 to R7). The class runs on one iPhone and one iPad simulator, as the SPEC's acceptance
// block names. The engine double answers as the adapter's facade answers a fresh install, and
// records each call the model makes, so a test reads what the model sent and in what order.
import Foundation
import XCTest

@testable import DeckStreak

/// A recording engine: the answers a fresh install's first sync meets, and every call made.
private actor RecordingEngine: SyncEngine {
    private(set) var calls: [String] = []
    private(set) var hostKeys: [String] = []
    private var answers: [UInt32]

    /// The normal syncs' answers, in order: the first needs a full download, the rest nothing.
    init(answers: [UInt32] = [3, 1]) {
        self.answers = answers
    }

    func sync(hostKey: String, endpoint: String) throws -> UInt32 {
        calls.append("sync")
        hostKeys.append(hostKey)
        return answers.removeFirst()
    }

    func unsynced() throws -> Unsynced {
        calls.append("unsynced")
        return Unsynced(reviews: 2, changed: true, schema: false)
    }

    func countChoice(hostKey: String, endpoint: String, required: UInt32) throws -> FullSyncStep {
        calls.append("count \(required)")
        hostKeys.append(hostKey)
        return FullSyncStep(
            written: false, said: "Choose a direction.",
            counts: FullSyncCounts(upload: nil, download: Loss(reviews: 2, cards: 0, notes: 0)))
    }

    func download(hostKey: String, endpoint: String) throws -> FullSyncStep {
        calls.append("download")
        hostKeys.append(hostKey)
        return FullSyncStep(written: true, said: "Written.", counts: nil)
    }

    func cancelChoice() {
        calls.append("cancel")
    }
}

final class SyncModelTests: XCTestCase {
    private let endpoint = "https://sync.sync-model-test.invalid/"
    private let user = "invalid-sync-model-test-user"

    /// The test's own Keychain item, holding `hostKey`.
    private func store(holding hostKey: String?) throws -> SyncCredentialStore {
        let store = SyncCredentialStore(service: endpoint, account: user)
        try store.delete()
        if let hostKey {
            try store.save(hostKey: hostKey)
        }
        return store
    }

    @MainActor
    func test_a_first_sync_goes_through_the_choice_and_a_normal_sync_follows_the_download()
        async throws
    {
        let engine = RecordingEngine()
        let model = SyncModel(engine: engine, store: try store(holding: "k1"), endpoint: endpoint)

        await model.sync()
        XCTAssertEqual(
            model.counts,
            FullSyncCounts(upload: nil, download: Loss(reviews: 2, cards: 0, notes: 0)),
            "A5: a first sync that needs a full sync opens the choice with its counts")
        await model.download()
        let calls = await engine.calls
        XCTAssertEqual(
            calls, ["sync", "unsynced", "count 3", "download", "sync"],
            "A5: the counts, the download from the owner's tap, then a normal sync")
        XCTAssertNil(model.counts, "A5: the choice closes once the download is written")
        try store(holding: nil).delete()
    }

    @MainActor
    func test_the_unsynced_count_is_read_before_the_choice_opens() async throws {
        let engine = RecordingEngine()
        let model = SyncModel(engine: engine, store: try store(holding: "k1"), endpoint: endpoint)

        await model.sync()
        let calls = await engine.calls
        XCTAssertEqual(
            Array(calls.prefix(2)), ["sync", "unsynced"],
            "A5: what the device has not synced is read before the counts")
        XCTAssertEqual(
            model.unsynced, Unsynced(reviews: 2, changed: true, schema: false),
            "A5: the unsynced count is shown while the choice is open")
        try store(holding: nil).delete()
    }

    @MainActor
    func test_the_host_key_comes_from_the_keychain_item_and_no_password_is_held() async throws
    {
        let engine = RecordingEngine()
        let model = SyncModel(engine: engine, store: try store(holding: "k2"), endpoint: endpoint)

        await model.sync()
        let hostKeys = await engine.hostKeys
        XCTAssertEqual(hostKeys, ["k2", "k2"], "A5: every call carries the stored host key")
        let labels = Mirror(reflecting: model).children.compactMap { $0.label?.lowercased() }
        XCTAssertFalse(
            labels.contains { $0.contains("password") },
            "A5: the sync model holds no password: \(labels)")
        try store(holding: nil).delete()
    }

    @MainActor
    func test_with_no_stored_host_key_nothing_is_synced() async throws {
        let engine = RecordingEngine()
        let model = SyncModel(engine: engine, store: try store(holding: nil), endpoint: endpoint)

        await model.sync()
        let calls = await engine.calls
        XCTAssertEqual(calls, [], "A5: signed out, the model makes no sync call")
        XCTAssertEqual(model.message, "Sign in to sync.", "A5: and says why")
    }
}
