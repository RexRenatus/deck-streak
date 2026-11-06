// The moments the app syncs at on its own, against a recording engine and the simulator's own
// Keychain (SPEC-600 A1 to A9; R1 to R5). The class runs on one iPhone and one iPad simulator, as
// the SPEC's acceptance block names. The engine double lists decks from a script and records each
// call the shell's model makes, so a test reads what was sent and in what order. Each absence it
// asserts sits beside a control in which the same moment does sync.
import Foundation
import SwiftUI
import XCTest

@testable import DeckStreak

/// A recording engine: the shell's calls and the sync section's, answered from a script.
private actor MomentsEngine: ShellEngine {
    private(set) var calls: [String] = []
    private var answers: [UInt32]
    private var lists: [[Deck]]
    private var holding = false
    private var held: CheckedContinuation<Void, Never>?

    /// `answers` are the normal syncs' answers in order, 1 once they run out; `lists` are the deck
    /// lists in order, the last one repeated.
    init(answers: [UInt32] = [], lists: [[Deck]] = [[]]) {
        self.answers = answers
        self.lists = lists
    }

    /// How many normal syncs reached the engine.
    var syncs: Int { calls.filter { call in call == "sync" }.count }

    /// Whether a sync is held inside the engine.
    var waiting: Bool { held != nil }

    /// Holds the next sync inside the engine until `release()`.
    func hold() {
        holding = true
    }

    func release() {
        holding = false
        held?.resume()
        held = nil
    }

    func open(arguments: [String]) throws {
        calls.append("open")
    }

    func decks() throws -> [Deck] {
        calls.append("decks")
        return lists.count > 1 ? lists.removeFirst() : lists[0]
    }

    func login(user: String, password: String, endpoint: String) throws -> String {
        calls.append("login")
        return "k-signed-in"
    }

    func sync(hostKey: String, endpoint: String) async throws -> UInt32 {
        calls.append("sync")
        if holding {
            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                held = continuation
            }
        }
        return answers.isEmpty ? 1 : answers.removeFirst()
    }

    func unsynced() throws -> Unsynced {
        calls.append("unsynced")
        return Unsynced(reviews: 2, changed: true, schema: false)
    }

    func countChoice(hostKey: String, endpoint: String, required: UInt32) throws -> FullSyncStep {
        calls.append("count \(required)")
        return FullSyncStep(
            written: false, said: "Choose a direction.",
            counts: FullSyncCounts(upload: nil, download: Loss(reviews: 2, cards: 0, notes: 0)))
    }

    func download(hostKey: String, endpoint: String) throws -> FullSyncStep {
        calls.append("download")
        return FullSyncStep(written: true, said: "Written.", counts: nil)
    }

    func cancelChoice() {
        calls.append("cancel")
    }
}

final class SyncMomentsTests: XCTestCase {
    private let endpoint = "https://sync.sync-moments-test.invalid/"
    private let user = "invalid-sync-moments-test-user"
    private let kept = Deck(id: 1, name: "Default")
    private let gone = Deck(id: 2, name: "Gone")

    override func tearDownWithError() throws {
        try SyncCredentialStore(service: endpoint, account: user).delete()
    }

    /// The shell's model over `engine`, with the test's own Keychain item holding `hostKey`.
    @MainActor
    private func appModel(_ engine: MomentsEngine, holding hostKey: String?) throws -> AppModel {
        let store = SyncCredentialStore(service: endpoint, account: user)
        try store.delete()
        if let hostKey {
            try store.save(hostKey: hostKey)
        }
        return AppModel(
            configuration: SyncConfiguration(endpoint: endpoint, user: user), engine: engine,
            session: EngineSession())
    }

    /// The scene leaves for the background and returns to active, as iOS turns it.
    @MainActor
    private func returnFromTheBackground(_ model: AppModel) async {
        await model.phaseChanged(.inactive)
        await model.phaseChanged(.background)
        await model.phaseChanged(.inactive)
        await model.phaseChanged(.active)
    }

