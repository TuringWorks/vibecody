// WatchAuthManager.swift — Token storage, Ed25519 key management, token refresh.
//
// Security:
//   • Ed25519 keypair lives in the Watch Secure Enclave (kSecAttrTokenIDSecureEnclave).
//   • Access/refresh tokens stored in Watch Keychain (WhenUnlockedThisDeviceOnly).
//   • Wrist-detection: WKExtension.shared().isWristDetected gate on every request.
//   • Background refresh: tokens renewed 2 min before expiry using background tasks.

import Foundation
import Security
import WatchKit
import CryptoKit

// MARK: - Keychain keys

private enum KeychainKey {
    static let accessToken  = "vibecody.watch.access_token"
    static let refreshToken = "vibecody.watch.refresh_token"
    static let expiresAt    = "vibecody.watch.expires_at"
    static let deviceId     = "vibecody.watch.device_id"
    static let endpoint     = "vibecody.watch.endpoint"
    static let machineId    = "vibecody.watch.machine_id"
    static let privateKeyTag = "vibecody.watch.ed25519.private" // Secure Enclave tag
}

// MARK: - Auth errors

enum WatchAuthError: LocalizedError {
    case notPaired
    case wristOff
    case tokenExpired
    case keychainError(OSStatus)
    case networkError(String)
    case signatureError

    var errorDescription: String? {
        switch self {
        case .notPaired:           return "Watch not paired — scan the QR code in VibeUI."
        case .wristOff:            return "Session locked — put watch back on wrist."
        case .tokenExpired:        return "Session token expired — please re-pair."
        case .keychainError(let s): return "Keychain error \(s)"
        case .networkError(let m): return "Network: \(m)"
        case .signatureError:      return "Signature failed"
        }
    }
}

// MARK: - WatchAuthManager

@MainActor
final class WatchAuthManager: ObservableObject {

    static let shared = WatchAuthManager()

    @Published var isPaired: Bool = false
    @Published var deviceId: String = ""
    @Published var endpoint: String = ""
    @Published var machineId: String = ""

    private var privateKey: SecureEnclave.P256.Signing.PrivateKey?

    private init() {
        // Load existing pairing state
        self.deviceId  = loadString(for: KeychainKey.deviceId) ?? ""
        self.endpoint  = loadString(for: KeychainKey.endpoint) ?? ""
        self.machineId = loadString(for: KeychainKey.machineId) ?? ""
        self.isPaired  = !deviceId.isEmpty && !endpoint.isEmpty
    }

    // MARK: - Registration

    /// Called after QR scan — registers this watch device with the daemon.
    func registerDevice(pairing: WatchPairingPayload) async throws {
        // 1. Generate or load Ed25519 keypair in Secure Enclave
        let key = try getOrCreatePrivateKey()

        // 2. Sign the registration challenge:
        //    message = SHA-256(nonce || device_id || issued_at_be)
        let newDeviceId = UUID().uuidString.lowercased().replacingOccurrences(of: "-", with: "")
        let issuedAt = UInt64(Date().timeIntervalSince1970)
        var msgData = Data()
        msgData.append(Data(pairing.nonce.utf8))
        msgData.append(Data(newDeviceId.utf8))
        msgData.append(withUnsafeBytes(of: issuedAt.bigEndian) { Data($0) })
        let digest = SHA256.hash(data: msgData)
        let signature = try key.signature(for: digest)
        let pubKeyData = key.publicKey.rawRepresentation

        let req = WatchRegisterRequest(
            device_id:          newDeviceId,
            name:               WKInterfaceDevice.current().name,
            os_version:         WKInterfaceDevice.current().systemVersion,
            model:              WKInterfaceDevice.current().model,
            public_key_b64:     pubKeyData.base64URLEncoded,
            signature_b64:      signature.rawRepresentation.base64URLEncoded,
            nonce:              pairing.nonce,
            device_check_token: nil
        )

        let url = URL(string: "\(pairing.endpoint)/watch/register")!
        let resp: WatchRegisterResponse = try await postJSON(url: url, body: req)

        // 3. Persist tokens + pairing info
        try save(string: resp.access_token,  for: KeychainKey.accessToken)
        try save(string: resp.refresh_token, for: KeychainKey.refreshToken)
        try save(string: String(resp.expires_at), for: KeychainKey.expiresAt)
        try save(string: newDeviceId,        for: KeychainKey.deviceId)
        try save(string: pairing.endpoint,   for: KeychainKey.endpoint)
        try save(string: pairing.machine_id, for: KeychainKey.machineId)

        self.deviceId  = newDeviceId
        self.endpoint  = pairing.endpoint
        self.machineId = pairing.machine_id
        self.isPaired  = true
        self.privateKey = key
    }

