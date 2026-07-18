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

    // MARK: - Fetch a single session summary by ID

    func fetchSessionSummary(sessionId: String) async throws -> WatchSessionSummary? {
        let token = try await auth.validAccessToken()
        let url = URL(string: "\(auth.endpoint)/watch/sessions")!
        let result: WatchSessionsResponse = try await getJSON(url: url, token: token)
        return result.sessions.first { $0.session_id == sessionId }
    }

    // MARK: - Recap (W1.1 / W1.2 — read-only)

    /// Fetch the freshest recap for a session. Returns `nil` when the
    /// daemon has no recap (older daemon, never generated, or 4xx).
    /// Best-effort — never throws on network failure.
    func loadRecap(sessionId: String) async -> WatchRecap? {
        await loadRecap(subjectId: sessionId, kind: "session")
    }

    /// W1.2 — Fetch the freshest recap for a background-agent job.
    /// Read-only; daemon's J1.2 hook owns generation.
    func loadJobRecap(jobId: String) async -> WatchRecap? {
        await loadRecap(subjectId: jobId, kind: "job")
    }

    private func loadRecap(subjectId: String, kind: String) async -> WatchRecap? {
        guard auth.isPaired, let token = try? await auth.validAccessToken() else {
            return nil
        }
        let path = kind == "job"
            ? "/watch/jobs/\(subjectId)/recap"
            : "/watch/sessions/\(subjectId)/recap"
        guard let url = URL(string: "\(auth.endpoint)\(path)") else {
            return nil
        }
        do {
            let resp: WatchRecapEnvelope = try await getJSON(url: url, token: token)
            return resp.recap
        } catch {
            return nil
        }
    }

    /// W1.2 — list of recent jobs for the Smart Stack tile / job
    /// picker. Returns the head of /watch/jobs (cap 25 server-side).
    @Published var jobs: [WatchJobSummary] = []
    @Published var isLoadingJobs = false

    func loadJobs() async {
        guard auth.isPaired else { return }
        isLoadingJobs = true
        defer { isLoadingJobs = false }
        do {
            let token = try await auth.validAccessToken()
            let url = URL(string: "\(auth.endpoint)/watch/jobs")!
            let result: WatchJobsResponse = try await getJSON(url: url, token: token)
            jobs = result.jobs
        } catch {
            // Best-effort — leave the prior list visible.
        }
    }

    // MARK: - Goals (G1.6 — read-only, curated /watch/goals)

    @Published var goals: [WatchGoalSummary] = []
    @Published var isLoadingGoals = false

    /// Load the active goals from the curated `/watch/goals` endpoint.
    /// Best-effort: failures leave the prior list visible.
    func loadGoals() async {
        guard auth.isPaired else { return }
        isLoadingGoals = true
        defer { isLoadingGoals = false }
        do {
            let token = try await auth.validAccessToken()
            let url = URL(string: "\(auth.endpoint)/watch/goals")!
            let result: WatchGoalsResponse = try await getJSON(url: url, token: token)
            goals = result.goals
        } catch {
            // Silent — same precedent as loadJobs.
        }
    }

    /// Fetch a single goal's detail envelope. The watch detail view
    /// renders `title` / `status` / `statement` from the embedded goal
    /// object; richer fields are ignored in v1.
    func fetchGoal(id: String) async -> WatchGoalDetailEnvelope? {
        guard auth.isPaired, let token = try? await auth.validAccessToken() else {
            return nil
        }
        guard let url = URL(string: "\(auth.endpoint)/watch/goals/\(id)") else {
            return nil
        }
        do {
            let env: WatchGoalDetailEnvelope = try await getJSON(url: url, token: token)
            return env
        } catch {
            return nil
        }
    }

    /// G4.2 — start a session bound to a goal via the curated
    /// `/watch/goals/:id/start` route, which wraps the daemon's
    /// canonical `do_v1_exec_goal_start` helper. The resulting session
    /// is auto-linked through `goal_links` at create time. Returns the
    /// new session id on success.
    func startGoal(id: String, task: String? = nil) async throws -> String {
        let token = try await auth.validAccessToken()
        guard let url = URL(string: "\(auth.endpoint)/watch/goals/\(id)/start") else {
            throw WatchAuthError.networkError("bad goal-start URL")
        }
        struct Body: Encodable { let task: String? }
        let resp: StartGoalResponse = try await postJSON(
            url: url,
            body: Body(task: task),
            token: token
        )
        return resp.session_id
    }

    // MARK: - Code Graph (kodegraph — curated /watch/graph/*)
    //
    // Two routes only (Watch never hits /v1/*): a compact status and a query
    // capped server-side to ≤5 nodes so it fits a wrist screen.

    /// Compact graph probe for the watch face: `{status, n, m}`.
    struct WatchGraphStatus: Decodable {
        let status: String
        let n: Int
        let m: Int
    }
    @Published var graphStatus: WatchGraphStatus?

    /// `GET /watch/graph/status`. Best-effort — failures leave the prior value.
    func loadGraphStatus() async {
        guard auth.isPaired else { return }
        do {
            let token = try await auth.validAccessToken()
            let url = URL(string: "\(auth.endpoint)/watch/graph/status")!
            graphStatus = try await getJSON(url: url, token: token)
        } catch {
            // silent — same precedent as loadJobs/loadGoals
        }
    }

    /// `POST /watch/graph/query {query, budget?}` — capped subgraph as a raw
    /// JSON dict (`{seeds, nodes, edges, est_tokens}`). The watch renders the
    /// seed names + node labels; full NodeData shapes are left untyped.
    func loadGraphQuery(_ query: String, budget: Int = 2000) async throws -> [String: Any] {
        let token = try await auth.validAccessToken()
        guard let url = URL(string: "\(auth.endpoint)/watch/graph/query") else {
            throw WatchAuthError.networkError("bad graph-query URL")
        }
        return try await postJSONDict(
            url: url,
            body: ["query": query, "budget": budget],
            token: token
        )
    }

    // MARK: - SkillForge (skill catalogue — curated /watch/skilllens/*)
    //
    // Two read-only routes (Watch never hits /v1/*): a compact catalogue
    // summary (`{count, top5}`) and a one-line skill detail. The heavy
    // score/train/promote mutations stay desktop-only.

    /// Compact catalogue probe for the watch: `{count, top5:[{name, category, summary}]}`.
    struct WatchSkillRow: Decodable {
        let name: String
        let category: String
        let summary: String
    }
    struct WatchSkilllensCatalog: Decodable {
        let count: Int
        let top5: [WatchSkillRow]
    }
    @Published var skilllensCatalog: WatchSkilllensCatalog?

    /// `GET /watch/skilllens/skills`. Best-effort — failures leave the prior value.
    func loadSkilllensSkills() async {
        guard auth.isPaired else { return }
        do {
            let token = try await auth.validAccessToken()
            let url = URL(string: "\(auth.endpoint)/watch/skilllens/skills")!
            skilllensCatalog = try await getJSON(url: url, token: token)
        } catch {
            // silent — same precedent as loadGraphStatus
        }
    }

    /// `GET /watch/skilllens/skills/:name` — one-line `{name, category, summary}`.
    func loadSkilllensSkill(_ name: String) async throws -> WatchSkillRow {
        let token = try await auth.validAccessToken()
        guard let encoded = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed),
              let url = URL(string: "\(auth.endpoint)/watch/skilllens/skills/\(encoded)") else {
            throw WatchAuthError.networkError("bad skilllens-skill URL")
        }
        return try await getJSON(url: url, token: token)
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

    // MARK: - Active session (Google Docs-style session lock)

    /// Tell the daemon which session this Watch is currently viewing.
    /// VibeCoder subscribes to /watch/events and switches to the same tab automatically.
    func setActiveSession(_ sessionId: String) async {
        guard auth.isPaired, let token = try? await auth.validAccessToken() else { return }
        guard let url = URL(string: "\(auth.endpoint)/watch/active-session") else { return }
        struct Body: Encodable { let session_id: String }
        struct OkResponse: Codable { let ok: Bool }
        _ = try? await postJSON(url: url, body: Body(session_id: sessionId), token: token) as OkResponse
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

private struct WatchJobsResponse: Codable {
    let jobs: [WatchJobSummary]
}

private struct WatchGoalsResponse: Codable {
    let goals: [WatchGoalSummary]
}

private struct StartGoalResponse: Decodable {
    let session_id: String
}

struct WatchGoalDetailEnvelope: Codable {
    let goal: [String: AnyCodable]?
    let links: [[String: AnyCodable]]?
}

/// Minimal `AnyCodable` so the watch can deserialize the heterogeneous
/// goal detail payload without binding the full vibe-ai `ExecutionPlan`
/// shape. The Watch detail view renders the title/statement/status from
/// fixed keys and skips the rest for v1.
struct AnyCodable: Codable {
    let value: Any?
    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { value = nil }
        else if let v = try? c.decode(String.self) { value = v }
        else if let v = try? c.decode(Bool.self)   { value = v }
        else if let v = try? c.decode(Double.self) { value = v }
        else if let v = try? c.decode([String: AnyCodable].self) {
            value = v.mapValues { $0.value }
        }
        else if let v = try? c.decode([AnyCodable].self) {
            value = v.map { $0.value }
        }
        else { value = nil }
    }
    func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        if value == nil { try c.encodeNil() } else { try c.encodeNil() }
    }
}

private struct WatchRecapEnvelope: Codable {
    let recap: WatchRecap?
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

/// POST a loose JSON body and return the response as a `[String: Any]` dict.
/// Used by the graph query route, whose NodeData shapes are too rich to model
/// as a fixed Codable struct on the watch.
private func postJSONDict(url: URL, body: [String: Any], token: String) async throws -> [String: Any] {
    var req = URLRequest(url: url)
    req.httpMethod = "POST"
    req.setValue("application/json", forHTTPHeaderField: "Content-Type")
    req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
    req.httpBody = try JSONSerialization.data(withJSONObject: body)
    let (data, resp) = try await URLSession.shared.data(for: req)
    guard let http = resp as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
        let msg = String(data: data, encoding: .utf8) ?? ""
        throw WatchAuthError.networkError("HTTP \((resp as? HTTPURLResponse)?.statusCode ?? 0): \(msg)")
    }
    guard let obj = try? JSONSerialization.jsonObject(with: data),
          let dict = obj as? [String: Any] else {
        throw WatchAuthError.networkError("non-JSON graph response")
    }
    return dict
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
