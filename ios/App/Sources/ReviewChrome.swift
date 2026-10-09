import SwiftUI

/// The review screen's top, above the card (SPEC-348 R9, R13, R14): the deck's name; the replay,
/// stop and voice buttons, and the bury and flag buttons, the flag's title beside its icon
/// (SPEC-358 R5); the queue's new, learning and review counts, each a number with its
/// word; a line naming how many media files the face could not show; and one line per note, a
/// refusal's sentence or a sound the player refused.
struct ReviewChrome: View {
    @Bindable var model: ReviewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                // The detail pane's heading, which SPEC-347 A11 reads as the detail.
                Text(model.deck.name)
                    .font(.headline)
                    .accessibilityIdentifier("detail")
                Spacer()
                Button {
                    Task { await model.perform(.replay) }
                } label: {
                    Label("Replay", systemImage: "arrow.counterclockwise")
                        .frame(minWidth: 44, minHeight: 44)
                }
                Button {
                    Task { await model.perform(.stop) }
                } label: {
                    Label("Stop", systemImage: "stop.fill")
                        .frame(minWidth: 44, minHeight: 44)
                }
                Button {
                    model.showingVoices = true
                } label: {
                    Label("Voices", systemImage: "person.wave.2")
                        .frame(minWidth: 44, minHeight: 44)
                }
                Button {
                    Task { await model.perform(.bury) }
                } label: {
                    Label("Bury", systemImage: "eye.slash")
                        .frame(minWidth: 44, minHeight: 44)
                }
                .accessibilityIdentifier("bury")
                .disabled(!idle)
                Button {
                    Task { await model.perform(.flag) }
                } label: {
                    Label(flagTitle.title, systemImage: flagTitle.symbol)
                        .labelStyle(.titleAndIcon)
                        .frame(minWidth: 44, minHeight: 44)
                }
                .accessibilityIdentifier("flag")
                .disabled(!idle)
            }
            .labelStyle(.iconOnly)
            HStack(spacing: 16) {
                Text("\(model.counts.new) new").accessibilityIdentifier("count-new")
                Text("\(model.counts.learning) learning").accessibilityIdentifier("count-learning")
                Text("\(model.counts.review) to review").accessibilityIdentifier("count-review")
            }
            .font(.subheadline)
            if !model.face.omitted.isEmpty {
                Text("Media files not shown: \(model.face.omitted.count)")
                    .font(.footnote)
            }
            ForEach(Array(model.notes.enumerated()), id: \.offset) { _, note in
                Text(note).font(.footnote)
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
    }

    /// Whether a bury or a flag may start: from a card's question or its answer, never while a
    /// call runs (SPEC-358 R5).
    private var idle: Bool {
        model.phase == .question || model.phase == .answer
    }

    /// The flag button's title and symbol: an icon with a text label, never a colour alone.
    private var flagTitle: (title: String, symbol: String) {
        model.flagged ? ("Flagged red", "flag.fill") : ("Flag", "flag")
    }
}
