import Observation
import SwiftUI

/// What the shell calls on the engine besides a review: the open, the deck list and the login,
/// with the sync section's calls. The engine session makes them; a test hands the model its own
/// (SPEC-600 R7, ADR-620 D7).
protocol ShellEngine: SyncEngine {
    func open(arguments: [String]) async throws
    func decks() async throws -> [Deck]
    func login(user: String, password: String, endpoint: String) async throws -> String
}

/// The shell's state, held in a model and never in a view's state, so a size-class change or a
/// scene reconnect keeps it (SPEC-347 R6 to R8). It lives on the main actor and awaits the
/// session, which makes every engine call off it. It keeps no deck name and no password of its
/// own: the names are the engine's answer, and the typed password leaves it before the login.
@MainActor
@Observable
final class AppModel {
    /// The collection's decks, in the engine's order.
    var decks: [Deck] = []
    /// The deck the learner chose, kept here so it survives the split view's collapse.
    var chosenDeck: Deck?
    /// The chosen deck's review, which the detail pane shows (SPEC-348 R9).
    private(set) var review: ReviewModel?
    /// The open's or the list's refusal, as its sentence, shown in place of the list.
    var refusal: String?
    /// Whether the account sheet is shown.
    var showingAccount = false
    /// The configured sync user, shown as text and never typed.
    let user: String
    /// The password as it is typed. It is cleared before the login call is made.
    var password = ""
    /// Whether a login is running; the sheet's controls are disabled while it does.
    var signingIn = false
    /// Whether a host key is stored for the configured endpoint and user.
    var signedIn: Bool
    /// The engine's message after a refused login, or a refused sign-out's.
    var message = ""
    /// The account sheet's sync section, which syncs through the same session with the host key
    /// the one Keychain item holds (SPEC-503 R6).
    let sync: SyncModel
    /// The data screen, which lists and makes this device's backups through the same session
    /// (SPEC-516 R5).
    let data: DataModel

    private let configuration: SyncConfiguration
    /// The shell's engine: the app's one session, or a test's own (SPEC-600 R7).
    private let engine: any ShellEngine
    /// The session a review and the data screen take, the same session as `engine` in the app.
    private let session: EngineSession
    private let store: SyncCredentialStore
    /// When the app syncs on its own (SPEC-600 R1 to R5).
    private let moments: SyncMoments
    private var started = false

    /// The app's model, over one engine session.
    convenience init(configuration: SyncConfiguration = .current) {
        let session = EngineSession()
        self.init(configuration: configuration, engine: session, session: session)
    }

    /// Reads the stored host key once, so a stored item reads as signed in at launch with no
    /// network call (R8). `engine` takes the shell's calls and the sync's, and `session` a review's
    /// and the data screen's.
    init(configuration: SyncConfiguration, engine: any ShellEngine, session: EngineSession) {
        let store = SyncCredentialStore(
            service: configuration.endpoint, account: configuration.user)
        self.configuration = configuration
        self.engine = engine
        self.session = session
        self.store = store
        user = configuration.user
        signedIn = (try? store.hostKey()) != nil
        sync = SyncModel(engine: engine, store: store, endpoint: configuration.endpoint)
        data = DataModel(engine: session)
        moments = SyncMoments(sync: sync)
        sync.completed = { [weak self] in await self?.reread() }
    }

    /// Opens the collection and lists its decks, once per model (R7), then runs the launch's
    /// sync moment (SPEC-600 R1 (a)).
    func start() async {
        guard !started else { return }
        started = true
        do {
            try await engine.open(arguments: ProcessInfo.processInfo.arguments)
            decks = try await engine.decks()
            await moments.opened(signedIn: signedIn, reviewOpen: review != nil)
        } catch {
            refusal = sentence(error)
        }
    }

    /// Opens the chosen deck's review, which makes it the current deck (7,22) and shows its first
    /// card, or closes the review when no deck is chosen (SPEC-348 R9).
    func openReview() async {
        review = chosenDeck.map { ReviewModel(deck: $0, session: ReviewSession(engine: session)) }
        await review?.start()
    }

    /// The review screen's own close: the chosen deck is cleared, the review closes, and one
    /// sync runs (SPEC-600 R1 (d)).
    func closeReview() async {
        chosenDeck = nil
        review = nil
        await moments.closed(signedIn: signedIn)
    }

    /// The scene's phase, as the shell hands it over (SPEC-600 R1 (b)).
    func phaseChanged(_ phase: ScenePhase) async {
        await moments.entered(phase, signedIn: signedIn, reviewOpen: review != nil)
    }

    /// Signs in as the configured user at the configured endpoint with the typed password (R8).
    /// On success the host key is stored; on a refusal the engine's message shows and nothing is
    /// stored.
    func signIn() async {
        let typed = password
        password = ""
        message = ""
        signingIn = true
        defer { signingIn = false }
        do {
            let hostKey = try await engine.login(
                user: configuration.user, password: typed, endpoint: configuration.endpoint)
            try store.save(hostKey: hostKey)
            signedIn = true
            await moments.stored(signedIn: signedIn)
        } catch {
            message = sentence(error)
        }
    }

    /// Deletes the stored host key (R8).
    func signOut() {
        do {
            try store.delete()
            signedIn = false
        } catch {
            message = sentence(error)
        }
    }

    /// Lists the decks again after a sync that completes, and keeps the chosen deck only while
    /// the new list holds it; that clearing is no close, so it starts no sync (SPEC-600 R5).
    private func reread() async {
        do {
            decks = try await engine.decks()
            chosenDeck = moments.kept(chosenDeck, in: decks)
        } catch {
            refusal = sentence(error)
        }
    }

    /// A failure's sentence: a refusal's own, or the error's description.
    private func sentence(_ error: any Error) -> String {
        (error as? Refusal)?.sentence ?? String(describing: error)
    }
}
