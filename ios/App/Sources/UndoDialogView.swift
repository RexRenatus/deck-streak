import SwiftUI

/// The undo's dialog (SPEC-515 R6): the adapter's title, then its lines, the card as one line of
/// text, the answer and the state the card goes back to, or what the undo changes, each shown as
/// text and never as a page; then Keep, which has the keyboard's and the reader's focus, and the
/// confirmation. Nothing here decides what an offer says.
struct UndoDialogView: View {
    let offer: UndoDialog
    let keep: () -> Void
    let confirm: () -> Void

    @FocusState private var keyboardOnKeep: Bool
    @AccessibilityFocusState private var readerOnKeep: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(verbatim: offer.title)
                .font(.headline)
                .accessibilityAddTraits(.isHeader)
            ForEach(Array(offer.lines.enumerated()), id: \.offset) { _, line in
                Text(verbatim: line)
            }
            HStack(spacing: 12) {
                Button(action: keep) {
                    Text(verbatim: offer.keep).frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.borderedProminent)
                .focused($keyboardOnKeep)
                .accessibilityFocused($readerOnKeep)
                .accessibilityIdentifier("undo-keep")
                Button(action: confirm) {
                    Text(verbatim: offer.confirm).frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("undo-confirm")
            }
        }
        .padding()
        .defaultFocus($keyboardOnKeep, true)
        .onAppear {
            keyboardOnKeep = true
            readerOnKeep = true
        }
        .presentationDetents([.medium])
    }
}
