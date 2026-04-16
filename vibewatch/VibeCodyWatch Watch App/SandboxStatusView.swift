// SandboxStatusView.swift — Live sandbox/container session monitoring.
//
// Shows CPU and RAM gauges using Digital Crown-scrollable cards.
// Allows pause/resume/stop from the wrist.

import SwiftUI

struct SandboxStatusView: View {
    @StateObject private var network = WatchNetworkManager.shared
    @State private var sandboxes: [WatchSandboxStatus] = []
    @State private var isLoading = false
    @State private var error: String?
    @State private var sandboxChatSession: WatchSessionSummary? = nil
    private let timer = Timer.publish(every: 10, on: .main, in: .common).autoconnect()

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 10) {
                    // AI Chat row — shows when VibeUI Sandbox chat is active
                    if let chatSession = sandboxChatSession {
                        NavigationLink(destination: ConversationView(session: chatSession)) {
                            HStack(spacing: 8) {
                                Image(systemName: "bubble.left.and.text.bubble.right")
                                    .font(.system(size: 14))
                                    .foregroundStyle(.blue)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("AI Chat")
                                        .font(.system(size: 12, weight: .medium))
                                    Text(chatSession.task_preview.isEmpty ? "Sandbox conversation" : chatSession.task_preview)
                                        .font(.system(size: 10))
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                                Spacer()
                                Image(systemName: "chevron.right")
                                    .font(.system(size: 10))
                                    .foregroundStyle(.secondary)
                            }
                            .padding(10)
                            .background(Color.blue.opacity(0.12))
                            .clipShape(RoundedRectangle(cornerRadius: 10))
                        }
                        .buttonStyle(.plain)
                    }

                    // Container list
                    if isLoading && sandboxes.isEmpty {
                        ProgressView()
                            .frame(maxWidth: .infinity, minHeight: 60)
                    } else if sandboxes.isEmpty {
                        VStack(spacing: 8) {
                            Image(systemName: "shippingbox")
                                .font(.title2)
                                .foregroundStyle(.secondary)
                            Text("No active sandboxes")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity, minHeight: 60)
                    } else {
                        ForEach(sandboxes) { sandbox in
                            SandboxCard(sandbox: sandbox, onControl: { action in
                                Task { await sendControl(sandboxId: sandbox.container_id, action: action) }
                            })
                        }
                    }
                }
                .padding(.horizontal, 4)
                .padding(.vertical, 6)
            }
            .navigationTitle("Sandbox")
        }
        .task {
            await loadSandboxChatSession()
            await loadSandboxes()
        }
        .onReceive(timer) { _ in Task {
            await loadSandboxChatSession()
            await loadSandboxes()
        }}
    }

    private func loadSandboxChatSession() async {
        guard WatchAuthManager.shared.isPaired,
              let token = try? await WatchAuthManager.shared.validAccessToken() else { return }
        let url = URL(string: "\(WatchAuthManager.shared.endpoint)/watch/sandbox/chat-session")!
        var req = URLRequest(url: url)
        req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        guard let (data, _) = try? await URLSession.shared.data(for: req),
              let json = try? JSONDecoder().decode(SandboxChatSessionResponse.self, from: data),
              let sid = json.session_id else {
            await MainActor.run { sandboxChatSession = nil }
            return
        }
        // Fetch session summary so ConversationView gets the full model
        guard let summary = try? await network.fetchSessionSummary(sessionId: sid) else {
            // Build a minimal stub so navigation still works
            let stub = WatchSessionSummary(
                session_id: sid,
                task_preview: "Sandbox Chat",
                status: "running",
                provider: "",
                model: "",
                message_count: 0,
                step_count: 0,
                started_at: 0,
                last_activity: Date().timeIntervalSince1970,
                last_message_preview: ""
            )
            await MainActor.run { sandboxChatSession = stub }
            return
        }
        await MainActor.run { sandboxChatSession = summary }
    }

    private func loadSandboxes() async {
        guard WatchAuthManager.shared.isPaired else { return }
        isLoading = true
        defer { isLoading = false }
        // Fetch from /watch/sandbox (implemented by watch_bridge.rs)
        guard let token = try? await WatchAuthManager.shared.validAccessToken() else { return }
        let url = URL(string: "\(WatchAuthManager.shared.endpoint)/watch/sandbox")!
        var req = URLRequest(url: url)
        req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        guard let (data, _) = try? await URLSession.shared.data(for: req),
              let resp = try? JSONDecoder().decode(SandboxListResponse.self, from: data) else { return }
        sandboxes = resp.sandboxes
    }

    private func sendControl(sandboxId: String, action: String) async {
        guard let token = try? await WatchAuthManager.shared.validAccessToken() else { return }
        let nonce = UUID().uuidString.lowercased().replacingOccurrences(of: "-", with: "")
        let body = WatchSandboxControlRequest(
            action: action,
            nonce: nonce,
            timestamp: UInt64(Date().timeIntervalSince1970)
        )
        let url = URL(string: "\(WatchAuthManager.shared.endpoint)/watch/sandbox/\(sandboxId)/control")!
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        req.httpBody = try? JSONEncoder().encode(body)
        _ = try? await URLSession.shared.data(for: req)
        await loadSandboxes()
    }
}

