import AVFoundation

/// A clip as the player plays it: a sound's bytes under its file's name, or a line of speech with
/// its language, its rate and the voice chosen for it, nil for the language's own (SPEC-348 R13).
enum ReviewClip: Equatable, Sendable {
    case sound(name: String, bytes: Data),
        speech(text: String, language: String, rate: Float, voice: String?)
}

/// Plays a face's clips in order (SPEC-348 R13): a sound from its bytes, a line of speech through
/// the one synthesizer in the chosen voice or the language's own. The audio session is set for
/// spoken audio that ducks other audio. A sound whose bytes the player refuses is named in one
/// line, and the clips after it play on. Playing a list stops what played before it, and the end
/// of a clip it replaced starts nothing: only the current clip's end starts the next.
@MainActor
final class ClipPlayer: NSObject {
    /// Called with one line naming a sound the player refused.
    var refused: (String) -> Void = { _ in }

    /// The clips left to play, and the sound now playing; the tests read both.
    private(set) var queue: [ReviewClip] = []
    private(set) var sound: AVAudioPlayer?
    private let synthesizer = AVSpeechSynthesizer()
    /// The clip now playing, as its player or its utterance; nil when nothing plays.
    private var current: AnyObject?

    override init() {
        super.init()
        synthesizer.delegate = self
    }

    /// Stops what plays and plays `clips` in order.
    func play(_ clips: [ReviewClip]) {
        stop()
        try? AVAudioSession.sharedInstance().setCategory(
            .playback, mode: .spokenAudio,
            options: [.duckOthers, .interruptSpokenAudioAndMixWithOthers])
        queue = clips
        playNext()
    }

    /// Stops what plays.
    func stop() {
        queue = []
        sound?.stop()
        sound = nil
        current = nil
        synthesizer.stopSpeaking(at: .immediate)
    }

    /// Starts the queue's next clip, if one is left.
    private func playNext() {
        let next = queue.first
        queue = Array(queue.dropFirst())
        _ = next.map(start)
    }

    /// A clip ended: the next starts only when it was the current one, so a late end from a
    /// clip that `stop()` replaced leaves the new list where it is.
    private func ended(_ end: ClipEnd) {
        guard end.clip === current else { return }
        playNext()
    }

    private func start(_ clip: ReviewClip) {
        switch clip {
        case .sound(let name, let bytes):
            do {
                let player = try AVAudioPlayer(data: bytes)
                player.delegate = self
                sound = player
                current = player
                player.play()
            } catch {
                refused("The sound \(name) could not be played.")
                playNext()
            }
        case .speech(let text, let language, let rate, let voice):
            let utterance = AVSpeechUtterance(string: text)
            utterance.voice = InstalledVoices.voice(voice, language: language)
            utterance.rate = rate
            current = utterance
            synthesizer.speak(utterance)
        }
    }
}

extension ClipPlayer: AVAudioPlayerDelegate {
    /// A sound ended: the next clip starts, if the sound was the current one.
    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        let end = ClipEnd(player)
        Task { @MainActor in self.ended(end) }
    }
}

extension ClipPlayer: AVSpeechSynthesizerDelegate {
    /// A line of speech ended: the next clip starts, if the line was the current one. A stopped
    /// line is cancelled, not finished, so a stop starts nothing.
    nonisolated func speechSynthesizer(
        _ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance
    ) {
        let end = ClipEnd(utterance)
        Task { @MainActor in self.ended(end) }
    }
}

/// A clip whose end a delegate reported. It holds the clip, so the clip outlives the comparison
/// and no newer clip can share its identity. It is `@unchecked Sendable` because it is only an
/// identity token: the delegate's thread wraps the clip, the box carries it to the main actor, and
/// there `===` against the current clip is its one read. Nothing dereferences the clip off the
/// main actor, and its one property is a constant, so no thread can race on it.
private final class ClipEnd: @unchecked Sendable {
    let clip: AnyObject

    init(_ clip: AnyObject) {
        self.clip = clip
    }
}
