// VibeCodyWatchApp.swift — App entry point
import SwiftUI

@main
struct VibeCodyWatchApp: App {
    @StateObject private var auth    = WatchAuthManager.shared
    @StateObject private var network = WatchNetworkManager.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(auth)
                .environmentObject(network)
        }
    }
}
