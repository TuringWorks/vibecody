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

/// Returns the public HTTPS Funnel URL for this machine if Tailscale Funnel
/// is active on the given port, e.g. `https://my-mac.tailnet-abc.ts.net`.
///
/// Parses `tailscale status --json`:
///   - `Self.DNSName`    → `<machine>.<tailnet>.ts.net.`
///   - `Self.FunnelPorts` → `[443]` when funnel is active
///
/// The daemon port is NOT appended because Tailscale Funnel reverse-proxies
/// port 443 to the local daemon port automatically.
pub fn tailscale_funnel_url(_port: u16) -> Option<String> {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let status: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    // Check that the funnel is active (FunnelPorts contains 443).
    let funnel_ports = status["Self"]["FunnelPorts"].as_array();
    let funnel_active = funnel_ports
        .map(|ports| ports.iter().any(|p| p.as_u64() == Some(443)))
        .unwrap_or(false);

    if !funnel_active {
        return None;
    }

    // DNSName is e.g. "my-machine.tailnet-abc.ts.net." (trailing dot)
    let dns_name = status["Self"]["DNSName"].as_str()?;
    let host = dns_name.trim_end_matches('.');
    if host.is_empty() {
        return None;
    }

    Some(format!("https://{host}"))
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

    #[test]
    fn tailscale_info_hostname_field() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.1.2.3".to_string()),
            hostname: Some("dev-box".to_string()),
            tailnet: None,
        };
        assert_eq!(info.hostname.as_deref(), Some("dev-box"));
    }

    #[test]
    fn tailscale_info_tailnet_field() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: None,
            hostname: None,
            tailnet: Some("acme.corp".to_string()),
        };
        assert_eq!(info.tailnet.as_deref(), Some("acme.corp"));
    }

    #[test]
    fn tailscale_info_clone() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.0.0.1".to_string()),
            hostname: Some("host".to_string()),
            tailnet: Some("net".to_string()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.connected, info.connected);
        assert_eq!(cloned.tailscale_ip, info.tailscale_ip);
        assert_eq!(cloned.hostname, info.hostname);
        assert_eq!(cloned.tailnet, info.tailnet);
    }

    #[test]
    fn tailscale_info_serialize_json() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.10.0.1".to_string()),
            hostname: Some("node1".to_string()),
            tailnet: Some("example.com".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"connected\":true"));
        assert!(json.contains("100.10.0.1"));
        assert!(json.contains("node1"));
    }

    #[test]
    fn tailscale_info_deserialize_json() {
        let json = r#"{"connected":false,"tailscale_ip":null,"hostname":null,"tailnet":null}"#;
        let info: TailscaleInfo = serde_json::from_str(json).unwrap();
        assert!(!info.connected);
        assert!(info.tailscale_ip.is_none());
    }

    #[test]
    fn tailscale_info_roundtrip_serde() {
        let original = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.64.1.2".to_string()),
            hostname: Some("test-host".to_string()),
            tailnet: Some("ts-net".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TailscaleInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.connected, original.connected);
        assert_eq!(parsed.tailscale_ip, original.tailscale_ip);
        assert_eq!(parsed.hostname, original.hostname);
        assert_eq!(parsed.tailnet, original.tailnet);
    }

    #[test]
    fn tailscale_info_all_none_fields() {
        let info = TailscaleInfo {
            connected: false,
            tailscale_ip: None,
            hostname: None,
            tailnet: None,
        };
        assert!(info.tailscale_ip.is_none());
        assert!(info.hostname.is_none());
        assert!(info.tailnet.is_none());
    }

    #[test]
    fn tailscale_info_all_some_fields() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.0.0.1".to_string()),
            hostname: Some("h".to_string()),
            tailnet: Some("t".to_string()),
        };
        assert!(info.tailscale_ip.is_some());
        assert!(info.hostname.is_some());
        assert!(info.tailnet.is_some());
    }

    #[test]
    fn tailscale_info_debug_format() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.1.1.1".to_string()),
            hostname: None,
            tailnet: None,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("TailscaleInfo"));
        assert!(debug.contains("100.1.1.1"));
    }

    #[test]
    fn tailscale_info_partial_fields() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("100.2.3.4".to_string()),
            hostname: None,
            tailnet: Some("my-tailnet".to_string()),
        };
        assert!(info.connected);
        assert!(info.hostname.is_none());
        assert_eq!(info.tailnet.as_deref(), Some("my-tailnet"));
    }

    #[test]
    fn tailscale_info_deserialize_with_all_fields() {
        let json = r#"{
            "connected": true,
            "tailscale_ip": "100.50.60.70",
            "hostname": "workstation",
            "tailnet": "corp.example.com"
        }"#;
        let info: TailscaleInfo = serde_json::from_str(json).unwrap();
        assert!(info.connected);
        assert_eq!(info.tailscale_ip.as_deref(), Some("100.50.60.70"));
        assert_eq!(info.hostname.as_deref(), Some("workstation"));
        assert_eq!(info.tailnet.as_deref(), Some("corp.example.com"));
    }

    #[test]
    fn tailscale_info_ipv6_tailscale_ip() {
        let info = TailscaleInfo {
            connected: true,
            tailscale_ip: Some("fd7a:115c:a1e0::1".to_string()),
            hostname: None,
            tailnet: None,
        };
        assert!(info.tailscale_ip.as_deref().unwrap().contains("fd7a"));
    }

    #[test]
    fn tailscale_info_empty_string_fields() {
        let info = TailscaleInfo {
            connected: false,
            tailscale_ip: Some("".to_string()),
            hostname: Some("".to_string()),
            tailnet: Some("".to_string()),
        };
        assert_eq!(info.tailscale_ip.as_deref(), Some(""));
        assert_eq!(info.hostname.as_deref(), Some(""));
    }
}
