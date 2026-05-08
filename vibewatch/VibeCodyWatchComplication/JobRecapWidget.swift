// JobRecapWidget.swift — W1.2 watchOS Smart Stack tile + complication.
//
// Surfaces the freshest terminal-state job recap as a glanceable
// widget. Reads the same `WatchAuthManager` shared keychain entries
// the main watch app uses, so the user has nothing else to set up.
//
// Patent / privacy posture: read-only. The widget never generates a
// recap; it just displays what the daemon's J1.2 hook has already
// stored. Falls back to the iPhone-relay path when the watch has no
// direct LAN/Tailscale reach (matches the main app's transport
// resolution).

import WidgetKit
import SwiftUI

// MARK: - Wire shapes (kept in sync with WatchRecap in the main app)

private struct ComplicationRecap: Codable {
    let id: String
    let kind: String
    let subject_id: String
    let headline: String
    let bullets: [String]
}

private struct ComplicationEnvelope: Codable {
    let recap: ComplicationRecap?
}

private struct JobSummary: Codable, Identifiable {
    var id: String { session_id }
    let session_id: String
    let task_preview: String
    let status: String
}

private struct JobsResponse: Codable {
    let jobs: [JobSummary]
}

// MARK: - Timeline entry

struct JobRecapEntry: TimelineEntry {
    let date: Date
    let headline: String
    let status: String
    /// `nil` when the daemon is unreachable or no terminal job has
    /// been recapped yet — the widget renders a "no recap" state.
    let recapId: String?
    let provider: String
}

extension JobRecapEntry {
    static let placeholder = JobRecapEntry(
        date: Date(),
        headline: "No recap yet",
        status: "idle",
        recapId: nil,
        provider: ""
    )
}

// MARK: - Provider

struct JobRecapProvider: TimelineProvider {
    func placeholder(in context: Context) -> JobRecapEntry { .placeholder }

    func getSnapshot(in context: Context, completion: @escaping (JobRecapEntry) -> Void) {
        Task {
            completion(await fetchEntry() ?? .placeholder)
        }
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<JobRecapEntry>) -> Void) {
        Task {
            let entry = await fetchEntry() ?? .placeholder
            // Refresh every 15 minutes — recap state is low-frequency
            // and the watch's energy budget for widget refresh is small.
            let next = Calendar.current.date(byAdding: .minute, value: 15, to: Date()) ?? Date()
            completion(Timeline(entries: [entry], policy: .after(next)))
        }
    }

    /// Best-effort fetch. We resolve the daemon endpoint + token from
    /// the same WatchAuthManager-shared keychain the main app writes;
    /// any failure (no pairing, daemon offline, no terminal jobs)
    /// returns nil so the placeholder renders.
    private func fetchEntry() async -> JobRecapEntry? {
        guard let endpoint = SharedAuth.endpoint(),
              let token = await SharedAuth.validToken() else {
            return nil
        }
        // 1) Find the latest terminal job.
        guard let jobsURL = URL(string: "\(endpoint)/watch/jobs") else { return nil }
        var req = URLRequest(url: jobsURL)
        req.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        guard let (data, resp) = try? await URLSession.shared.data(for: req),
              let http = resp as? HTTPURLResponse, http.statusCode == 200,
              let parsed = try? JSONDecoder().decode(JobsResponse.self, from: data) else {
            return nil
        }
        let terminal = parsed.jobs.first { ["complete", "failed", "cancelled"].contains($0.status) }
        guard let job = terminal else { return nil }

        // 2) Pull the recap for that job.
        guard let recapURL = URL(string: "\(endpoint)/watch/jobs/\(job.session_id)/recap") else {
            return JobRecapEntry(
                date: Date(),
                headline: job.task_preview,
                status: job.status,
                recapId: nil,
                provider: ""
            )
        }
        var req2 = URLRequest(url: recapURL)
        req2.setValue("Watch-Token \(token)", forHTTPHeaderField: "Authorization")
        if let (rdata, rresp) = try? await URLSession.shared.data(for: req2),
           let rhttp = rresp as? HTTPURLResponse, rhttp.statusCode == 200,
           let env = try? JSONDecoder().decode(ComplicationEnvelope.self, from: rdata),
           let recap = env.recap {
            return JobRecapEntry(
                date: Date(),
                headline: recap.headline,
                status: job.status,
                recapId: recap.id,
                provider: ""
            )
        }
        // Recap missing — show the bare task as a fallback.
        return JobRecapEntry(
            date: Date(),
            headline: job.task_preview,
            status: job.status,
            recapId: nil,
            provider: ""
        )
    }
}

// MARK: - View

struct JobRecapView: View {
    let entry: JobRecapEntry
    @Environment(\.widgetFamily) private var family

    var body: some View {
        switch family {
        case .accessoryCircular: circular
        case .accessoryCorner:   corner
        case .accessoryInline:   inline
        case .accessoryRectangular: rectangular
        default: rectangular
        }
    }

    private var statusIcon: String {
        switch entry.status {
        case "complete":  return "checkmark.circle.fill"
        case "failed":    return "xmark.circle.fill"
        case "cancelled": return "xmark.octagon.fill"
        case "running":   return "play.circle.fill"
        default:          return "circle"
        }
    }

    private var rectangular: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 4) {
                Image(systemName: statusIcon)
                    .font(.caption2)
                Text("Last job")
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundStyle(.secondary)
            }
            Text(entry.headline)
                .font(.caption2)
                .lineLimit(2)
        }
    }

    private var circular: some View {
        Image(systemName: statusIcon)
            .font(.title3)
    }

    private var corner: some View {
        Text(entry.headline)
            .font(.system(size: 9))
            .lineLimit(1)
            .widgetCurvesContent()
    }

    private var inline: some View {
        Text("Job · \(entry.headline)")
            .lineLimit(1)
    }
}

// MARK: - Widget

@main
struct VibeCodyWatchComplicationBundle: WidgetBundle {
    var body: some Widget {
        JobRecapWidget()
    }
}

struct JobRecapWidget: Widget {
    let kind: String = "VibeCodyJobRecap"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: JobRecapProvider()) { entry in
            JobRecapView(entry: entry)
                .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Job Recap")
        .description("The latest VibeCody background-job recap.")
        .supportedFamilies([
            .accessoryCircular,
            .accessoryCorner,
            .accessoryInline,
            .accessoryRectangular,
        ])
    }
}

private extension View {
    /// Older watchOS versions don't expose `widgetCurvesContent`;
    /// we degrade gracefully by returning self so the widget still
    /// renders even if the modifier is unavailable.
    @ViewBuilder
    func widgetCurvesContent() -> some View {
        if #available(watchOS 10.0, *) {
            self.widgetCurvesContentInternal()
        } else {
            self
        }
    }

    @available(watchOS 10.0, *)
    @ViewBuilder
    func widgetCurvesContentInternal() -> some View {
        // `widgetLabel` is the supported curved-text affordance on
        // watchOS 10's `accessoryCorner` family; we keep this thin
        // so the widget compiles even if Apple renames the API.
        self
    }
}
