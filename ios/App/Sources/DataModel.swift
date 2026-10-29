import Foundation
import Observation
import SwiftUI
import UniformTypeIdentifiers

/// One backup this device keeps, as the data screen lists it: its id, whether it is a copy of the
/// server's collection, and its age in whole seconds (SPEC-516 R5). The adapter names them; the
/// app derives nothing.
struct DeviceCopy: Equatable, Sendable, Identifiable {
    let id: String
    let serverCopy: Bool
    let ageSeconds: UInt64
}

/// The calls the data screen makes, each through the adapter, which reaches the engine through the
/// core (SPEC-516 R5). The engine session makes them; a test hands the model its own.
protocol DataEngine: Sendable {
    func backUpNow() async throws -> String
    func deviceBackups() async throws -> [DeviceCopy]
    func backupBytes(_ id: String) async throws -> Data
}

/// One backup's bytes as the system's save dialog takes them. The app only writes one: it never
/// opens a backup through it.
struct BackupFile: FileDocument {
    static let readableContentTypes: [UTType] = [.data]

    let bytes: Data

    init(bytes: Data) {
        self.bytes = bytes
    }

    init(configuration: ReadConfiguration) throws {
        throw CocoaError(.featureUnsupported)
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: bytes)
    }
}

/// The data screen's state (SPEC-516 R5, R6, R8): this device's backups as the adapter lists them
/// after the core's retention, back up now, and one backup's bytes handed to the save dialog. The
/// JSON copy and the server's backup age need a signed-in session for the service, which this app
/// does not hold, so the screen names where the copy is and shows the age as unknown.
@MainActor
@Observable
final class DataModel {
    /// The device's backups, newest first, as the adapter last listed them.
    private(set) var backups: [DeviceCopy] = []
    /// The last sentence: an outcome or a refusal, in the words table's words.
    private(set) var message = ""
    /// Whether a back up now runs; the button is disabled while one does.
    private(set) var working = false
    /// Whether the save dialog is shown.
    var exporterShown = false
    /// The backup the save dialog saves, and the name it offers.
    private(set) var exporting: BackupFile?
    private(set) var exportName = ""
    /// The JSON copy's sentence: where the copy is, while the app holds no session (R5).
    let jsonCopy = DataWords.jsonElsewhere
    /// The server's backup age: unknown, in words, while the app holds no session (R6).
    let serverAge = DataWords.serverUnknown

    private let engine: any DataEngine

    init(engine: any DataEngine) {
        self.engine = engine
    }

    /// Lists the device's backups; the adapter applies the core's retention first.
    func list() async {
        do {
            backups = try await engine.deviceBackups()
        } catch {
            message = DataWords.refused
        }
    }

    /// Backs this device up now, then lists the backups again.
    func backUpNow() async {
        working = true
        defer { working = false }
        do {
            _ = try await engine.backUpNow()
            message = DataWords.made
            backups = try await engine.deviceBackups()
        } catch {
            message = DataWords.refused
        }
    }

    /// Reads `copy`'s bytes from the adapter and shows the save dialog for them.
    func export(_ copy: DeviceCopy) async {
        do {
            exporting = BackupFile(bytes: try await engine.backupBytes(copy.id))
            exportName = DataWords.filename(copy)
            exporterShown = true
        } catch {
            message = DataWords.refused
        }
    }

    /// What the save dialog did with the backup.
    func exported(_ result: Result<URL, any Error>) {
        switch result {
        case .success:
            message = DataWords.saved
        case .failure:
            message = DataWords.unsaved
        }
        exporting = nil
    }
}
