import SwiftUI

/// The review's undo control and its dialog (SPEC-515 R6): one button in the review's top row,
/// enabled from a card's question or answer while the adapter names a step it would undo, that
/// step's title its accessibility value. A press asks first; the dialog's Keep and its dismissal
/// write nothing, and only its confirmation undoes.
struct UndoControl: View {
    let model: ReviewModel
    @Bindable var undo: ReviewUndo

    var body: some View {
        Button {
            Task { await model.perform(.undo) }
        } label: {
            Label("Undo", systemImage: "arrow.uturn.backward")
                .frame(minWidth: 44, minHeight: 44)
        }
        .accessibilityIdentifier("undo")
        .accessibilityValue(undo.label)
        .disabled(undo.label.isEmpty || !idle)
        .sheet(
            item: $undo.offer,
            onDismiss: {
                Task { await model.perform(.keep) }
            },
            content: { offer in
                UndoDialogView(
                    offer: offer,
                    keep: { Task { await model.perform(.keep) } },
                    confirm: { Task { await model.perform(.confirmUndo) } })
            })
    }

    /// Whether an undo may be asked for: from a card's question or its answer, never while a call
    /// runs.
    private var idle: Bool {
        model.phase == .question || model.phase == .answer
    }
}
