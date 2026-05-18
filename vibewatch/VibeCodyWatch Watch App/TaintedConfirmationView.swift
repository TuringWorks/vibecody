// TaintedConfirmationView.swift — DREAD #1 Slice G part 3 (watch).
//
// Glanceable confirmation surface for a tainted-argument prompt
// pushed by the daemon. Consumes `GET /watch/tainted/pending` (SSE,
// Watch-Token auth) and posts `POST /watch/tainted/respond` on
// approve / deny. Same `HttpPromptQueue` as the desktop modal and
// the mobile sheet — whichever client decides first wins.
//
// Threat-model invariants:
//
// * Payload bytes never leave the daemon. The view renders only
//   `audit_summary` (kind, provenance fields, audit_id) — same
//   contract as the CLI prompter banner and the mobile sheet.
// * Deny-by-default: closing the view without tapping Approve does
//   NOT send anything; the daemon will time out (5 min) and deny.
//   Only an explicit Approve tap fires `approve=true`.
// * Small-screen tradeoff: the watch shows `sink` + the first ~100
//   chars of `summary` on the glance; the full summary scrolls.

import SwiftUI

/// One pending prompt as published by the daemon's SSE stream.
struct TaintedPromptEvent: Decodable, Equatable, Identifiable {
    let request_id: String
    let audit_id: String
    let summary: String
    let sink: String
    let issued_at: UInt64

    var id: String { request_id }

    /// Human-friendly sink label for the header. Mirrors the mapping
    /// in the Flutter `TaintedPrompt.sinkLabel` getter.
    var sinkLabel: String {
        switch sink {
        case "ToolCallArgument": return "Run tool"
        case "McpArgument":      return "Call MCP tool"
        case "RagDocument":      return "Use document"
        case "WebFetch":         return "Fetch URL"
        case "LlmRequestBody":   return "Send to LLM"
        case "LogLine":          return "Emit log"
        case "ShellCommand":     return "Run shell"
        default:                 return "Confirm action"
        }
    }
}

/// POST body for the resolve endpoint.
private struct TaintedRespondRequest: Encodable {
    let request_id: String
    let approve: Bool
}

/// SwiftUI overlay that watches the daemon's tainted-prompt SSE and
/// presents a glanceable approve/deny prompt for the head-of-queue
/// entry. Embed in `ContentView`'s root `TabView` or a
/// `.overlay { TaintedConfirmationOverlay() }` so it's always live
/// while paired.
struct TaintedConfirmationOverlay: View {
    @StateObject private var queue = TaintedConfirmationQueue()

    var body: some View {
        Group {
            if let head = queue.head {
                TaintedConfirmationView(
                    prompt: head,
                    queuedBehind: max(0, queue.pending.count - 1),
                    onApprove: { Task { await queue.respond(head, approve: true) } },
                    onDeny:    { Task { await queue.respond(head, approve: false) } }
                )
                // The watch can't dismiss the prompt without a
                // decision — that's the design. There is no swipe-down.
                .transition(.opacity)
            }
        }
        .task {
            await queue.start()
        }
        .onDisappear { queue.stop() }
    }
}

/// The actual glanceable card.
struct TaintedConfirmationView: View {
    let prompt: TaintedPromptEvent
    let queuedBehind: Int
    let onApprove: () -> Void
    let onDeny: () -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 4) {
                    Image(systemName: "shield.lefthalf.filled")
                        .foregroundStyle(.red)
                    Text(prompt.sinkLabel)
                        .font(.caption.bold())
                }

                Text("Untrusted data — review before approving.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)

                Text(prompt.summary)
                    .font(.system(size: 10, design: .monospaced))
                    .lineLimit(8)
                    .padding(6)
                    .background(.gray.opacity(0.2))
                    .cornerRadius(4)

                HStack(spacing: 2) {
                    Text(prompt.audit_id.prefix(8) + "…")
                        .font(.system(size: 9))
                        .foregroundStyle(.secondary)
                    if queuedBehind > 0 {
                        Spacer()
                        Text("+\(queuedBehind) more")
                            .font(.system(size: 9))
                            .foregroundStyle(.secondary)
                    }
                }

                HStack(spacing: 6) {
                    Button(action: onDeny) {
                        Text("Deny")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .tint(.gray)

                    Button(action: onApprove) {
                        Text("Approve")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                }
                .padding(.top, 4)
            }
            .padding(8)
        }
        .background(Color.black.opacity(0.85))
    }
}

