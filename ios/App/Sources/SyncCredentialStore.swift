import Foundation
import Security

/// A Keychain call's failure, by its status alone, so a refusal never names the item it was for.
public struct KeychainRefusal: Error, Equatable {
    public let status: OSStatus
}

/// The one Keychain item that holds the sync host key (SPEC-347 R9): a generic password whose
/// service is the configured endpoint and whose account is the configured user. It is readable
/// after the first unlock and never leaves this device: never synchronized, never restored to
/// another, and in no shared access group. The password is never stored, here or anywhere.
public struct SyncCredentialStore: Sendable {
    public let service: String
    public let account: String

    public init(service: String, account: String) {
        self.service = service
        self.account = account
    }

    /// The item's identity, which every call names: a generic password under this endpoint and
    /// user that is not synchronized, so no call reaches a synchronized item of the same name.
    private var query: [CFString: Any] {
        [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: account,
            kSecAttrSynchronizable: kCFBooleanFalse!,
        ]
    }

    /// Stores the host key, replacing any stored before it.
    public func save(hostKey: String) throws {
        try delete()
        var item = query
        item[kSecValueData] = Data(hostKey.utf8)
        item[kSecAttrAccessible] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        try check(SecItemAdd(item as CFDictionary, nil))
    }

    /// The stored host key, or nil when none is stored.
    public func hostKey() throws -> String? {
        var lookup = query
        lookup[kSecReturnData] = true
        lookup[kSecMatchLimit] = kSecMatchLimitOne
        var found: CFTypeRef?
        let status = SecItemCopyMatching(lookup as CFDictionary, &found)
        if status == errSecItemNotFound {
            return nil
        }
        try check(status)
        return (found as? Data).map { String(decoding: $0, as: UTF8.self) }
    }

    /// Deletes the stored host key; deleting none is not a failure.
    public func delete() throws {
        let status = SecItemDelete(query as CFDictionary)
        if status != errSecItemNotFound {
            try check(status)
        }
    }

    /// A Keychain status other than success, as a refusal.
    private func check(_ status: OSStatus) throws {
        guard status == errSecSuccess else {
            throw KeychainRefusal(status: status)
        }
    }
}
