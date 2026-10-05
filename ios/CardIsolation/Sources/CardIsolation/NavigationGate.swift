/// The kind of a navigation a card's view is asked to allow, as WebKit names it, read as a pure value
/// so the gate is decided and tested without WebKit (SPEC-349 R3, ADR-360 D3).
public enum NavigationKind: CaseIterable, Sendable {
    case link
    case formSubmit
    case backForward
    case reload
    case formResubmit
    case other
}

/// One navigation action: whether it targets the main frame, and its kind.
public struct NavigationRequest: Equatable, Sendable {
    public let isMainFrame: Bool
    public let kind: NavigationKind

    public init(isMainFrame: Bool, kind: NavigationKind) {
        self.isMainFrame = isMainFrame
        self.kind = kind
    }
}

/// What the gate answers for one action.
public enum NavigationDecision: Equatable, Sendable {
    case allow
    case cancel
}

/// The card view's navigation gate, layer L5: it allows exactly one action, the first main-frame
/// load, which is the string the factory hands the view, and cancels every later action of every
/// kind, in the main frame or a subframe. Two states and no way back, so a card can never replace
/// itself with a document the app did not build.
/// It does NOT stop a subresource load; the rule list (L3) does.
public struct NavigationGate: Sendable {
    public enum State: Equatable, Sendable {
        case awaitingFirstLoad
        case sealed
    }

    public private(set) var state: State = .awaitingFirstLoad

    public init() {}

    public mutating func decide(_ request: NavigationRequest) -> NavigationDecision {
        .allow
    }
}