/// Owns the SSE subscription and the FIFO queue of pending prompts.
/// Lives as long as the overlay is on-screen.
@MainActor
final class TaintedConfirmationQueue: ObservableObject {
    @Published private(set) var pending: [TaintedPromptEvent] = []

    /// Head-of-queue prompt rendered by the overlay.
    var head: TaintedPromptEvent? { pending.first }

    private var streamTask: Task<Void, Never>?
    private var seen = Set<String>()
    private var resolved = Set<String>()
    private let auth = WatchAuthManager.shared

    /// Subscribe to /watch/tainted/pending and forward each `pending`
    /// SSE event into the FIFO queue.
    func start() async {
        stop()
        streamTask = Task { [weak self] in
            await self?.runStream()
        }
    }

    func stop() {
        streamTask?.cancel()
        streamTask = nil
    }

    private func runStream() async {
        var backoff: UInt64 = 1_000_000_000 // 1 second
        let maxBackoff: UInt64 = 30_000_000_000 // 30 seconds

        while !Task.isCancelled {
            guard auth.isPaired,
                  let token = try? await auth.validAccessToken() else {
                try? await Task.sleep(nanoseconds: backoff)
                backoff = min(backoff * 2, maxBackoff)
                continue
            }
            let url = URL(string: "\(auth.endpoint)/watch/tainted/pending")
            guard let url else { return }

            var request = URLRequest(url: url)
            request.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
            request.setValue("text/event-stream", forHTTPHeaderField: "Accept")
            // Long timeout — SSE keep-alive.
            request.timeoutInterval = 300

            do {
                let (asyncBytes, response) = try await URLSession.shared.bytes(for: request)
                guard let http = response as? HTTPURLResponse,
                      (200...299).contains(http.statusCode) else {
                    try? await Task.sleep(nanoseconds: backoff)
                    backoff = min(backoff * 2, maxBackoff)
                    continue
                }
                backoff = 1_000_000_000 // healthy connection — reset

                var currentEvent = ""
                var dataBuffer = ""
                for try await line in asyncBytes.lines {
                    if Task.isCancelled { return }
                    if line.isEmpty {
                        if currentEvent == "pending", !dataBuffer.isEmpty {
                            await handleEventPayload(dataBuffer)
                        }
                        currentEvent = ""
                        dataBuffer = ""
                        continue
                    }
                    if line.hasPrefix("event: ") {
                        currentEvent = String(line.dropFirst(7))
                            .trimmingCharacters(in: .whitespaces)
                    } else if line.hasPrefix("data: ") {
                        if !dataBuffer.isEmpty { dataBuffer += "\n" }
                        dataBuffer += String(line.dropFirst(6))
                    }
                }
            } catch {
                // Cancellation / network error — back off and retry.
                try? await Task.sleep(nanoseconds: backoff)
                backoff = min(backoff * 2, maxBackoff)
            }
        }
    }

    private func handleEventPayload(_ json: String) async {
        guard let data = json.data(using: .utf8),
              let event = try? JSONDecoder().decode(TaintedPromptEvent.self, from: data)
        else { return }
        if resolved.contains(event.request_id) { return }
        if seen.contains(event.request_id) { return }
        seen.insert(event.request_id)
        pending.append(event)
    }

    /// POST the user's decision. Optimistically pops the head;
    /// failures don't re-queue (daemon-side timeout will deny on its
    /// own).
    func respond(_ prompt: TaintedPromptEvent, approve: Bool) async {
        resolved.insert(prompt.request_id)
        pending.removeAll { $0.request_id == prompt.request_id }

        guard auth.isPaired,
              let token = try? await auth.validAccessToken(),
              let url = URL(string: "\(auth.endpoint)/watch/tainted/respond")
        else { return }
        let body = TaintedRespondRequest(
            request_id: prompt.request_id,
            approve: approve
        )
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        request.httpBody = try? JSONEncoder().encode(body)

        // Fire-and-forget — server returns 200 / 404, neither of
        // which we surface on the watch's tiny screen. Daemon
        // timeout is the safety net.
        _ = try? await URLSession.shared.data(for: request)
    }
}
