import AVFAudio

/// A voice installed on the device, as the voice picker and the engine's choice read it; its
/// quality is 0 default, 1 enhanced and 2 premium (SPEC-348 R14).
struct InstalledVoice: Equatable, Sendable {
    let identifier: String
    let name: String
    let language: String
    let quality: Int

    /// The quality in a word, as the picker shows it.
    var qualityInWords: String {
        ["Default", "Enhanced", "Premium"][min(max(quality, 0), 2)]
    }
}

/// The device's installed voices, and the voice a line of speech speaks in (SPEC-348 R13, R14).
enum InstalledVoices {
    /// Every installed voice, its quality numbered from 0 as the adapter's are. The device can be
    /// slow to answer, so the app reads it only through `InstalledVoiceList`, off the main actor
    /// (SPEC-687 R1).
    static func all() -> [InstalledVoice] {
        AVSpeechSynthesisVoice.speechVoices().map { voice in
            InstalledVoice(
                identifier: voice.identifier, name: voice.name, language: voice.language,
                quality: voice.quality.rawValue - 1)
        }
    }

    /// The voice `identifier` names when it is installed, otherwise the language's own.
    static func voice(_ identifier: String?, language: String) -> AVSpeechSynthesisVoice? {
        identifier.flatMap(AVSpeechSynthesisVoice.init(identifier:))
            ?? AVSpeechSynthesisVoice(language: language)
    }
}

/// The installed voices as far as the device has listed them, none until its list is in. The list
/// is read once, off the main actor, and no step of a review waits for it, so a line spoken before
/// it is in speaks in its language's own voice (SPEC-687 R1 to R3, ADR-707).
@MainActor
final class InstalledVoiceList {
    /// The device's list, read once per process from the first time a review asks for it.
    static let device = InstalledVoiceList(read: { InstalledVoices.all() })

    /// The voices listed so far.
    private(set) var voices: [InstalledVoice] = []
    /// The one read, which ends once its voices are kept.
    private(set) var reading: Task<Void, Never>?

    /// Starts `read` off the main actor, and keeps what it lists when it returns.
    init(read: @escaping @Sendable () -> [InstalledVoice]) {
        let listed = Task.detached(priority: .userInitiated) { read() }
        reading = Task { [weak self] in
            let voices = await listed.value
            self?.voices = voices
        }
    }

    /// The device's voices listed so far, the review's default, which never waits (SPEC-687 R2).
    static func listed() -> [InstalledVoice] {
        device.voices
    }
}