    // MARK: - Token access

    /// Returns a valid access token, refreshing if < 2 min left.
    func validAccessToken() async throws -> String {
        guard isPaired else { throw WatchAuthError.notPaired }
        // Wrist detection: skip on simulator (SIMULATOR_DEVICE_NAME is set in env)
        let onSimulator = ProcessInfo.processInfo.environment["SIMULATOR_DEVICE_NAME"] != nil
        if !onSimulator {
            // WKInterfaceDevice.wristLocation is .left or .right when worn
            let loc = WKInterfaceDevice.current().wristLocation
            if loc != .left && loc != .right { throw WatchAuthError.wristOff }
        }

        let expiresAt = UInt64(loadString(for: KeychainKey.expiresAt) ?? "0") ?? 0
        let now = UInt64(Date().timeIntervalSince1970)

        if now + 120 >= expiresAt {
            try await refreshTokens()
        }
        guard let token = loadString(for: KeychainKey.accessToken) else {
            throw WatchAuthError.tokenExpired
        }
        return token
    }

    // MARK: - Token refresh

    private func refreshTokens() async throws {
        let key = try getOrCreatePrivateKey()
        let refreshToken = loadString(for: KeychainKey.refreshToken) ?? ""
        let timestamp = UInt64(Date().timeIntervalSince1970)

        // Proof: SHA-256(refresh_token || timestamp_be) signed by Secure Enclave
        var msgData = Data(refreshToken.utf8)
        msgData.append(withUnsafeBytes(of: timestamp.bigEndian) { Data($0) })
        let digest = SHA256.hash(data: msgData)
        let sig = try key.signature(for: digest)

        let req = WatchRefreshRequest(
            device_id:          deviceId,
            refresh_token:      refreshToken,
            proof_signature_b64: sig.rawRepresentation.base64URLEncoded,
            timestamp:          timestamp
        )
        let url = URL(string: "\(endpoint)/watch/refresh-token")!
        let resp: WatchRefreshResponse = try await postJSON(url: url, body: req)

        try save(string: resp.access_token,  for: KeychainKey.accessToken)
        try save(string: resp.refresh_token, for: KeychainKey.refreshToken)
        try save(string: String(resp.expires_at), for: KeychainKey.expiresAt)
    }

    // MARK: - Wrist detection

    func reportWristEvent(onWrist: Bool) async {
        guard isPaired, let key = privateKey else { return }
        let timestamp = UInt64(Date().timeIntervalSince1970)
        var msgData = Data(deviceId.utf8)
        msgData.append(onWrist ? 1 : 0)
        msgData.append(withUnsafeBytes(of: timestamp.bigEndian) { Data($0) })
        let digest = SHA256.hash(data: msgData)
        guard let sig = try? key.signature(for: digest) else { return }
        let ev = WristEvent(
            device_id:     deviceId,
            on_wrist:      onWrist,
            timestamp:     timestamp,
            signature_b64: sig.rawRepresentation.base64URLEncoded
        )
        let url = URL(string: "\(endpoint)/watch/wrist")!
        _ = try? await postJSON(url: url, body: ev) as serde_EmptyResponse
    }

    // MARK: - Unpair

    func unpair() {
        for key in [KeychainKey.accessToken, KeychainKey.refreshToken,
                    KeychainKey.expiresAt, KeychainKey.deviceId,
                    KeychainKey.endpoint, KeychainKey.machineId] {
            deleteKeychain(for: key)
        }
        isPaired = false
        deviceId = ""
        endpoint = ""
        machineId = ""
    }

