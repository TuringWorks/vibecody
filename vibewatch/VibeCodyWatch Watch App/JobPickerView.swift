// JobPickerView.swift — W1.2 Background-job list on the watch.
//
// Counterpart to SessionPickerView. Lists the most recent background-
// agent jobs (queued / running / terminal). Tapping a row opens the
// job recap; long-press exposes the same Recap context menu so the
// gesture matches W1.1's session picker.

import SwiftUI

struct JobPickerView: View {
    @StateObject private var network = WatchNetworkManager.shared

    var body: some View {
        NavigationStack {
            Group {
                if network.isLoadingJobs && network.jobs.isEmpty {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if network.jobs.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "briefcase")
                            .font(.title2)
                            .foregroundStyle(.secondary)
                        Text("No jobs")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                } else {
                    List(network.jobs) { job in
                        NavigationLink(destination: RecapView(
                            sessionId: job.session_id,
                            task_preview: job.task_preview,
                            kind: .job
                        )) {
                            JobRowView(job: job)
                        }
                    }
                    .listStyle(.carousel)
                }
            }
            .navigationTitle("Jobs")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    if network.isLoadingJobs {
                        ProgressView()
                    } else {
                        Button {
                            Task { await network.loadJobs() }
                        } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
            }
        }
        .task { await network.loadJobs() }
    }
}

// MARK: - Job row

struct JobRowView: View {
    let job: WatchJobSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(job.statusIcon)
                    .font(.caption2)
                Text(job.task_preview)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(2)
                Spacer(minLength: 0)
            }
            HStack {
                Text(job.provider)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
                Spacer()
                Text(Date(timeIntervalSince1970: job.started_at), style: .relative)
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }
}
