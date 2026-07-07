// SkillforgeView.swift — G4 Apple Watch SkillForge catalogue + detail.
//
// Read-only counterpart to GoalsView. Renders the curated
// `/watch/skilllens/skills` catalogue (`{count, top5}`) that
// WatchNetworkManager already fetches (Phase 5). Tapping a row opens
// a one-line detail (`/watch/skilllens/skills/:name` — name, category,
// summary). The heavy score/train/promote mutations stay desktop-only
// (STRICT — the watch surfaces no toolbar LLM).

import SwiftUI

struct SkillforgeView: View {
    @StateObject private var network = WatchNetworkManager.shared

    var body: some View {
        NavigationStack {
            Group {
                let catalog = network.skilllensCatalog
                if catalog == nil {
                    // Not loaded yet — show a placeholder until the
                    // first /watch/skilllens/skills round-trip lands.
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let cat = catalog, cat.top5.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "graduationcap")
                            .font(.title2)
                            .foregroundStyle(.secondary)
                        Text("No skills surfaced")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                } else if let cat = catalog {
                    List(cat.top5, id: \.name) { skill in
                        NavigationLink(destination: SkillforgeDetailView(summary: skill)) {
                            SkillforgeRowView(skill: skill)
                        }
                    }
                    .listStyle(.carousel)
                }
            }
            .navigationTitle("Skills")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        Task { await network.loadSkilllensSkills() }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }
        }
        .task { await network.loadSkilllensSkills() }
    }
}

// MARK: - Row

struct SkillforgeRowView: View {
    let skill: WatchNetworkManager.WatchSkillRow

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(skill.name)
                .font(.caption)
                .fontWeight(.medium)
                .lineLimit(2)
            HStack {
                if !skill.category.isEmpty {
                    Text(skill.category)
                        .font(.system(size: 9))
                        .foregroundStyle(.tertiary)
                }
                Spacer()
                Text("skilllens")
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Detail

struct SkillforgeDetailView: View {
    let summary: WatchNetworkManager.WatchSkillRow
    @StateObject private var network = WatchNetworkManager.shared
    @State private var detail: WatchNetworkManager.WatchSkillRow?
    @State private var loading = true
    @State private var loadError: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text(summary.name)
                    .font(.headline)
                    .fixedSize(horizontal: false, vertical: true)
                if !summary.category.isEmpty {
                    Text(summary.category)
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.secondary.opacity(0.18))
                        .clipShape(Capsule())
                }
                if loading {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.top, 8)
                } else if let err = loadError {
                    Text(err)
                        .font(.caption2)
                        .foregroundStyle(.red)
                        .padding(.top, 4)
                } else if let d = detail {
                    Text(d.summary.isEmpty ? "—" : d.summary)
                        .font(.caption)
                        .foregroundStyle(.primary)
                        .padding(.top, 4)
                }
                Text("Score / train / promote stay on desktop.")
                    .font(.system(size: 9))
                    .foregroundStyle(.secondary)
                    .padding(.top, 8)
            }
            .padding()
        }
        .navigationTitle("Skill")
        .task {
            do {
                detail = try await network.loadSkilllensSkill(summary.name)
            } catch {
                loadError = error.localizedDescription
            }
            loading = false
        }
    }
}