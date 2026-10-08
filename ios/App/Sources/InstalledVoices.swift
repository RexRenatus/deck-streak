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
    /// Every installed voice, its quality numbered from 0 as the adapter's are.
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
