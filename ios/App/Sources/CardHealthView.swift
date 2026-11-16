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
                                .foregroundStyle(Color.tideInk)
                        }
                    } header: {
                        Text(section.title)
                            .foregroundStyle(Color.tideMuted)
                            .accessibilityAddTraits(.isHeader)
                    }
                    .listRowBackground(Color.tideCard)
                    .listRowSeparatorTint(Color.tideBorder)
                }
            }
            .scrollContentBackground(.hidden)
            .background(Color.tideBackground)
            .navigationTitle("Card info")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Close") {
                        Task { await health.close() }
                    }
                    .accessibilityIdentifier("card-info-close")
                }
            }
        }
        .tint(Color.tideAccent)
    }
}

/// The one least height of a control, which every control frame reads: 44 pt on iOS, the
/// touch target SPEC-348 holds, and 28 pt on macOS, the Mac idiom's control (SPEC-679).
enum ControlHeight {
    static var least: CGFloat {
        #if os(macOS)
        return 28
        #else
        return 44
        #endif
    }
}
