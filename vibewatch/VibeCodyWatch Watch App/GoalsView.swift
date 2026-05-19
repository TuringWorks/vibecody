// GoalsView.swift — G2.3 Apple Watch goals list + detail.
//
// Read-only counterpart to JobPickerView. Lists active execution
// goals from `/watch/goals` (curated route). Tapping a row opens a
// detail view with the goal's statement and a "Start session" button
// that POSTs to `/v1/goals/:id/start` via the existing watch dispatch
// surface.

import SwiftUI

struct GoalsView: View {
    @StateObject private var network = WatchNetworkManager.shared

    var body: some View {
        NavigationStack {
            Group {
                if network.isLoadingGoals && network.goals.isEmpty {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if network.goals.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "target")
                            .font(.title2)
                            .foregroundStyle(.secondary)
                        Text("No active goals")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                } else {
                    List(network.goals) { goal in
                        NavigationLink(destination: GoalDetailView(summary: goal)) {
                            GoalRowView(goal: goal)
                        }
                    }
                    .listStyle(.carousel)
                }
            }
            .navigationTitle("Goals")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    if network.isLoadingGoals {
                        ProgressView()
                    } else {
                        Button {
                            Task { await network.loadGoals() }
                        } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
            }
        }
        .task { await network.loadGoals() }
    }
}

// MARK: - Row

struct GoalRowView: View {
    let goal: WatchGoalSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(goal.statusIcon)
                    .font(.caption2)
                // G11.2 — show the workspace-pin marker so the user
                // sees which goal new /agent runs will auto-link to.
                if goal.isPinned {
                    Image(systemName: "star.fill")
                        .font(.caption2)
                        .foregroundStyle(.yellow)
                        .accessibilityLabel("current pinned goal")
                }
                Text(goal.title)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(2)
                Spacer(minLength: 0)
            }
            HStack {
                Text(goal.workspace_label)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
                Spacer()
                Text(goal.status)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Detail

struct GoalDetailView: View {
    let summary: WatchGoalSummary
    @StateObject private var network = WatchNetworkManager.shared
    @State private var detail: WatchGoalDetailEnvelope?
    @State private var statement: String = ""
    @State private var loading = true
    @State private var starting = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 4) {
                    // G12.1 — surface the pin marker on the detail
                    // header too so the ★ is consistent with the list
                    // row the user just tapped.
                    if summary.isPinned {
                        Image(systemName: "star.fill")
                            .foregroundStyle(.yellow)
                            .accessibilityLabel("current pinned goal")
                    }
                    Text(summary.title)
                        .font(.headline)
                        .fixedSize(horizontal: false, vertical: true)
                }
                HStack {
                    Text(summary.status)
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.secondary.opacity(0.18))
                        .clipShape(Capsule())
                    Text(summary.workspace_label)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                if loading {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.top, 8)
                } else if !statement.isEmpty {
                    Text(statement)
                        .font(.caption)
                        .foregroundStyle(.primary)
                        .padding(.top, 4)
                }

                Button {
                    Task { await startSession() }
                } label: {
                    HStack {
                        Image(systemName: "play.fill")
                        Text(starting ? "Starting…" : "Start session")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(starting)
                .padding(.top, 8)
            }
            .padding()
        }
        .navigationTitle("Goal")
        .task {
            detail = await network.fetchGoal(id: summary.id)
            if let goalMap = detail?.goal,
               let stmt = goalMap["statement"]?.value as? String {
                statement = stmt
            }
            loading = false
        }
    }

    private func startSession() async {
        starting = true
        defer { starting = false }
        // G4.2 — go through the curated `/watch/goals/:id/start`
        // route so the new session is linked to the goal in the same
        // transaction as it's created (daemon-side `do_v1_exec_goal_start`).
        // Falls back to plain `/watch/dispatch` if the daemon is older
        // and the curated route 404s, so freshly-paired watches stay
        // functional against pre-G4 daemons.
        do {
            _ = try await network.startGoal(id: summary.id)
        } catch {
            _ = try? await network.dispatch(
                content: "Goal: \(summary.title)",
                sessionId: nil,
                provider: nil
            )
        }
    }
}
