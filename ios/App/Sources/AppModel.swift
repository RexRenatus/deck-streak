import Observation

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

    private let configuration: SyncConfiguration
    private let session = EngineSession()
    private let store: SyncCredentialStore
    private var started = false

    /// Reads the stored host key once, so a stored item reads as signed in at launch with no
    /// network call (R8).
    init(configuration: SyncConfiguration = .current) {
        let store = SyncCredentialStore(
            service: configuration.endpoint, account: configuration.user)
        self.configuration = configuration
        self.store = store
        user = configuration.user
        signedIn = (try? store.hostKey()) != nil
    }

    /// Opens the collection and lists its decks, once per model (R7).
    func start() async {
        guard !started else { return }
        started = true
        do {
            try await session.open()
            decks = try await session.decks()
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
            let hostKey = try await session.login(
                user: configuration.user, password: typed, endpoint: configuration.endpoint)
            try store.save(hostKey: hostKey)
            signedIn = true
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

    /// A failure's sentence: a refusal's own, or the error's description.
    private func sentence(_ error: any Error) -> String {
        (error as? Refusal)?.sentence ?? String(describing: error)
    }
}
