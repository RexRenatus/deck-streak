import Foundation

/// A clip as the player plays it: a sound's bytes under its file's name, or a line of speech with
/// its language, its rate and the voice chosen for it, nil for the language's own (SPEC-348 R13).
enum ReviewClip: Equatable, Sendable {
    case sound(name: String, bytes: Data),
        speech(text: String, language: String, rate: Float, voice: String?)
}

/// Plays a face's clips in order (SPEC-348 R13).
@MainActor
final class ClipPlayer {
    /// Called with one line naming a sound the player refused.
    var refused: (String) -> Void = { _ in }

    /// Stops what plays and plays `clips` in order.
    func play(_ clips: [ReviewClip]) {}

    /// Stops what plays.
    func stop() {}
}
