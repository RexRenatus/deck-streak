import SwiftUI

/// The voice picker sheet (SPEC-348 R14): for each language the shown face speaks, "System
/// default" and then the installed voices the adapter offers, each by its name and its quality in
/// words. The chosen one is marked in words beside it and in its accessible value; a language with
/// no installed voice says so in a sentence.
struct VoicePickerView: View {
    let model: ReviewModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List(model.voices, id: \.language) { language in
                Section(language.language) {
                    row("System default", nil, language)
                    ForEach(language.options, id: \.identifier) { voice in
                        row("\(voice.name), \(voice.qualityInWords)", voice.identifier, language)
                    }
                    if language.options.isEmpty {
                        Text("No \(language.language) voice is installed.")
                            .accessibilityIdentifier("no-voice-\(language.language)")
                    }
                }
            }
            .navigationTitle("Voices")
            .toolbar {
                Button("Done") { dismiss() }
            }
            .task { await model.readVoices() }
        }
    }

    /// One choice: a button named by its title alone, which records `identifier` for the
    /// language, nil for the system's default.
    private func row(
        _ title: String, _ identifier: String?, _ language: VoiceLanguage
    ) -> some View {
        let mark = language.chosen == identifier ? "Chosen" : ""
        return Button {
            Task { await model.choose(voice: identifier, for: language.language) }
        } label: {
            HStack {
                Text(title)
                Spacer()
                Text(mark).foregroundStyle(.secondary).accessibilityHidden(true)
            }
        }
        .accessibilityValue(mark)
    }
}
