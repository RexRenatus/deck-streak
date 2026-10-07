/// A voice installed on the device, as the voice picker and the engine's choice read it; its
/// quality is 0 default, 1 enhanced and 2 premium (SPEC-348 R14).
struct InstalledVoice: Equatable, Sendable {
    let identifier: String
    let name: String
    let language: String
    let quality: Int
}

/// The device's installed voices (SPEC-348 R13, R14).
enum InstalledVoices {
    /// Every installed voice.
    static func all() -> [InstalledVoice] {
        []
    }
}
