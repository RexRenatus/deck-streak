// The data screen's model against a recording engine (SPEC-516 B5, B6, B8; R5, R6, R8). The class
// runs on one iPhone and one iPad simulator, as the SPEC's acceptance block names. The engine double
// answers as the adapter's backup calls answer, and records each call the model makes, so a test
// reads what the model sent and in what order.
import Foundation
import XCTest

@testable import DeckStreak

/// Why the recording engine refused a call: the adapter's refusal, as the model meets it.
private struct Refused: Error {}

/// A recording engine: the device's backups as the adapter lists them, and every call made.
private actor RecordingData: DataEngine {
    private(set) var calls: [String] = []
    private var listed: [DeviceCopy]
    private let refuses: Bool

    init(listed: [DeviceCopy], refuses: Bool = false) {
        self.listed = listed
        self.refuses = refuses
    }

    func backUpNow() throws -> String {
        calls.append("backUpNow")
        if refuses {
            throw Refused()
        }
        let made = DeviceCopy(id: "backup-3-7", serverCopy: false, ageSeconds: 0)
        listed.insert(made, at: 0)
        return made.id
    }

    func deviceBackups() throws -> [DeviceCopy] {
        calls.append("deviceBackups")
        return listed
    }

    func backupBytes(_ id: String) throws -> Data {
        calls.append("backupBytes \(id)")
        if refuses {
            throw Refused()
        }
        return Data(id.utf8)
    }
}

@MainActor
final class DataModelTests: XCTestCase {
    private let older = DeviceCopy(id: "backup-1-7", serverCopy: false, ageSeconds: 7200)
    private let copy = DeviceCopy(id: "server-copy-2-7", serverCopy: true, ageSeconds: 60)

    /// The list is the adapter's, in its order, newest first.
    func testTheListIsTheAdaptersNewestFirst() async {
        let engine = RecordingData(listed: [copy, older])
        let model = DataModel(engine: engine)
        await model.list()
        XCTAssertEqual(model.backups, [copy, older])
        let calls = await engine.calls
        XCTAssertEqual(calls, ["deviceBackups"])
    }

    /// Back up now asks the adapter once, then lists the backups again, and says a backup was made.
    func testBackUpNowMakesOneBackupAndListsAgain() async {
        let engine = RecordingData(listed: [older])
        let model = DataModel(engine: engine)
        await model.list()
        await model.backUpNow()
        XCTAssertEqual(model.backups.map(\.id), ["backup-3-7", "backup-1-7"])
        XCTAssertEqual(model.message, DataWords.made)
        XCTAssertFalse(model.working)
        let calls = await engine.calls
        XCTAssertEqual(calls, ["deviceBackups", "backUpNow", "deviceBackups"])
    }

    /// A refused back up now lists nothing new and says so in the words table's words.
    func testARefusedBackUpNowListsNothingNew() async {
        let engine = RecordingData(listed: [older], refuses: true)
        let model = DataModel(engine: engine)
        await model.list()
        await model.backUpNow()
        XCTAssertEqual(model.backups, [older])
        XCTAssertEqual(model.message, DataWords.refused)
        let calls = await engine.calls
        XCTAssertEqual(calls, ["deviceBackups", "backUpNow"])
    }

    /// An export reads the listed backup's bytes by its id and shows the save dialog with them.
    func testAnExportHandsTheListedBytesToTheSaveDialog() async {
        let engine = RecordingData(listed: [copy])
        let model = DataModel(engine: engine)
        await model.list()
        await model.export(copy)
        XCTAssertTrue(model.exporterShown)
        XCTAssertEqual(model.exporting?.bytes, Data(copy.id.utf8))
        XCTAssertEqual(model.exportName, "deck-streak-server-copy-2-7.anki2")
        model.exported(.success(URL(filePath: "/saved")))
        XCTAssertEqual(model.message, DataWords.saved)
        XCTAssertNil(model.exporting)
        model.exported(.failure(Refused()))
        XCTAssertEqual(model.message, DataWords.unsaved)
        let calls = await engine.calls
        XCTAssertEqual(calls, ["deviceBackups", "backupBytes server-copy-2-7"])
    }

    /// A refused export shows no save dialog.
    func testARefusedExportShowsNoSaveDialog() async {
        let engine = RecordingData(listed: [copy], refuses: true)
        let model = DataModel(engine: engine)
        await model.export(copy)
        XCTAssertFalse(model.exporterShown)
        XCTAssertNil(model.exporting)
        XCTAssertEqual(model.message, DataWords.refused)
    }

    /// With no signed-in session the server's backup age is unknown, in words and with no number,
    /// and the JSON copy names the web screen and the bot's /export (R5, R6).
    func testWithoutASessionTheAgeIsUnknownAndTheCopyNamesExport() {
        let model = DataModel(engine: RecordingData(listed: []))
        XCTAssertEqual(model.serverAge, DataWords.serverUnknown)
        XCTAssertNil(model.serverAge.rangeOfCharacter(from: .decimalDigits))
        XCTAssertTrue(model.jsonCopy.contains("/export"))
        XCTAssertTrue(model.jsonCopy.contains("data screen on the web"))
    }

    /// Each line names whose collection a backup holds.
    func testEachLineNamesWhoseCollectionItHolds() {
        XCTAssertTrue(DataWords.line(older).hasPrefix("This device's collection, saved "))
        XCTAssertTrue(DataWords.line(copy).hasPrefix("The server's collection, saved "))
    }
}