    /// Waits, a bounded number of turns, until the engine holds a sync.
    private func held(_ engine: MomentsEngine) async throws {
        for _ in 0..<500 {
            if await engine.waiting {
                return
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("the held sync never reached the engine")
    }

    @MainActor
    func test_a_signed_in_launch_syncs_once_after_the_decks_are_listed() async throws {
        let engine = MomentsEngine(lists: [[kept]])
        let model = try appModel(engine, holding: "k1")

        await model.start()
        let calls = await engine.calls
        XCTAssertEqual(
            Array(calls.prefix(3)), ["open", "decks", "sync"],
            "A1: the launch lists the decks, then syncs")
        XCTAssertEqual(model.decks, [kept], "A1: the decks are listed")
        await model.start()
        await model.phaseChanged(.inactive)
        await model.phaseChanged(.active)
        let syncs = await engine.syncs
        XCTAssertEqual(syncs, 1, "A1: one sync at launch, and the first active is no second")
    }

    @MainActor
    func test_a_return_from_the_background_syncs_once_and_an_inactive_turn_does_not()
        async throws
    {
        let engine = MomentsEngine()
        let model = try appModel(engine, holding: "k1")

        await model.start()
        await model.phaseChanged(.inactive)
        await model.phaseChanged(.active)
        var syncs = await engine.syncs
        XCTAssertEqual(syncs, 1, "A2: the launch syncs, and an inactive turn is no moment")
        await returnFromTheBackground(model)
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 2, "A2: a return from the background syncs once")
        await model.phaseChanged(.inactive)
        await model.phaseChanged(.active)
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 2, "A2: the return is spent, and a later inactive turn is none")
    }

