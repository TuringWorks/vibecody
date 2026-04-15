// ContentView.swift — Root navigation for VibeCody Watch App
//
// Navigation tree:
//   ContentView
//   ├── SessionPickerView    (when paired)
//   │   └── ConversationView (per session)
//   │       └── VoiceInputView
//   ├── SandboxStatusView
//   └── PairingView          (when not paired)

import SwiftUI

struct ContentView: View {
    @StateObject private var auth    = WatchAuthManager.shared
    @StateObject private var network = WatchNetworkManager.shared
    @State private var selectedTab   = 0

    var body: some View {
        if auth.isPaired {
            TabView(selection: $selectedTab) {
                SessionPickerView()
                    .tag(0)
                    .tabItem { Label("Sessions", systemImage: "bubble.left.and.bubble.right") }

                SandboxStatusView()
                    .tag(1)
                    .tabItem { Label("Sandbox", systemImage: "shippingbox") }

                SettingsView()
                    .tag(2)
                    .tabItem { Label("Settings", systemImage: "gear") }
            }
        } else {
            PairingView()
        }
    }
}

// MARK: - Pairing view

struct PairingView: View {
    @StateObject private var auth = WatchAuthManager.shared
    @State private var errorMsg: String?
    @State private var showManual = false
    @State private var daemonURL  = "http://localhost:7878"
    @State private var isPairing  = false

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                Image(systemName: "applewatch")
                    .font(.system(size: 32))
                    .foregroundStyle(.blue)

                Text("VibeCody")
                    .font(.headline)

                Text("Open VibeUI → Governance → Watch Devices and tap **Pair**.")
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)

                // Dev / simulator shortcut
                Button {
                    showManual = true
                } label: {
                    Label("Connect manually", systemImage: "link")
                        .font(.caption2)
                }
                .buttonStyle(.bordered)
                .tint(.blue)

                if let err = errorMsg {
                    Text(err)
                        .font(.caption2)
                        .foregroundStyle(.red)
                        .lineLimit(4)
                }
            }
            .padding()
        }
        .navigationTitle("Pair Watch")
        .sheet(isPresented: $showManual) {
            ManualPairingView(daemonURL: $daemonURL, isPairing: $isPairing, errorMsg: $errorMsg)
        }
    }
}

// MARK: - Manual pairing (simulator / dev)

struct ManualPairingView: View {
    @Binding var daemonURL:  String
    @Binding var isPairing:  Bool
    @Binding var errorMsg:   String?
    @State private var apiToken = ""
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        ScrollView {
            VStack(spacing: 10) {
                Text("Daemon URL")
                    .font(.caption2)
                    .foregroundStyle(.secondary)

                TextField("http://host:7878", text: $daemonURL)
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .autocorrectionDisabled()

                Text("API Token")
                    .font(.caption2)
                    .foregroundStyle(.secondary)

                SecureField("Bearer token", text: $apiToken)
                    .font(.caption2)
                    .multilineTextAlignment(.center)

                if isPairing {
                    ProgressView("Pairing…")
                        .font(.caption2)
                } else {
                    Button("Connect") {
                        Task { await pair() }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(daemonURL.isEmpty || apiToken.isEmpty)
                }

                if let err = errorMsg {
                    Text(err)
                        .font(.caption2)
                        .foregroundStyle(.red)
                        .lineLimit(4)
                }
            }
            .padding()
        }
        .navigationTitle("Connect")
    }

    private func pair() async {
        isPairing = true
        errorMsg  = nil
        defer { isPairing = false }

        let base = daemonURL.hasSuffix("/") ? String(daemonURL.dropLast()) : daemonURL

        // 1. POST /watch/challenge with Bearer token
        guard let challengeURL = URL(string: "\(base)/watch/challenge") else {
            errorMsg = "Invalid URL"; return
        }
        do {
            var req = URLRequest(url: challengeURL)
            req.httpMethod = "POST"
            req.setValue("Bearer \(apiToken)", forHTTPHeaderField: "Authorization")
            req.setValue("application/json", forHTTPHeaderField: "Content-Type")
            req.httpBody = Data("{}".utf8)

            let (challengeData, response) = try await URLSession.shared.data(for: req)
            if let http = response as? HTTPURLResponse, http.statusCode != 200 {
                let msg = String(data: challengeData, encoding: .utf8) ?? "HTTP \(http.statusCode)"
                errorMsg = msg; return
            }
            let challenge = try JSONDecoder().decode(WatchChallengeResponse.self, from: challengeData)

            // 2. Build pairing payload and register
            let payload = WatchPairingPayload(
                endpoint:   base,
                nonce:      challenge.nonce,
                machine_id: challenge.machine_id,
                expires_at: challenge.expires_at,
                version:    "1"
            )
            try await WatchAuthManager.shared.registerDevice(pairing: payload)
            dismiss()
        } catch {
            errorMsg = error.localizedDescription
        }
    }
}

/// Minimal decodable for the /watch/challenge response.
private struct WatchChallengeResponse: Decodable {
    let nonce:      String
    let machine_id: String
    let issued_at:  UInt64
    let expires_at: UInt64
}

// MARK: - Settings view

struct SettingsView: View {
    @StateObject private var auth = WatchAuthManager.shared
    @State private var showConfirmUnpair = false

    var body: some View {
        List {
            Section("Device") {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Machine")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    Text(auth.machineId.prefix(16) + "…")
                        .font(.caption)
                        .lineLimit(1)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text("Watch ID")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    Text(auth.deviceId.prefix(16) + "…")
                        .font(.caption)
                        .lineLimit(1)
                }
            }

            Section {
                Button(role: .destructive) {
                    showConfirmUnpair = true
                } label: {
                    Label("Unpair Watch", systemImage: "trash")
                        .foregroundStyle(.red)
                }
            }
        }
        .navigationTitle("Settings")
        .alert("Unpair?", isPresented: $showConfirmUnpair) {
            Button("Unpair", role: .destructive) { auth.unpair() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("All session access will be removed from this watch.")
        }
    }
}
