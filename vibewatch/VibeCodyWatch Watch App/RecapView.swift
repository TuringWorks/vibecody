// RecapView.swift — W1.1 read-only recap screen on Apple Watch.
//
// Reachable via long-press / context menu on a row in
// SessionPickerView. Mirrors the desktop RecapCard and the Flutter
// RecapCard: headline, generator badge, bullets, next actions,
// artifacts, and a "Continue on phone" button that hands off to the
// paired iPhone via WatchConnectivity (so the user can resume on a
// keyboard surface). Watch never generates recaps.

import SwiftUI
import WatchConnectivity

struct RecapView: View {
    let sessionId: String
    let task_preview: String

    @State private var recap: WatchRecap?
    @State private var isLoading = true
    @State private var loadError: String?

    @StateObject private var network = WatchNetworkManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                if isLoading {
                    ProgressView()
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.top, 24)
                } else if let r = recap {
                    headline(r)
                    if !r.bullets.isEmpty {
                        section(label: "What", items: r.bullets)
                    }
                    if !r.next_actions.isEmpty {
                        section(label: "Next", items: r.next_actions)
                    }
                    if !r.artifacts.isEmpty {
                        artifactSection(r.artifacts)
                    }
                    continueOnPhoneButton(r)
                } else {
                    emptyState
                }
            }
            .padding(.horizontal, 4)
        }
        .navigationTitle("Recap")
        .task { await load() }
    }

    // MARK: - Subviews

    private func headline(_ r: WatchRecap) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(r.headline)
                .font(.headline)
                .lineLimit(3)
            HStack(spacing: 4) {
                Text(r.generator.label)
                    .font(.system(size: 9))
                    .padding(.horizontal, 5)
                    .padding(.vertical, 1)
                    .background(generatorTint(r.generator).opacity(0.25))
                    .clipShape(Capsule())
                Spacer(minLength: 0)
            }
        }
    }

    private func section(label: String, items: [String]) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label.uppercased())
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
            ForEach(Array(items.enumerated()), id: \.offset) { _, line in
                HStack(alignment: .top, spacing: 4) {
                    Text("•").font(.caption2).foregroundStyle(.secondary)
                    Text(line).font(.caption2)
                }
            }
        }
    }

    private func artifactSection(_ artifacts: [WatchRecapArtifact]) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("ARTIFACTS")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
            ForEach(Array(artifacts.enumerated()), id: \.offset) { _, a in
                HStack(spacing: 4) {
                    Image(systemName: artifactIcon(a.kind))
                        .font(.system(size: 9))
                        .foregroundStyle(.secondary)
                    Text(a.label).font(.caption2)
                    Text(a.locator)
                        .font(.system(size: 9))
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }
        }
    }

    private func continueOnPhoneButton(_ r: WatchRecap) -> some View {
        Button {
            handoffToPhone(recap: r)
        } label: {
            Label("Continue on phone", systemImage: "iphone.and.arrow.forward")
                .font(.caption)
        }
        .buttonStyle(.borderedProminent)
        .tint(.blue)
        .padding(.top, 4)
    }

    private var emptyState: some View {
        VStack(spacing: 6) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.title3)
                .foregroundStyle(.secondary)
            Text("No recap yet")
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(task_preview)
                .font(.system(size: 10))
                .foregroundStyle(.tertiary)
                .lineLimit(2)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 24)
    }

    // MARK: - Helpers

    private func load() async {
        isLoading = true
        defer { isLoading = false }
        recap = await network.loadRecap(sessionId: sessionId)
    }

    private func generatorTint(_ g: WatchRecapGenerator) -> Color {
        switch g.type {
        case "llm": return .blue
        case "user_edited": return .orange
        default: return .gray
        }
    }

    private func artifactIcon(_ kind: String) -> String {
        switch kind {
        case "file": return "doc"
        case "diff": return "arrow.triangle.2.circlepath"
        case "job":  return "briefcase"
        case "url":  return "link"
        default:     return "circle"
        }
    }

    /// Hand off to paired iPhone via WatchConnectivity so the user can
    /// resume on a keyboard surface. The phone listens on
    /// WatchConnectivityBridge in VibeCodyWatchCompanion.
    private func handoffToPhone(recap r: WatchRecap) {
        guard WCSession.isSupported() else { return }
        let session = WCSession.default
        guard session.activationState == .activated else { return }
        let payload: [String: Any] = [
            "action": "open_session",
            "session_id": r.subject_id,
            "recap_id": r.id,
            "headline": r.headline,
            "seed": r.next_actions.first ?? "",
        ]
        if session.isReachable {
            session.sendMessage(payload, replyHandler: nil, errorHandler: nil)
        } else {
            // Phone offline — drop into transferUserInfo so it lands on next launch.
            session.transferUserInfo(payload)
        }
    }
}

#if DEBUG
#Preview("Heuristic") {
    NavigationStack {
        RecapView_PreviewHost(
            recap: WatchRecap(
                id: "rcp_abc",
                kind: "session",
                subject_id: "sess_xyz",
                headline: "Wired auth refresh-token rotation",
                bullets: ["Ran cargo test (3x)", "Edited src/auth.rs"],
                next_actions: ["Wire refresh token to frontend"],
                artifacts: [WatchRecapArtifact(kind: "file", label: "auth.rs", locator: "src/auth.rs")],
                generator: WatchRecapGenerator(type: "heuristic", provider: nil, model: nil),
                schema_version: 1
            )
        )
    }
}

#Preview("LLM") {
    NavigationStack {
        RecapView_PreviewHost(
            recap: WatchRecap(
                id: "rcp_abc",
                kind: "session",
                subject_id: "sess_xyz",
                headline: "Investigated flaky migration test",
                bullets: ["8 tools used", "12 messages exchanged"],
                next_actions: ["Add idempotency check"],
                artifacts: [],
                generator: WatchRecapGenerator(
                    type: "llm",
                    provider: "anthropic",
                    model: "claude-opus-4-7"
                ),
                schema_version: 1
            )
        )
    }
}

/// Test-only host that injects a `WatchRecap` into the view, bypassing
/// the network call so previews and snapshot tests are deterministic.
struct RecapView_PreviewHost: View {
    let recap: WatchRecap
    var body: some View {
        // We can't easily mock WatchNetworkManager.shared, so render the
        // recap body directly using the same layout primitives. Keeping
        // this as a thin alternate composition is simpler than wiring a
        // dependency injection seam through the production view.
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text(recap.headline).font(.headline).lineLimit(3)
                Text(recap.generator.label).font(.system(size: 9))
                ForEach(recap.bullets, id: \.self) { b in
                    Text("• \(b)").font(.caption2)
                }
            }
            .padding(.horizontal, 4)
        }
    }
}
#endif
