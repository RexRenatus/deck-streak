import SwiftUI

/// The account sheet (SPEC-347 R8): the configured sync user as text, one secure password field and
/// a sign-in button; once signed in, "Signed in as" the user and a sign-out button. While a login
/// runs the controls are disabled, and sign-out is not offered until a host key is stored. The
/// engine's message after a refused login shows below.
struct AccountView: View {
    @Bindable var model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Text(model.user).accessibilityIdentifier("account-user")
                if model.signedIn {
                    Text("Signed in as \(model.user)").accessibilityIdentifier("account-status")
                    Button("Sign out") { model.signOut() }
                        .accessibilityIdentifier("sign-out")
                } else {
                    SecureField("Password", text: $model.password)
                        .accessibilityIdentifier("password")
                        .disabled(model.signingIn)
                    Button("Sign in") {
                        Task { await model.signIn() }
                    }
                    .accessibilityIdentifier("sign-in")
                    .disabled(model.signingIn)
                }
                Text(model.message).accessibilityIdentifier("account-message")
            }
            .navigationTitle("Account")
            .toolbar {
                Button("Done") { dismiss() }
                    .accessibilityIdentifier("done")
            }
        }
    }
}
