import Foundation
import Security

/// A Keychain call's failure, by its status alone, so a refusal never names the item it was for.
public struct KeychainRefusal: Error, Equatable {
    public let status: OSStatus
}

/// The one Keychain item that holds the sync host key (SPEC-347 R9): a generic password whose
/// service is the configured endpoint and whose account is the configured user. It is readable
/// after the first unlock and never leaves this device: never synchronized, never restored to
/// another, and in no shared access group. The password is never stored, here or anywhere. This
/// first version stores nothing and reads nothing, so the store's test is red against it before
/// the store is written.
public struct SyncCredentialStore: Sendable {
    public let service: String
    public let account: String

    public init(service: String, account: String) {
        self.service = service
        self.account = account
    }

    /// Stores the host key, replacing any stored before it.
    public func save(hostKey: String) throws {}

    /// The stored host key, or nil when none is stored.
    public func hostKey() throws -> String? {
        nil
    }

    /// Deletes the stored host key; deleting none is not a failure.
    public func delete() throws {}
}
