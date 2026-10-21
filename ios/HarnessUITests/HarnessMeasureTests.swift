// Section 7's cold start and memory (SPEC-339 M1, M2), run only by the harness job's Release step;
// the Debug step skips this class. Each test measures and asserts nothing, so the red-first record
// discloses each as `not red: a measurement`. XCTest runs each block once more than
// `iterationCount` and discards the first run.
import XCTest

/// How long a pass waits for the app to show what the engine answered.
private let engineWait: TimeInterval = 30

final class HarnessMeasureTests: XCTestCase {
    /// Five measured passes, after the one XCTest discards.
    private func fivePasses() -> XCTMeasureOptions {
        let options = XCTMeasureOptions()
        options.iterationCount = 5
        return options
    }

    /// The app's own interval, by the names `Signposts.swift` gives it. A UI test cannot import the
    /// app, so the names are repeated here.
    private let openToDeckList = XCTOSSignpostMetric(
        subsystem: "deck-streak.harness", category: "launch", name: "open-to-deck-list")

    /// M1: the launch to the first responsive frame, and the harness's interval from its first
    /// line to the deck names shown. Each pass launches the app afresh, since `launch()` ends a
    /// running instance first, and waits for the deck list so the interval ends inside the pass.
    @MainActor
    func test_m1_cold_start() {
        let app = XCUIApplication()
        measure(
            metrics: [XCTApplicationLaunchMetric(waitUntilResponsive: true), openToDeckList],
            options: fivePasses()
        ) {
            app.launch()
            _ = app.staticTexts["deck-row-Default"].waitForExistence(timeout: engineWait)
        }
    }

    /// M2: the app's peak physical memory over one pass of open, list, render and answer. As
    /// Apple's sample does, each pass launches the app and then starts measuring, so the metric
    /// reads a running process.
    @MainActor
    func test_m2_memory() {
        let app = XCUIApplication()
        let options = fivePasses()
        options.invocationOptions = [.manuallyStart]
        measure(metrics: [XCTMemoryMetric(application: app)], options: options) {
            app.launch()
            startMeasuring()
            let deck = app.staticTexts["deck-row-Default"].firstMatch
            if deck.waitForExistence(timeout: engineWait) {
                deck.tap()
            }
            let study = app.buttons["study"].firstMatch
            if study.waitForExistence(timeout: engineWait) {
                study.tap()
            }
            _ = app.webViews.firstMatch.staticTexts["synthetic front"]
                .waitForExistence(timeout: engineWait)
            let good = app.buttons["answer-good"].firstMatch
            if good.waitForExistence(timeout: engineWait) {
                good.tap()
            }
            _ = app.staticTexts["new-count"].firstMatch.waitForExistence(timeout: engineWait)
        }
    }
}
