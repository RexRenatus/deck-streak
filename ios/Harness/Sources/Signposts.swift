import os

/// The harness's own launch interval, from its first line to the deck names shown (SPEC-339
/// section 7). `HarnessMeasureTests` measures it with `XCTOSSignpostMetric` by these three names;
/// a UI test cannot import the app, so it repeats them, and a rename here must be made there too.
@MainActor
enum Signposts {
    static let subsystem = "deck-streak.harness"
    static let category = "launch"
    static let launchName: StaticString = "open-to-deck-list"

    private static let signposter = OSSignposter(subsystem: subsystem, category: category)
    private static var launch: OSSignpostIntervalState?

    /// Begins the interval. The app's initializer calls it before anything else it does.
    static func beginLaunch() {
        launch = signposter.beginInterval(launchName)
    }

    /// Ends the interval the first time the deck list shows names; later calls do nothing, and an
    /// open that is refused never ends it, so its figure reads `not measured`.
    static func endLaunch() {
        guard let state = launch else { return }
        launch = nil
        signposter.endInterval(launchName, state)
    }
}
