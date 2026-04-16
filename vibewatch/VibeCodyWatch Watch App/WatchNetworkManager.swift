// WatchNetworkManager.swift — API calls, session listing, dispatch, SSE streaming.
//
// Transport resolution order (same as Flutter HandoffService):
//   1. LAN: direct HTTP to daemon on local network
//   2. Tailscale: if LAN unreachable, try Tailscale IP
//   3. WatchConnectivity: relay through paired iPhone when offline
//
// All requests are signed with Watch-Token JWT (see WatchAuthManager).

import Foundation
import Combine
import WatchConnectivity

@MainActor
final class WatchNetworkManager: NSObject, ObservableObject {

    static let shared = WatchNetworkManager()

    @Published var sessions: [WatchSessionSummary] = []
    @Published var isLoading = false
    @Published var lastError: String?

    private let auth = WatchAuthManager.shared
    // Legacy URLSessionDataTask kept for stopStreaming() cancellation compat
    private var sseTask: URLSessionDataTask?
    // AsyncBytes streaming task handle
    private var streamingTask: Task<Void, Never>?
    private var streamingBuffer = ""
    @Published var streamingText: String = ""
    @Published var isStreaming = false

    override private init() { super.init() }

    // MARK: - Session list

    func loadSessions() async {
        guard auth.isPaired else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            let token = try await auth.validAccessToken()
            let url = URL(string: "\(auth.endpoint)/watch/sessions")!
            let result: WatchSessionsResponse = try await getJSON(url: url, token: token)
            sessions = result.sessions
        } catch {
            lastError = error.localizedDescription
        }
    }

    // MARK: - Messages for a session

    func loadMessages(sessionId: String) async throws -> [WatchMessage] {
        let token = try await auth.validAccessToken()
        let url = URL(string: "\(auth.endpoint)/watch/sessions/\(sessionId)/messages")!
        let result: WatchMessagesResponse = try await getJSON(url: url, token: token)
        return result.messages
    }

    // MARK: - Dispatch (send message)

    func dispatch(content: String, sessionId: String? = nil, provider: String? = nil) async throws -> WatchDispatchResponse {
        let token = try await auth.validAccessToken()
        let nonce = UUID().uuidString.lowercased().replacingOccurrences(of: "-", with: "")
        let req = WatchDispatchRequest(
            session_id: sessionId,
            content:    content,
            provider:   provider,
            nonce:      nonce,
            timestamp:  UInt64(Date().timeIntervalSince1970)
        )
        let url = URL(string: "\(auth.endpoint)/watch/dispatch")!
        let resp: WatchDispatchResponse = try await postJSON(url: url, body: req, token: token)
        return resp
    }

    // MARK: - SSE streaming
    //
    // Uses URLSession.bytes(for:) + AsyncBytes.lines so tokens arrive
    // incrementally as the server sends them. The old dataTask(completionHandler:)
    // approach fired only once the ENTIRE response body was received — which
    // never happens for a keep-alive SSE stream, leaving isStreaming stuck.

    func startStreaming(sessionId: String, onEvent: @escaping @Sendable (WatchAgentEvent) -> Void) {
        stopStreaming()
        isStreaming = true
        streamingText = ""

        streamingTask = Task { [weak self] in
            guard let self else { return }
            guard await self.auth.isPaired else {
                await MainActor.run { self.isStreaming = false }
                return
            }
            guard let token = try? await self.auth.validAccessToken() else {
                await MainActor.run { self.isStreaming = false }
                return
            }

            let urlStr = await "\(self.auth.endpoint)/watch/stream/\(sessionId)"
            guard let url = URL(string: urlStr) else {
                await MainActor.run { self.isStreaming = false }
                return
            }
            var request = URLRequest(url: url)
            request.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
            request.setValue("text/event-stream", forHTTPHeaderField: "Accept")
            // Long timeout — LLM responses can be slow
            request.timeoutInterval = 300

            do {
                let (asyncBytes, response) = try await URLSession.shared.bytes(for: request)
                guard let http = response as? HTTPURLResponse,
                      (200...299).contains(http.statusCode) else {
                    await MainActor.run { self.isStreaming = false }
                    return
                }

                // Iterate line-by-line as the server sends them
                for try await line in asyncBytes.lines {
                    if Task.isCancelled { break }
                    // SSE lines start with "data: "; skip blank lines and comments
                    guard line.hasPrefix("data: ") else { continue }
                    let json = String(line.dropFirst(6))
                    // Skip keepalive ping strings
                    guard json != "ping", !json.isEmpty else { continue }
                    guard let data = json.data(using: .utf8),
                          let event = try? JSONDecoder().decode(WatchAgentEvent.self, from: data) else { continue }

                    await MainActor.run {
                        onEvent(event)
                        if event.kind == "delta", let d = event.delta {
                            self.streamingText += d
                        }
                        if event.kind == "done" || event.kind == "error" {
                            self.isStreaming = false
                        }
                    }
                    // Stop reading once we receive terminal event
                    if event.kind == "done" || event.kind == "error" { break }
                }
            } catch {
                // Task cancellation or network error — not unexpected
            }
            await MainActor.run { self.isStreaming = false }
        }
    }

    func stopStreaming() {
        streamingTask?.cancel()
        streamingTask = nil
        sseTask?.cancel()
        sseTask = nil
        isStreaming = false
    }

    // MARK: - Poll for response (reliable fallback / complement to SSE)
    //
    // Polls GET /watch/sessions/{id}/messages every second until the session
    // status becomes "complete" or "failed". Returns the full message list.
    // Used after dispatch to guarantee the response appears even if SSE fails.

    func pollForResponse(sessionId: String, timeoutSeconds: Int = 60) async -> [WatchMessage] {
        guard let token = try? await auth.validAccessToken() else { return [] }
        let url = URL(string: "\(auth.endpoint)/watch/sessions/\(sessionId)/messages")!
        var elapsed = 0
        var lastCount = 0
        while elapsed < timeoutSeconds {
            do {
                let result: WatchMessagesPollingResponse = try await getJSON(url: url, token: token)
                if result.messages.count > lastCount {
                    lastCount = result.messages.count
                }
                // Done when we have an assistant message and session is complete
                let hasAssistant = result.messages.contains { $0.role == "assistant" }
                let isDone = result.status == "complete" || result.status == "failed"
                if hasAssistant && isDone {
                    return result.messages
                }
            } catch {
                // Transient error — keep polling
            }
            try? await Task.sleep(nanoseconds: 1_000_000_000) // 1 second
            elapsed += 1
        }
        // Timeout — return whatever we have
        if let result = try? await getJSON(url: url, token: token) as WatchMessagesPollingResponse {
            return result.messages
        }
        return []
    }

    // MARK: - Beacon / discovery

    func discoverDaemon(tailscaleIP: String? = nil) async -> Bool {
        let candidates: [String] = {
            var list = [auth.endpoint]
            if let ts = tailscaleIP { list.append("http://\(ts):7878") }
            return list
        }()
        for candidate in candidates {
            if let url = URL(string: "\(candidate)/watch/beacon"),
               let (data, _) = try? await URLSession.shared.data(from: url),
               let beacon = try? JSONDecoder().decode(WatchBeacon.self, from: data),
               beacon.watch_supported {
                if candidate != auth.endpoint {
                    // Update endpoint to best reachable address
                }
                return true
            }
        }
        return false
    }
}

