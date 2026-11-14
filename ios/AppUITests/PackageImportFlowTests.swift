// The deck package import on an iPhone and an iPad simulator (SPEC-681 A1, A4 and A6): the deck
// list opens the system file importer, a file the app refuses reads as its words, and the empty
// deck list offers the import. Each test launches the app with `-DSCollectionDirectory` on a
// fresh, empty collection directory under the test runner's own temporary folder, which the app
// reaches on the simulator. The refusal's test also plants a file that is no package
// beside it and names it with `-DSImportPackage`, which imports it through the importer's own
// path. The importer's test reads the system's own interface by element type first and its Cancel
// button second, dismisses it, and never picks a file. A test waits for what it reads with a
// bounded poll and then asserts the value it read, so a red quotes what the app showed when the
// wait ran out.
import XCTest

final class PackageImportFlowTests: XCTestCase {
    /// The words a file that is no package reads as (SPEC-681 R5).
    private let unreadable =
        "This file is not an Anki deck package the app can read, so nothing was imported."
    /// What a label reads when the app does not show its element.
    private let notShown = "(not shown)"
    /// How long a test waits for what it reads, in seconds.
    private let patience: TimeInterval = 20

    /// The deck list's Import control opens the system file importer (SPEC-681 A1).
    @MainActor
    func testTheDeckListOpensTheSystemFileImporter() throws {
        let app = try launched(planting: nil)
        tapWhenShown(app.buttons["import-package"])
        let search = app.searchFields.firstMatch
        let cancel = app.buttons["Cancel"]
        let opened = reading(until: true) { search.exists || cancel.exists }
        XCTAssertTrue(
            opened,
            "the Import control opens the system file importer: its search field or its Cancel button shows")
        tapWhenShown(cancel)
    }

    /// A file the app refuses reads as the refusal's words and leaves the deck list empty
    /// (SPEC-681 A4).
    @MainActor
    func testARefusedPackageReadsAsWords() throws {
        let app = try launched(planting: "this file is no deck package")
        let refusal = app.descendants(matching: .any).matching(identifier: "import-refusal").firstMatch
        _ = reading(until: true) { refusal.exists }
        XCTAssertEqual(
            label(of: refusal), unreadable,
            "the refusal shown as import-refusal reads the words for a file that is no package")
        let offer = app.buttons["import-deck"]
        XCTAssertTrue(
            reading(until: true) { offer.exists },
            "the refused file writes no deck: the empty deck list still offers the import")
    }

    /// The empty deck list offers the import (SPEC-681 A6).
    @MainActor
    func testTheEmptyDeckListOffersTheImport() throws {
        let app = try launched(planting: nil)
        let offer = app.buttons["import-deck"]
        XCTAssertTrue(
            reading(until: true) { offer.exists && offer.isHittable },
            "the empty deck list shows the import offer import-deck, ready to tap")
    }

    // MARK: - Steps

    /// The app, launched on a fresh, empty collection directory, and given a planted file to
    /// import at start when `planting` holds its text.
    @MainActor
    private func launched(planting text: String?) throws -> XCUIApplication {
        let folder = FileManager.default.temporaryDirectory
        let stem = "package-import-\(UUID().uuidString)"
        let fresh = folder.appending(path: stem, directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: fresh, withIntermediateDirectories: true)
        var arguments = ["-DSCollectionDirectory", fresh.path(percentEncoded: false)]
        if let text {
            let planted = folder.appending(path: "\(stem).apkg")
            try Data(text.utf8).write(to: planted)
            arguments += ["-DSImportPackage", planted.path(percentEncoded: false)]
        }
        XCUIDevice.shared.orientation = .portrait
        let app = XCUIApplication()
        app.launchArguments = arguments
        app.launch()
        return app
    }

    /// Reads `read` until it returns `wanted` or the patience runs out, and returns the last read.
    @MainActor
    private func reading(until wanted: Bool, _ read: () -> Bool) -> Bool {
        let deadline = Date().addingTimeInterval(patience)
        var value = read()
        while value != wanted && Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
            value = read()
        }
        return value
    }

    /// Taps an element once it is shown and hittable. One the app never shows is left for the
    /// test's assertion to report through the value it reads.
    @MainActor
    private func tapWhenShown(_ element: XCUIElement) {
        if reading(until: true, { element.exists && element.isHittable }) {
            element.tap()
        }
    }

    /// An element's label, or `notShown` when the app does not show it.
    @MainActor
    private func label(of element: XCUIElement) -> String {
        element.exists ? element.label : notShown
    }
}
