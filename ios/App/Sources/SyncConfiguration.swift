import Foundation

/// The sync server the app signs in to and the user it signs in as, read from the Info.plist,
/// which the build fills from `App.xcconfig` or the lane's own include (SPEC-347 R5, R14). The app
/// reads no other configuration.
public struct SyncConfiguration: Sendable {
    public let endpoint: String
    public let user: String

    public init(endpoint: String, user: String) {
        self.endpoint = endpoint
        self.user = user
    }

    /// This build's values. A key the Info.plist lacks reads as empty, which the engine's endpoint
    /// guard refuses before any network call.
    public static let current = SyncConfiguration(
        endpoint: Bundle.main.object(forInfoDictionaryKey: "DSSyncEndpoint") as? String ?? "",
        user: Bundle.main.object(forInfoDictionaryKey: "DSSyncUser") as? String ?? "")
}
