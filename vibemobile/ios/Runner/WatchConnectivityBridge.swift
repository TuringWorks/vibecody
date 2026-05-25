// WatchConnectivityBridge.swift — iOS phone-relay for Apple Watch.
//
// The watch may not have direct network access (no LTE, off-WiFi). When
// that happens it sends requests via WCSession.sendMessage and this
// bridge relays them to the VibeCody daemon over the iPhone's network.
//
// Activated from AppDelegate.didFinishLaunchingWithOptions so the
// WCSession is live whenever the iPhone app is running (foreground or
// background — WCSession messages with a replyHandler wake the app for
// up to 30s of background processing).
//
// Credentials are written by the Flutter side via MethodChannel
// "vibecody.relay/credentials" (see vibemobile/lib/services/relay_bridge.dart)
// into Keychain under kSecAttrService = "com.turingworks.vibecody.companion".

import Foundation
import WatchConnectivity

final class WatchConnectivityBridge: NSObject, WCSessionDelegate, @unchecked Sendable {

    static let shared = WatchConnectivityBridge()
    static let keychainService = "com.turingworks.vibecody.companion"

    private let session = WCSession.default

    private override init() {
        super.init()
        if WCSession.isSupported() {
            session.delegate = self
            session.activate()
        }
    }

    /// Idempotent — AppDelegate calls this on launch. The init above
    /// also activates, so this exists mostly for explicit lifecycle.
    func activate() {
        if WCSession.isSupported() && session.activationState != .activated {
            session.delegate = self
            session.activate()
        }
    }

    // MARK: - WCSessionDelegate

    func session(
        _ session: WCSession,
        activationDidCompleteWith activationState: WCSessionActivationState,
        error: Error?
    ) {
        if let error = error {
            NSLog("[VibeRelay] WCSession activation failed: \(error.localizedDescription)")
        } else {
            NSLog("[VibeRelay] WCSession activated (state=\(activationState.rawValue))")
        }
    }

    func sessionDidBecomeInactive(_ session: WCSession) {}
    func sessionDidDeactivate(_ session: WCSession) { session.activate() }

    /// Receives a relay request from the watch and forwards it to the daemon.
    func session(
        _ session: WCSession,
        didReceiveMessage message: [String: Any],
        replyHandler: @escaping ([String: Any]) -> Void
    ) {
        guard let action = message["action"] as? String else {
            replyHandler(["error": "Missing 'action' field"])
            return
        }
        Task {
            do {
                let result = try await relay(action: action, message: message)
                replyHandler(result)
            } catch {
                replyHandler(["error": error.localizedDescription])
            }
        }
    }

    // MARK: - Relay logic

    private func relay(action: String, message: [String: Any]) async throws -> [String: Any] {
        let creds = try loadCredentials()
        let endpoint = creds.baseUrl

        switch action {
        case "dispatch":
            let content    = message["content"] as? String ?? ""
            let sessionId  = message["session_id"] as? String
            let nonce      = message["nonce"] as? String ?? UUID().uuidString
            let timestamp  = UInt64(Date().timeIntervalSince1970)

            var body: [String: Any] = [
                "content":   content,
                "nonce":     nonce,
                "timestamp": timestamp,
            ]
            if let sid = sessionId { body["session_id"] = sid }

            let data = try JSONSerialization.data(withJSONObject: body)
            let resp = try await httpPost(
                url:   URL(string: "\(endpoint)/watch/dispatch")!,
                body:  data,
                token: creds.bearerToken
            )
            return ["data": resp]

        case "sessions":
            let resp = try await httpGet(
                url:   URL(string: "\(endpoint)/watch/sessions")!,
                token: creds.bearerToken
            )
            return ["data": resp]

        case "messages":
            let sid = message["session_id"] as? String ?? ""
            let resp = try await httpGet(
                url:   URL(string: "\(endpoint)/watch/sessions/\(sid)/messages")!,
                token: creds.bearerToken
            )
            return ["data": resp]

        default:
            throw BridgeError.unknownAction(action)
        }
    }