// MARK: - Sandbox card

struct SandboxCard: View {
    let sandbox: WatchSandboxStatus
    let onControl: (String) -> Void
    @State private var showOutput = false
    @State private var showActions = false

    var stateColor: Color {
        switch sandbox.state {
        case "running": return .green
        case "paused":  return .yellow
        case "stopped": return .gray
        default:        return .red
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            // Header
            HStack {
                Circle()
                    .fill(stateColor)
                    .frame(width: 8, height: 8)
                Text(sandbox.container_id.prefix(12))
                    .font(.system(size: 11, weight: .medium))
                    .lineLimit(1)
                Spacer()
                Text(formatUptime(sandbox.uptime_secs))
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
            }

            // Resource gauges
            VStack(spacing: 4) {
                GaugeBar(label: "CPU", value: Double(sandbox.cpu_pct), max: 100, color: cpuColor)
                GaugeBar(label: "RAM", value: Double(sandbox.memPct), max: 100, color: .blue)
            }

            // Last output preview
            if let lastLine = sandbox.last_output_lines.last {
                Text(lastLine)
                    .font(.system(size: 9, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .background(Color.black.opacity(0.3))
                    .clipShape(RoundedRectangle(cornerRadius: 4))
            }

            // Action buttons
            if sandbox.isRunning {
                HStack(spacing: 8) {
                    Button {
                        onControl("pause")
                    } label: {
                        Label("Pause", systemImage: "pause.fill")
                            .font(.system(size: 10))
                    }
                    .buttonStyle(.bordered)
                    .tint(.yellow)

                    Button {
                        onControl("stop")
                    } label: {
                        Label("Stop", systemImage: "stop.fill")
                            .font(.system(size: 10))
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                }
            } else if sandbox.state == "paused" {
                Button {
                    onControl("resume")
                } label: {
                    Label("Resume", systemImage: "play.fill")
                        .font(.system(size: 10))
                }
                .buttonStyle(.bordered)
                .tint(.green)
            }
        }
        .padding(8)
        .background(Color.gray.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 10))
        .onTapGesture { showOutput = true }
        .sheet(isPresented: $showOutput) {
            FullOutputView(sandbox: sandbox)
        }
    }

    var cpuColor: Color {
        sandbox.cpu_pct > 80 ? .red : sandbox.cpu_pct > 50 ? .yellow : .green
    }

    func formatUptime(_ secs: UInt64) -> String {
        if secs < 60 { return "\(secs)s" }
        if secs < 3600 { return "\(secs/60)m" }
        return "\(secs/3600)h\((secs%3600)/60)m"
    }
}

// MARK: - Gauge bar

struct GaugeBar: View {
    let label: String
    let value: Double
    let max: Double
    let color: Color

    var body: some View {
        HStack(spacing: 4) {
            Text(label)
                .font(.system(size: 9))
                .foregroundStyle(.secondary)
                .frame(width: 28, alignment: .leading)
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3)
                        .fill(Color.gray.opacity(0.2))
                    RoundedRectangle(cornerRadius: 3)
                        .fill(color)
                        .frame(width: geo.size.width * min(value / max, 1.0))
                }
            }
            .frame(height: 6)
            Text(String(format: "%.0f%%", value))
                .font(.system(size: 9))
                .foregroundStyle(.secondary)
                .frame(width: 28, alignment: .trailing)
        }
    }
}

// MARK: - Full output view

struct FullOutputView: View {
    let sandbox: WatchSandboxStatus
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(sandbox.last_output_lines, id: \.self) { line in
                    Text(line)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.primary)
                }
            }
            .padding()
        }
        .navigationTitle("Output")
    }
}

// MARK: - Response envelopes

private struct SandboxListResponse: Codable {
    let sandboxes: [WatchSandboxStatus]
}

private struct SandboxChatSessionResponse: Codable {
    let session_id: String?
}

// WatchSandboxControlRequest is defined in Models.swift