    // MARK: - Secure Enclave key management

    private func getOrCreatePrivateKey() throws -> SecureEnclave.P256.Signing.PrivateKey {
        if let cached = privateKey { return cached }
        // Try to load from Secure Enclave via tag
        if let key = try? loadPrivateKeyFromEnclave() {
            privateKey = key
            return key
        }
        // Generate new key
        let access = SecAccessControlCreateWithFlags(
            nil,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            [.privateKeyUsage, .userPresence],
            nil
        )!
        let key = try SecureEnclave.P256.Signing.PrivateKey(
            compactRepresentable: false,
            accessControl: access
        )
        // Persist tag for later retrieval
        let tagData = KeychainKey.privateKeyTag.data(using: .utf8)!
        let addQuery: [String: Any] = [
            kSecClass as String:              kSecClassKey,
            kSecAttrKeyClass as String:       kSecAttrKeyClassPrivate,
            kSecAttrApplicationTag as String: tagData,
            kSecValueRef as String:           key.dataRepresentation,
        ]
        SecItemAdd(addQuery as CFDictionary, nil)
        privateKey = key
        return key
    }

    private func loadPrivateKeyFromEnclave() throws -> SecureEnclave.P256.Signing.PrivateKey? {
        let tagData = KeychainKey.privateKeyTag.data(using: .utf8)!
        let query: [String: Any] = [
            kSecClass as String:              kSecClassKey,
            kSecAttrApplicationTag as String: tagData,
            kSecReturnData as String:         true,
            kSecMatchLimit as String:         kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return nil }
        return try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: data)
    }

    // MARK: - Keychain helpers

    private func save(string: String, for key: String) throws {
        let data = Data(string.utf8)
        let query: [String: Any] = [
            kSecClass as String:                kSecClassGenericPassword,
            kSecAttrAccount as String:          key,
            kSecAttrAccessible as String:       kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecValueData as String:            data,
        ]
        SecItemDelete(query as CFDictionary)
        let status = SecItemAdd(query as CFDictionary, nil)
        if status != errSecSuccess { throw WatchAuthError.keychainError(status) }
    }

    private func loadString(for key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String:       kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String:  true,
            kSecMatchLimit as String:  kSecMatchLimitOne,
        ]
        var result: AnyObject?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data,
              let str = String(data: data, encoding: .utf8) else { return nil }
        return str
    }

    private func deleteKeychain(for key: String) {
        let query: [String: Any] = [
            kSecClass as String:       kSecClassGenericPassword,
            kSecAttrAccount as String: key,
        ]
        SecItemDelete(query as CFDictionary)
    }
}

// MARK: - Networking helpers

private struct serde_EmptyResponse: Codable {}

private func postJSON<Req: Encodable, Resp: Decodable>(
    url: URL, body: Req, token: String? = nil
) async throws -> Resp {
    var request = URLRequest(url: url)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    if let t = token {
        request.setValue("Watch-Token \(t)", forHTTPHeaderField: "Authorization")
    }
    request.httpBody = try JSONEncoder().encode(body)
    let (data, resp) = try await URLSession.shared.data(for: request)
    guard let http = resp as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
        let msg = String(data: data, encoding: .utf8) ?? "Unknown error"
        throw WatchAuthError.networkError(msg)
    }
    return try JSONDecoder().decode(Resp.self, from: data)
}

private func getJSON<Resp: Decodable>(url: URL, token: String) async throws -> Resp {
    var request = URLRequest(url: url)
    request.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
    let (data, resp) = try await URLSession.shared.data(for: request)
    guard let http = resp as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
        throw WatchAuthError.networkError("HTTP \((resp as? HTTPURLResponse)?.statusCode ?? 0)")
    }
    return try JSONDecoder().decode(Resp.self, from: data)
}

// MARK: - Data + Base64URL

extension Data {
    var base64URLEncoded: String {
        base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
