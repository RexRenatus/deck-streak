import SwiftUI

/// The sync section of the account sheet (SPEC-503 R4 to R7): a sync button; while the full-sync
/// choice is open, what this device has not synced, what the download loses, and the download and
/// cancel buttons. The native client offers no upload: an upload waits for the server's snapshot,
/// which this client does not read. The last sentence shows below.
struct SyncView: View {
    let model: SyncModel

    var body: some View {
        Section("Sync") {
            Button("Sync") {
                Task { await model.sync() }
            }
            .accessibilityIdentifier("sync")
            .disabled(model.syncing)
            if let unsynced = model.unsynced {
                Text("Not yet synced: \(unsynced.reviews) reviews")
                    .accessibilityIdentifier("sync-unsynced")
            }
            if let loss = model.counts?.download {
                Text("Download loses \(loss.reviews) reviews, \(loss.cards) cards, \(loss.notes) notes")
                    .accessibilityIdentifier("sync-download-loses")
                Button("Download") {
                    Task { await model.download() }
                }
                .accessibilityIdentifier("sync-download")
                .disabled(model.syncing)
                Button("Cancel") {
                    Task { await model.cancel() }
                }
                .accessibilityIdentifier("sync-cancel")
            }
            Text(model.message).accessibilityIdentifier("sync-message")
        }
    }
}
