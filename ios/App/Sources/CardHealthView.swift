import SwiftUI

/// The card info sheet over the review, on the iPhone and the iPad (SPEC-507 R14, R16): the shown
/// card's sections as the engine's adapter words them, each title a heading and each line one
/// element VoiceOver reads as it is shown, and a Close button. It judges no card and reads
/// nothing: its model hands it the sections, and closing it starts the answer clock again.
struct CardHealthView: View {
    let health: CardHealthModel

    var body: some View {
        NavigationStack {
            List {
                ForEach(Array(health.sections.enumerated()), id: \.offset) { _, section in
                    Section {
                        ForEach(Array(section.lines.enumerated()), id: \.offset) { _, line in
                            Text(line)
                        }
                    } header: {
                        Text(section.title)
                            .accessibilityAddTraits(.isHeader)
                    }
                }
            }
            .navigationTitle("Card info")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Close") {
                        Task { await health.close() }
                    }
                    .accessibilityIdentifier("card-info-close")
                }
            }
        }
    }
}
