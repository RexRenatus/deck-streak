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
/// line, and the clips after it play on. Playing a list stops what played before it.
@MainActor
final class ClipPlayer: NSObject {
    /// Called with one line naming a sound the player refused.
    var refused: (String) -> Void = { _ in }

    private var queue: [ReviewClip] = []
    private var sound: AVAudioPlayer?
    private let synthesizer = AVSpeechSynthesizer()

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
        synthesizer.stopSpeaking(at: .immediate)
    }

    /// Starts the queue's next clip, if one is left.
    private func playNext() {
        let next = queue.first
        queue = Array(queue.dropFirst())
        _ = next.map(start)
    }

    private func start(_ clip: ReviewClip) {
        switch clip {
        case .sound(let name, let bytes):
            do {
                let player = try AVAudioPlayer(data: bytes)
                player.delegate = self
                sound = player
                player.play()
            } catch {
                refused("The sound \(name) could not be played.")
                playNext()
            }
        case .speech(let text, let language, let rate, let voice):
            let utterance = AVSpeechUtterance(string: text)
            utterance.voice =
                voice.flatMap(AVSpeechSynthesisVoice.init(identifier:))
                ?? AVSpeechSynthesisVoice(language: language)
            utterance.rate = rate
            synthesizer.speak(utterance)
        }
    }
}

extension ClipPlayer: AVAudioPlayerDelegate {
    /// A sound ended: the next clip starts.
    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        Task { @MainActor in self.playNext() }
    }
}

extension ClipPlayer: AVSpeechSynthesizerDelegate {
    /// A line of speech ended: the next clip starts. A stopped line is cancelled, not finished,
    /// so a stop starts nothing.
    nonisolated func speechSynthesizer(
        _ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance
    ) {
        Task { @MainActor in self.playNext() }
    }
}
