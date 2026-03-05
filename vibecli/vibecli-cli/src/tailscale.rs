#![allow(dead_code)]
//! Tailscale integration for secure remote access to VibeCLI daemon.
//!
//! Provides Tailscale status checking and funnel setup for exposing
//! the VibeCLI daemon securely over Tailscale's mesh VPN.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Tailscale connection status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleInfo {
    /// Whether Tailscale is running and connected.
    pub connected: bool,
    /// The Tailscale IP address (e.g., "100.x.y.z").
    pub tailscale_ip: Option<String>,
    /// The machine hostname on the tailnet.
    pub hostname: Option<String>,
    /// The tailnet name (e.g., "user@example.com").
    pub tailnet: Option<String>,
}

/// Get the current Tailscale status.
pub fn tailscale_status() -> Result<TailscaleInfo> {
    let output = std::process::Command::new("tailscale")
        .arg("status")
        .arg("--json")
        .output()
        .context("Failed to run 'tailscale status'. Is Tailscale installed?")?;

    if !output.status.success() {
        return Ok(TailscaleInfo {
            connected: false,
            tailscale_ip: None,
            hostname: None,
            tailnet: None,
        });
    }

    let status: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse tailscale status JSON")?;

    let self_info = &status["Self"];
    let tailscale_ip = self_info["TailscaleIPs"]
        .as_array()
        .and_then(|ips| ips.first())
        .and_then(|ip| ip.as_str())
        .map(|s| s.to_string());

    let hostname = self_info["HostName"].as_str().map(|s| s.to_string());
    let tailnet = status["CurrentTailnet"]["Name"]
        .as_str()
        .map(|s| s.to_string());

    Ok(TailscaleInfo {
        connected: true,
        tailscale_ip,
        hostname,
        tailnet,
    })
}

/// Serve the VibeCLI daemon via Tailscale Funnel (publicly accessible over HTTPS).
///
/// This starts `tailscale funnel <port>` in the background.
pub async fn serve_via_funnel(port: u16) -> Result<tokio::process::Child> {
    let child = tokio::process::Command::new("tailscale")
        .args(["funnel", &port.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to start tailscale funnel")?;

    tracing::info!("[tailscale] Started funnel on port {}", port);
    Ok(child)
}

/// Serve the VibeCLI daemon via Tailscale Serve (accessible only within tailnet).
pub async fn serve_via_tailscale(port: u16) -> Result<tokio::process::Child> {
    let child = tokio::process::Command::new("tailscale")
        .args(["serve", &port.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to start tailscale serve")?;

    tracing::info!("[tailscale] Started serve on port {}", port);
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailscale_info_default() {
        let info = TailscaleInfo {
            connected: false,
            tailscale_ip: None,
            hostname: None,
            tailnet: None,
        };
        assert!(!info.connected);
        assert!(info.tailscale_ip.is_none());
    }

    #[test]
    fn tailscale_info_connected() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.64.0.1".to_string()),
            hostname: Some("my-machine".to_string()),
            tailnet: Some("user@example.com".to_string()),
        };
        assert!(info.connected);
        assert_eq!(info.tailscale_ip.as_deref(), Some("100.64.0.1"));
    }
}
