import Foundation
import Observation

/// What the deck package import needs of the engine session (SPEC-681 R2): the one import call,
/// and the decks the collection holds after it.
protocol PackageImportEngine: Sendable {
    func importPackage(from picked: URL) async throws
    func decks() async throws -> [Deck]
}

/// The deck package import's state (SPEC-681 R1, R2, R5): whether the system file importer is
/// shown, and a refused package's words. A package the launch names with `-DSImportPackage` is
/// imported through the same path as a picked one. It lives on the main actor and awaits the
/// session, which makes the engine call off it.
@MainActor
@Observable
final class PackageImportModel {
    /// Whether the system file importer is shown.
    var showingImporter = false
    /// The last refused package's words, cleared by the next import the engine takes.
    private(set) var refusal: String?

    private let engine: any PackageImportEngine
    private let launchPackage: String?

    init(engine: any PackageImportEngine, arguments: [String] = ProcessInfo.processInfo.arguments) {
        self.engine = engine
        launchPackage = Self.value(of: "-DSImportPackage", in: arguments)
    }

    /// Imports the file at `picked` and answers the decks the collection then holds. After a
    /// refusal its words show and the decks stay `decks`, since nothing was written (R5).
    func importPackage(from picked: URL, keeping decks: [Deck]) async -> [Deck] {
        do {
            try await engine.importPackage(from: picked)
            refusal = nil
            return try await engine.decks()
        } catch {
            refusal = (error as? Refusal)?.sentence ?? String(describing: error)
            return decks
        }
    }

    /// The importer's answer: the picked file is imported, and a pick that failed leaves `decks`.
    func picked(_ result: Result<URL, any Error>, keeping decks: [Deck]) async -> [Deck] {
        guard let url = try? result.get() else { return decks }
        return await importPackage(from: url, keeping: decks)
    }

    /// The package the launch names, imported once the collection is open; with none, `decks`.
    func importAtLaunch(keeping decks: [Deck]) async -> [Deck] {
        guard let launchPackage else { return decks }
        return await importPackage(from: URL(filePath: launchPackage), keeping: decks)
    }

    /// The launch argument after `flag`, or nil when the launch does not give the flag.
    private static func value(of flag: String, in arguments: [String]) -> String? {
        guard let index = arguments.firstIndex(of: flag) else { return nil }
        return arguments.dropFirst(index + 1).first
    }
}