    // MARK: - HTTP helpers
    //
    // The daemon accepts `Authorization: Bearer <token>` on /watch/dispatch,
    // /watch/sessions, /watch/sessions/:id/messages (extract_any_auth in
    // vibecli/vibecli-cli/src/watch_bridge.rs). The phone-relay uses Bearer
    // directly — the watch's own JWT never crosses the air gap.

    private func httpPost(url: URL, body: Data, token: String) async throws -> Data {
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        req.httpBody = body
        let (data, _) = try await URLSession.shared.data(for: req)
        return data
    }

    private func httpGet(url: URL, token: String) async throws -> Data {
        var req = URLRequest(url: url)
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        let (data, _) = try await URLSession.shared.data(for: req)
        return data
    }

    // MARK: - Keychain — credentials written by Flutter via RelayBridge

    struct Credentials {
        let baseUrl: String
        let bearerToken: String
        let deviceId: String
        let machineId: String
    }

    private func loadCredentials() throws -> Credentials {
        let baseUrl     = try loadKeychainString("base_url")
        let bearerToken = try loadKeychainString("bearer_token")
        let deviceId    = (try? loadKeychainString("device_id")) ?? ""
        let machineId   = (try? loadKeychainString("machine_id")) ?? ""
        return Credentials(baseUrl: baseUrl, bearerToken: bearerToken, deviceId: deviceId, machineId: machineId)
    }

    private func loadKeychainString(_ account: String) throws -> String {
        let query: [String: Any] = [
            kSecClass as String:        kSecClassGenericPassword,
            kSecAttrService as String:  Self.keychainService,
            kSecAttrAccount as String:  account,
            kSecReturnData as String:   true,
            kSecMatchLimit as String:   kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess,
              let data = result as? Data,
              let str  = String(data: data, encoding: .utf8) else {
            throw BridgeError.keychainMissing(account)
        }
        return str
    }
}

// MARK: - Credential writer (called from AppDelegate's MethodChannel handler)

enum RelayCredentialStore {
    private static let service = WatchConnectivityBridge.keychainService

    /// Writes the active machine credentials to Keychain. Called when
    /// Flutter invokes MethodChannel "vibecody.relay/credentials" → setActiveMachine.
    static func writeActiveMachine(
        baseUrl: String,
        bearerToken: String,
        deviceId: String,
        machineId: String
    ) throws {
        try writeKeychain(account: "base_url",     value: baseUrl)
        try writeKeychain(account: "bearer_token", value: bearerToken)
        try writeKeychain(account: "device_id",    value: deviceId)
        try writeKeychain(account: "machine_id",   value: machineId)
    }

    /// Clears cached credentials. Called on sign-out / unpair.
    static func clearActiveMachine() {
        for account in ["base_url", "bearer_token", "device_id", "machine_id"] {
            deleteKeychain(account: account)
        }
    }

    private static func writeKeychain(account: String, value: String) throws {
        let data = value.data(using: .utf8) ?? Data()
        let query: [String: Any] = [
            kSecClass as String:       kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        let attributes: [String: Any] = [
            kSecValueData as String:   data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
        ]
        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecItemNotFound {
            var insert = query
            insert.merge(attributes) { _, new in new }
            let addStatus = SecItemAdd(insert as CFDictionary, nil)
            if addStatus != errSecSuccess {
                throw BridgeError.keychainWriteFailed(account, addStatus)
            }
        } else if updateStatus != errSecSuccess {
            throw BridgeError.keychainWriteFailed(account, updateStatus)
        }
    }

    private static func deleteKeychain(account: String) {
        let query: [String: Any] = [
            kSecClass as String:       kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        SecItemDelete(query as CFDictionary)
    }
}

// MARK: - Errors

enum BridgeError: LocalizedError {
    case unknownAction(String)
    case keychainMissing(String)
    case keychainWriteFailed(String, OSStatus)

    var errorDescription: String? {
        switch self {
        case .unknownAction(let a):
            return "Unknown relay action: \(a)"
        case .keychainMissing(let k):
            return "Keychain key missing: \(k) — phone not paired yet?"
        case .keychainWriteFailed(let k, let s):
            return "Keychain write failed for \(k) (OSStatus=\(s))"
        }
    }
}