    @MainActor
    func test_signed_out_or_with_a_review_open_no_automatic_sync_reaches_the_engine()
        async throws
    {
        let out = MomentsEngine(lists: [[kept]])
        let signedOut = try appModel(out, holding: nil)
        await signedOut.start()
        await returnFromTheBackground(signedOut)
        let outCalls = await out.calls
        XCTAssertEqual(outCalls, ["open", "decks"], "A3: signed out, no sync reaches the engine")
        XCTAssertEqual(signedOut.sync.message, "", "A3: and the sync section's sentence stays")

        let engine = MomentsEngine(lists: [[kept]])
        let model = try appModel(engine, holding: "k1")
        await model.start()
        model.chosenDeck = kept
        await model.openReview()
        XCTAssertNotNil(model.review, "A3: a review is open")
        await returnFromTheBackground(model)
        var syncs = await engine.syncs
        XCTAssertEqual(syncs, 1, "A3: the launch syncs, and a return under a review does not")
        model.chosenDeck = nil
        await model.openReview()
        await returnFromTheBackground(model)
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 2, "A3: with no review open, the same return syncs once")
    }

    @MainActor
    func test_a_sync_asked_while_one_runs_or_while_the_choice_is_open_is_dropped() async throws {
        let engine = MomentsEngine(answers: [1, 1, 3], lists: [[kept]])
        let model = try appModel(engine, holding: "k1")
        await model.start()

        await engine.hold()
        let running = Task { await model.sync.sync() }
        try await held(engine)
        await returnFromTheBackground(model)
        await engine.release()
        await running.value
        var syncs = await engine.syncs
        XCTAssertEqual(syncs, 2, "A4: a return asked while a sync runs is dropped")

        await model.sync.sync()
        XCTAssertNotNil(model.sync.counts, "A4: the choice is open with its counts")
        await returnFromTheBackground(model)
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 3, "A4: a return while the choice is open is dropped")

        await model.sync.cancel()
        await returnFromTheBackground(model)
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 4, "A4: none running and no choice open, a return syncs once")
    }

    @MainActor
    func test_an_automatic_sync_that_needs_a_full_sync_stops_at_the_counts_and_downloads_nothing()
        async throws
    {
        let engine = MomentsEngine(answers: [3], lists: [[kept]])
        let model = try appModel(engine, holding: "k1")

        await model.start()
        let calls = await engine.calls
        XCTAssertEqual(
            calls, ["open", "decks", "sync", "unsynced", "count 3"],
            "A5: the launch's sync stops at the unsynced read and the counts")
        XCTAssertNotNil(model.sync.counts, "A5: the counts wait in the sync section")
        XCTAssertFalse(model.showingAccount, "A5: no sheet opens by itself")
        await returnFromTheBackground(model)
        let after = await engine.calls
        XCTAssertEqual(after, calls, "A5: no download, confirmation, cancel or second sync follows")
    }

    @MainActor
    func test_a_sign_in_that_stores_a_key_runs_one_sync() async throws {
        let engine = MomentsEngine(lists: [[kept]])
        let model = try appModel(engine, holding: nil)
        await model.start()

        model.password = "typed"
        await model.signIn()
        XCTAssertTrue(model.signedIn, "A6: the sign-in stored its key")
        let calls = await engine.calls
        XCTAssertEqual(
            calls.filter { call in call == "login" || call == "sync" }, ["login", "sync"],
            "A6: one sync after the sign-in")
    }

    @MainActor
    func test_the_review_close_action_runs_one_sync_and_a_cleared_deck_runs_none() async throws {
        let engine = MomentsEngine(lists: [[kept, gone], [kept, gone], [kept, gone], [kept]])
        let model = try appModel(engine, holding: "k1")
        await model.start()

        model.chosenDeck = kept
        await model.openReview()
        XCTAssertNotNil(model.review, "A7: a review is open")
        await model.closeReview()
        var syncs = await engine.syncs
        XCTAssertEqual(syncs, 2, "A7: the launch's sync, then one at the review's close")
        XCTAssertNil(model.chosenDeck, "A7: the close clears the deck")
        XCTAssertNil(model.review, "A7: and the review is closed")

        model.chosenDeck = gone
        await model.sync.sync()
        XCTAssertNil(model.chosenDeck, "A7: the new list lacks the chosen deck, so it is cleared")
        syncs = await engine.syncs
        XCTAssertEqual(syncs, 3, "A7: the button's sync, and the cleared deck runs none")
    }

    @MainActor
    func test_the_decks_are_read_again_after_a_sync_and_after_a_written_download() async throws {
        let downloaded = Deck(id: 7, name: "Spanish")
        let engine = MomentsEngine(
            answers: [3, 1], lists: [[kept], [kept, downloaded], [downloaded]])
        let model = try appModel(engine, holding: "k1")

        await model.start()
        XCTAssertEqual(model.decks, [kept], "A8: a fresh install lists its default deck")
        await model.sync.download()
        let calls = await engine.calls
        XCTAssertEqual(
            calls,
            ["open", "decks", "sync", "unsynced", "count 3", "download", "sync", "decks"],
            "A8: after the written download and its normal sync, the decks are read again")
        XCTAssertEqual(
            model.decks, [kept, downloaded], "A8: the downloaded decks list with no relaunch")
        await model.sync.sync()
        XCTAssertEqual(model.decks, [downloaded], "A8: and again after a normal sync")
    }

    @MainActor
    func test_a_chosen_deck_the_new_list_lacks_is_cleared() async throws {
        let engine = MomentsEngine(lists: [[kept, gone], [kept, gone], [kept]])
        let model = try appModel(engine, holding: "k1")
        await model.start()

        model.chosenDeck = kept
        await model.sync.sync()
        XCTAssertEqual(model.decks, [kept], "A9: the decks are read again")
        XCTAssertEqual(model.chosenDeck, kept, "A9: a chosen deck the new list holds is kept")
        model.chosenDeck = gone
        await model.sync.sync()
        XCTAssertNil(model.chosenDeck, "A9: a chosen deck the new list lacks is cleared")
    }
}
