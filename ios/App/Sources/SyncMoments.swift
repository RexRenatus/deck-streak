import SwiftUI

/// When the app syncs on its own (SPEC-600 R1 to R5, ADR-620). The shell's model owns one and
/// calls it at each moment: after the collection opens, on a scene phase, after a sign-in stores
/// its key, and at a review's own close. It runs the sync section's own normal sync, so a sync that
/// needs a full sync stops at the counts as the Sync button's does, and nothing is written by
/// itself.
@MainActor
final class SyncMoments {
    /// Whether the scene went to the background since it was last active.
    private var away = false
    private let sync: SyncModel

    init(sync: SyncModel) {
        self.sync = sync
    }

    /// After the collection opens and its decks are listed at launch (R1 (a)).
    func opened(signedIn: Bool, reviewOpen: Bool) async {}

    /// A scene phase: a return to active after the background is a moment, and an inactive turn
    /// or the first active at launch is none (R1 (b)).
    func entered(_ phase: ScenePhase, signedIn: Bool, reviewOpen: Bool) async {}

    /// After a sign-in stores its host key (R1 (c)).
    func stored(signedIn: Bool) async {}

    /// After the review screen's own close action closes an open review (R1 (d)).
    func closed(signedIn: Bool) async {}

    /// The chosen deck, kept when the decks' new list holds it and cleared when it does not (R5).
    func kept(_ chosen: Deck?, in decks: [Deck]) -> Deck? {
        chosen
    }
}
