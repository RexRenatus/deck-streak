import SwiftUI

/// The deck list's first-run inset (SPEC-680 R2): a loading state while the collection opens and
/// its decks are listed, then, with no refusal and no account signed in, an offer to bring the
/// collection from the account, whose button opens the account sheet, where the sign-in and the
/// sync are.
struct FirstRunOffer: View {
    let model: AppModel

    var body: some View {
        if model.loading {
            ProgressView("Opening your collection")
                .accessibilityIdentifier("first-run-loading")
                .foregroundStyle(Color.tideMuted)
                .frame(maxWidth: .infinity)
                .padding()
        } else if !model.signedIn && model.refusal == nil {
            VStack(spacing: 8) {
                Text("Your collection can come to this device from your account.")
                    .foregroundStyle(Color.tideInk)
                    .multilineTextAlignment(.center)
                Button("Sync from your account") { model.showingAccount = true }
                    .accessibilityIdentifier("first-run-sync")
                    .buttonStyle(.borderedProminent)
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color.tideCard)
            .tint(Color.tideAccent)
        }
    }
}