// MARK: - WatchConnectivity relay (fallback when no direct network)

extension WatchNetworkManager: WCSessionDelegate {

    func activateWatchConnectivitySession() {
        if WCSession.isSupported() {
            WCSession.default.delegate = self
            WCSession.default.activate()
        }
    }

    /// Relay a dispatch request through the paired iPhone.
    func relayDispatchThroughPhone(content: String, sessionId: String?) async throws -> WatchDispatchResponse {
        guard WCSession.default.activationState == .activated,
              WCSession.default.isCompanionAppInstalled else {
            throw WatchAuthError.networkError("iPhone companion app not available for relay")
        }
        let nonce = UUID().uuidString.lowercased().replacingOccurrences(of: "-", with: "")
        let payload: [String: Any] = [
            "action":     "dispatch",
            "content":    content,
            "session_id": sessionId ?? NSNull(),
            "nonce":      nonce,
            "device_id":  auth.deviceId,
        ]
        return try await withCheckedThrowingContinuation { continuation in
            WCSession.default.sendMessage(payload, replyHandler: { reply in
                if let data = reply["data"] as? Data,
                   let resp = try? JSONDecoder().decode(WatchDispatchResponse.self, from: data) {
                    continuation.resume(returning: resp)
                } else {
                    continuation.resume(throwing: WatchAuthError.networkError("Invalid relay response"))
                }
            }, errorHandler: { error in
                continuation.resume(throwing: WatchAuthError.networkError(error.localizedDescription))
            })
        }
    }

    // WCSessionDelegate stubs
    nonisolated func session(_ session: WCSession, activationDidCompleteWith activationState: WCSessionActivationState, error: Error?) {}
    nonisolated func session(_ session: WCSession, didReceiveMessage message: [String: Any]) {}
}

// MARK: - Response envelopes

private struct WatchSessionsResponse: Codable {
    let sessions: [WatchSessionSummary]
}

private struct WatchMessagesResponse: Codable {
    let session_id: String
    let messages: [WatchMessage]
    let total: Int
}

// MARK: - HTTP helpers (token-authenticated)

private func getJSON<Resp: Decodable>(url: URL, token: String) async throws -> Resp {
    var req = URLRequest(url: url)
    req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
    let (data, resp) = try await URLSession.shared.data(for: req)
    guard let http = resp as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
        let msg = String(data: data, encoding: .utf8) ?? ""
        throw WatchAuthError.networkError("HTTP \((resp as? HTTPURLResponse)?.statusCode ?? 0): \(msg)")
    }
    return try JSONDecoder().decode(Resp.self, from: data)
}

private func postJSON<Req: Encodable, Resp: Decodable>(url: URL, body: Req, token: String) async throws -> Resp {
    var req = URLRequest(url: url)
    req.httpMethod = "POST"
    req.setValue("application/json", forHTTPHeaderField: "Content-Type")
    req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
    req.httpBody = try JSONEncoder().encode(body)
    let (data, resp) = try await URLSession.shared.data(for: req)
    guard let http = resp as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
        let msg = String(data: data, encoding: .utf8) ?? ""
        throw WatchAuthError.networkError("HTTP \((resp as? HTTPURLResponse)?.statusCode ?? 0): \(msg)")
    }
    return try JSONDecoder().decode(Resp.self, from: data)
}
