// The sync credential's store against the simulator's own Keychain (SPEC-347 A10, R9). The test
// uses an endpoint and a user of its own, so it never meets the app's configured item, and reads
// the stored item's attributes back from the Keychain rather than from the store's query.
import DeckStreak
import Security
import XCTest

/// The two attributes R9 fixes on the stored item, as the Keychain reports them.
private struct Stored: Equatable {
    var accessibility: String?
    var synchronizable: Bool?
}

final class CredentialStoreTests: XCTestCase {
    private let endpoint = "https://sync.store-test.invalid/"
    private let user = "invalid-store-test-user"

    func test_a10_the_host_key_round_trips_and_sign_out_deletes_it() throws {
        let store = SyncCredentialStore(service: endpoint, account: user)
        try store.delete()

        // The round trip first: the host key saved is the host key read.
        try store.save(hostKey: "k1")
        XCTAssertEqual(try store.hostKey(), "k1", "A10: the host key round-trips through the store")

        // The same user under another endpoint holds no item.
        XCTAssertNil(
            try SyncCredentialStore(service: "https://other.invalid/", account: user).hostKey(),
            "A10: under another endpoint the host key reads as absent")

        // R9's attributes: readable after the first unlock, on this device only, never synchronized.
        XCTAssertEqual(
            stored(),
            Stored(
                accessibility: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String,
                synchronizable: false),
            "A10: the stored item carries R9's accessibility and is not synchronizable")

        // Sign-out deletes it.
        try store.delete()
        XCTAssertNil(try store.hostKey(), "A10: sign-out deletes the stored item")
    }

    /// The stored item's accessibility and synchronizable attributes, read whichever its
    /// synchronizable value is, or nil with the Keychain's status when no item is read.
    private func stored() -> Stored? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: endpoint,
            kSecAttrAccount as String: user,
            kSecAttrSynchronizable as String: kSecAttrSynchronizableAny,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var found: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &found)
        guard status == errSecSuccess, let attributes = found as? [String: Any] else {
            XCTFail("A10: the stored item's attributes were not read: status \(status)")
            return nil
        }
        return Stored(
            accessibility: attributes[kSecAttrAccessible as String] as? String,
            synchronizable: attributes[kSecAttrSynchronizable as String] as? Bool)
    }
}
