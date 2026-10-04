import Observation

/// The review session's state, held in a model and never in a view's state, so a size-class change
/// or a scene reconnect keeps it (SPEC-339 R8). A stub: every action does nothing until the session
/// is built.
@MainActor
@Observable
final class HarnessModel {
    var openStatus = ""
    var deckNames: [String] = []
    var questionHTML = ""
    var newCount = ""
    var refusal: String?

    private let session = EngineSession()

    func start() async {}

    func study() async {}

    func answerGood() async {}
}
