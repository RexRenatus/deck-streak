import SwiftUI

/// Where a review stands (SPEC-348 R10, the schematic's state machine). A call in flight has its
/// own phase, written before the call is awaited, so the bar is disabled while it runs; a bury
/// or a flag runs in `marking` (SPEC-358 R5). A card whose face the engine withheld shows in
/// `withheld`, where only a bury or a flag starts (SPEC-380 R7).
enum ReviewPhase: Equatable, Sendable {
    case loading, question, withheld, revealing, answer, answering, marking, finished, refused
}

/// A rating as the answer bar names it, in the bar's order (SPEC-348 R10): Again and Good alone,
/// the two grades a press records, numbered as the engine numbers them, so a rating's number picks
/// its interval (SPEC-365 R13). The codec's own `Rating` numbers the engine's; this one is the
/// app's, so a view or the model can name a rating without importing the codec.
enum Rating: Int32, CaseIterable, Sendable { case again = 0, good = 2 }

/// The review screen in the detail pane (SPEC-348 R9): the deck's name, its counts and the
/// controls, then the card, then the answer bar inside the bottom safe area. The haptics are
/// the screen's: one impact when a rating is sent, one success when the designed end shows (R12).
struct ReviewView: View {
    @Bindable var model: ReviewModel
    let back: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            ReviewChrome(model: model)
            if model.phase == .finished {
                Text("Congratulations! You have finished this deck for now.")
                    .font(.title3)
                    .multilineTextAlignment(.center)
                    .padding()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                CardFaceView(document: model.face.document)
            }
        }
        .safeAreaInset(edge: .bottom, spacing: 0) {
            AnswerBar(model: model, back: back)
        }
        .sensoryFeedback(.impact, trigger: model.answered)
        .sensoryFeedback(.success, trigger: model.phase) { _, phase in phase == .finished }
        .sheet(isPresented: $model.showingVoices) {
            VoicePickerView(model: model)
        }
    }
}
