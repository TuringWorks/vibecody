// ConversationView.swift — Scrollable conversation + voice input for a session.
//
// Layout:
//   - Scrollable message list (Digital Crown scrolls)
//   - "Dictate" button at bottom → VoiceInputView sheet
//   - Live streaming indicator when agent is responding
//   - Tap message to see full text (overflow sheet)

import SwiftUI

struct ConversationView: View {
    let session: WatchSessionSummary

    @StateObject private var network = WatchNetworkManager.shared
    @State private var messages:   [WatchMessage] = []
    @State private var isLoading = false
    @State private var showVoice = false
    @State private var streamingDelta = ""
    @State private var error: String?

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Message list
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 8) {
                            ForEach(messages) { msg in
                                MessageBubble(message: msg)
                                    .id(msg.id)
                            }
                            // Live streaming delta
                            if !streamingDelta.isEmpty {
                                StreamingBubble(text: streamingDelta)
                                    .id("streaming")
                            }
                            if isLoading {
                                HStack {
                                    ProgressView()
                                        .scaleEffect(0.6)
                                    Text("Thinking…")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                }
                                .id("loading")
                            }
                        }
                        .padding(.horizontal, 4)
                        .padding(.bottom, 8)
                    }
                    .onChange(of: messages.count) { _ in
                        if let last = messages.last {
                            withAnimation { proxy.scrollTo(last.id, anchor: .bottom) }
                        }
                    }
                    .onChange(of: streamingDelta) { _ in
                        withAnimation { proxy.scrollTo("streaming", anchor: .bottom) }
                    }
                }

                Divider()

                // Input bar
                HStack(spacing: 8) {
                    Button {
                        showVoice = true
                    } label: {
                        Image(systemName: "mic.fill")
                            .font(.system(size: 16))
                            .foregroundStyle(network.isStreaming ? Color.secondary : Color.blue)
                    }
                    .disabled(network.isStreaming)
                    .buttonStyle(.plain)

                    if network.isStreaming {
                        HStack(spacing: 4) {
                            ProgressView()
                                .scaleEffect(0.5)
                            Text("Responding…")
                                .font(.system(size: 10))
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
            }
            .navigationTitle(String(session.task_preview.prefix(24)))
            .sheet(isPresented: $showVoice) {
                VoiceInputView(sessionId: session.session_id) { text in
                    Task { await sendMessage(text) }
                }
            }
        }
        .task { await loadMessages() }
    }

    // MARK: - Load messages

    private func loadMessages() async {
        isLoading = true
        defer { isLoading = false }
        do {
            messages = try await network.loadMessages(sessionId: session.session_id)
        } catch {
            self.error = error.localizedDescription
        }
    }

    // MARK: - Send message + stream response

    private func sendMessage(_ text: String) async {
        guard !text.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        // Optimistically add user message
        let optimistic = WatchMessage(
            id: Int(Date().timeIntervalSince1970 * 1000),
            role: "user",
            content: text,
            created_at: Date().timeIntervalSince1970
        )
        messages.append(optimistic)
        streamingDelta = ""
        do {
            let resp = try await network.dispatch(content: text, sessionId: session.session_id)
            // Start SSE stream — non-blocking fire-and-forget; events arrive via callback
            network.startStreaming(sessionId: resp.session_id) { [self] event in
                switch event.kind {
                case "delta":
                    streamingDelta += event.delta ?? ""
                case "done":
                    // Commit accumulated streaming text as an assistant message
                    if !streamingDelta.isEmpty {
                        let msg = WatchMessage(
                            id: Int(Date().timeIntervalSince1970 * 1000) + 1,
                            role: "assistant",
                            content: streamingDelta,
                            created_at: Date().timeIntervalSince1970
                        )
                        messages.append(msg)
                        streamingDelta = ""
                    }
                    // Reload from server to get the persisted message IDs
                    Task { await self.loadMessages() }
                case "error":
                    self.error = event.error
                    streamingDelta = ""
                default: break
                }
            }
        } catch {
            self.error = error.localizedDescription
        }
    }
}

// MARK: - Message bubble

struct MessageBubble: View {
    let message: WatchMessage
    @State private var showFull = false

    var body: some View {
        Button { showFull = true } label: {
            HStack(alignment: .top, spacing: 6) {
                if message.isUser {
                    Spacer(minLength: 20)
                    Text(message.content)
                        .font(.caption)
                        .padding(6)
                        .background(Color.blue.opacity(0.3))
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .multilineTextAlignment(.trailing)
                } else {
                    VStack(alignment: .leading, spacing: 2) {
                        if message.role == "tool" {
                            Label("Tool", systemImage: "wrench")
                                .font(.system(size: 9))
                                .foregroundStyle(.orange)
                        }
                        Text(message.content)
                            .font(.caption)
                            .padding(6)
                            .background(Color.gray.opacity(0.2))
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                    Spacer(minLength: 20)
                }
            }
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $showFull) {
            FullMessageView(message: message)
        }
    }
}

struct StreamingBubble: View {
    let text: String
    var body: some View {
        HStack {
            Text(text)
                .font(.caption)
                .padding(6)
                .background(Color.gray.opacity(0.15))
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.blue.opacity(0.4), lineWidth: 1)
                )
            Spacer(minLength: 20)
        }
    }
}

struct FullMessageView: View {
    let message: WatchMessage
    var body: some View {
        ScrollView {
            Text(message.content)
                .font(.caption)
                .padding()
        }
        .navigationTitle(message.isUser ? "You" : "Assistant")
    }
}
