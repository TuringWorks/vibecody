// SharedAuth.swift — W1.2 widget-side accessor for the watch's
// pairing credentials.
//
// The main watch app (WatchAuthManager) writes the daemon endpoint
// and a refreshable Watch-Token JWT to the keychain. This widget
// extension reads the same keys; it can refresh the token via the
// /watch/refresh-token route but never re-pairs. If the user is not
// yet paired, both accessors return nil and the widget renders the
// placeholder ("No recap yet").
//
// Patent / privacy: read-only. Tokens are short-lived and
// device-bound; the widget doesn't transmit them anywhere except
// back to the user's own daemon.

import Foundation
import Security

enum SharedAuth {
    private static let kEndpoint = "vibecody.watch.endpoint"
    private static let kAccessToken = "vibecody.watch.access_token"
    private static let kAccessExpiry = "vibecody.watch.access_expiry"

    static func endpoint() -> String? {
        guard let str = readKeychainString(kEndpoint), !str.isEmpty else { return nil }
        return str
    }

    /// Returns a non-expired access token. Does not perform a network
    /// refresh — the widget extension's energy budget for refresh is
    /// tight, so we let the main app handle rotation. If the cached
    /// token is expired, returns nil and the widget shows the
    /// placeholder until the next user-initiated refresh.
    static func validToken() async -> String? {
        guard let token = readKeychainString(kAccessToken), !token.isEmpty else { return nil }
        if let expiryStr = readKeychainString(kAccessExpiry),
           let expiry = TimeInterval(expiryStr) {
            let now = Date().timeIntervalSince1970
            // 60s safety margin so the timeline-fetch RTT doesn't
            // straddle the expiry boundary.
            if now > expiry - 60 { return nil }
        }
        return token
    }

    // MARK: - Keychain helpers

    private static func readKeychainString(_ key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess,
              let data = result as? Data,
              let str = String(data: data, encoding: .utf8) else {
            return nil
        }
        return str
    }
}
