// SessionPickerView.swift — List of recent sessions on the watch.
// Digital Crown scrolls the list. Tap a session to open ConversationView.

import SwiftUI

struct SessionPickerView: View {
    @StateObject private var network = WatchNetworkManager.shared

    var body: some View {
        NavigationStack {
            Group {
                if network.isLoading && network.sessions.isEmpty {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if network.sessions.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "bubble.left.and.bubble.right")
                            .font(.title2)
                            .foregroundStyle(.secondary)
                        Text("No sessions")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                } else {
                    List(network.sessions) { session in
                        NavigationLink(destination: ConversationView(session: session)) {
                            SessionRowView(session: session)
                        }
                        .contextMenu {
                            // W1.1 — long-press on a session row to view
                            // its recap without opening the conversation.
                            NavigationLink {
                                RecapView(
                                    sessionId: session.session_id,
                                    task_preview: session.task_preview
                                )
                            } label: {
                                Label("Recap", systemImage: "doc.text.magnifyingglass")
                            }
                        }
                    }
                    .listStyle(.carousel)
                }
            }
            .navigationTitle("Sessions")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    if network.isLoading {
                        ProgressView()
                    } else {
                        Button {
                            Task { await network.loadSessions() }
                        } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
            }
        }
        .task { await network.loadSessions() }
    }
}

// MARK: - Session row

struct SessionRowView: View {
    let session: WatchSessionSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(session.statusIcon)
                    .font(.caption2)
                Text(session.task_preview)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(2)
                Spacer(minLength: 0)
            }
            if !session.last_message_preview.isEmpty {
                Text(session.last_message_preview)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            HStack {
                Text(session.provider)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
                Spacer()
                Text(session.lastActivityDate, style: .relative)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }
}
