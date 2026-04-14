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
    @State private var isScanning = false
    @State private var errorMsg: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                Image(systemName: "applewatch")
                    .font(.system(size: 32))
                    .foregroundStyle(.blue)

                Text("VibeCody")
                    .font(.headline)

                Text("Open VibeUI on Mac\nand tap **Watch** in Settings to pair.")
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)

                if let err = errorMsg {
                    Text(err)
                        .font(.caption2)
                        .foregroundStyle(.red)
                        .lineLimit(3)
                }
            }
            .padding()
        }
        .navigationTitle("Pair Watch")
    }
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
