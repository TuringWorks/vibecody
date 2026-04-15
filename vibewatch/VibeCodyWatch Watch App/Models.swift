// Models.swift — VibeCody Watch App data models
// Mirrors the Rust watch_session_relay.rs structs exactly.

import Foundation

// MARK: - Session

struct WatchSessionSummary: Codable, Identifiable {
    var id: String { session_id }
    let session_id: String
    let task_preview: String
    let status: String           // "running" | "complete" | "failed"
    let provider: String
    let model: String
    let message_count: Int
    let step_count: Int
    let started_at: TimeInterval
    let last_activity: TimeInterval
    let last_message_preview: String

    var isRunning: Bool { status == "running" }
    var statusIcon: String {
        switch status {
        case "running":  return "⚡"
        case "complete": return "✓"
        case "failed":   return "✗"
        default:         return "○"
        }
    }
    var lastActivityDate: Date { Date(timeIntervalSince1970: last_activity) }
}

struct WatchMessage: Codable, Identifiable {
    let id: Int
    let role: String
    let content: String
    let created_at: TimeInterval

    var isUser: Bool { role == "user" }
    var isAssistant: Bool { role == "assistant" }
    var date: Date { Date(timeIntervalSince1970: created_at) }
}

// MARK: - Streaming

struct WatchAgentEvent: Codable {
    let kind: String      // "delta" | "tool_start" | "tool_end" | "done" | "error"
    let delta: String?
    let tool: String?
    let status: String?
    let error: String?
    let step: Int?
}

// MARK: - Dispatch

struct WatchDispatchRequest: Codable {
    let session_id: String?
    let content: String
    let provider: String?
    let nonce: String
    let timestamp: UInt64
}

struct WatchDispatchResponse: Codable {
    let session_id: String
    let message_id: Int
    let streaming_url: String
}

// MARK: - Auth

struct WatchRegisterRequest: Codable {
    let device_id: String
    let name: String
    let os_version: String
    let model: String
    let public_key_b64: String
    let signature_b64: String
    let nonce: String
    let device_check_token: String?
}

struct WatchRegisterResponse: Codable {
    let device_id: String
    let access_token: String
    let refresh_token: String
    let expires_in: Int
    let expires_at: UInt64
}

struct WatchRefreshRequest: Codable {
    let device_id: String
    let refresh_token: String
    let proof_signature_b64: String
    let timestamp: UInt64
}

struct WatchRefreshResponse: Codable {
    let access_token: String
    let refresh_token: String
    let expires_at: UInt64
    let expires_in: Int
}

// MARK: - Wrist

struct WristEvent: Codable {
    let device_id: String
    let on_wrist: Bool
    let timestamp: UInt64
    let signature_b64: String
}

// MARK: - Sandbox

struct WatchSandboxStatus: Codable, Identifiable {
    var id: String { container_id }
    let container_id: String
    let session_id: String?
    let state: String
    let uptime_secs: UInt64
    let cpu_pct: Float
    let mem_mb: UInt64
    let mem_limit_mb: UInt64
    let last_output_lines: [String]
    let exit_code: Int?

    var isRunning: Bool { state == "running" }
    var memPct: Float { mem_limit_mb > 0 ? Float(mem_mb) / Float(mem_limit_mb) * 100 : 0 }
}

// MARK: - Pairing QR payload

struct WatchPairingPayload: Codable {
    let endpoint: String
    let nonce: String
    let machine_id: String
    let expires_at: UInt64
    let version: String
}

// MARK: - Sandbox control

struct WatchSandboxControlRequest: Codable {
    let action: String   // "pause" | "resume" | "stop"
    let nonce: String
    let timestamp: UInt64
}

// MARK: - Beacon

struct WatchBeacon: Codable {
    let machine_id: String
    let api_version: String
    let watch_supported: Bool
    let tailscale_ip: String?
    let uptime_secs: UInt64
}
