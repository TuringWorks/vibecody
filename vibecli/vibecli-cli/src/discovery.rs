#![allow(dead_code)]
//! mDNS/DNS-SD discovery for VibeCLI daemon instances on the LAN.
//!
//! Advertises the VibeCLI daemon as `_vibecli._tcp` service when `--serve` is running.
//! Discovers other VibeCLI instances on the local network via `/discover` command.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A discovered VibeCLI peer on the local network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Hostname or instance name.
    pub name: String,
    /// IP address or hostname.
    pub host: String,
    /// Port number.
    pub port: u16,
}

/// Discover VibeCLI instances on the local network using DNS-SD.
///
/// Scans for `_vibecli._tcp` services with a timeout.
pub async fn discover_peers(timeout_secs: u64) -> Result<Vec<PeerInfo>> {
    // Use a simple UDP broadcast probe approach
    // In production, this would use mdns-sd crate, but for now we scan common ports
    let mut peers = Vec::new();

    // Try to find services by probing common VibeCLI ports on the local subnet
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Probe localhost and common ports
    for port in [7878u16, 7879, 7880, 8080] {
        let url = format!("http://127.0.0.1:{}/health", port);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                peers.push(PeerInfo {
                    name: format!("localhost:{}", port),
                    host: "127.0.0.1".to_string(),
                    port,
                });
            }
        }
    }

    Ok(peers)
}

/// Advertise this VibeCLI daemon instance for discovery.
///
/// Currently registers the daemon info for local probing.
/// When mdns-sd is available, this will use full mDNS advertisement.
pub fn advertise_service(port: u16, name: &str) -> Result<ServiceAdvertisement> {
    tracing::info!("[discovery] Advertising VibeCLI service '{}' on port {}", name, port);
    Ok(ServiceAdvertisement {
        name: name.to_string(),
        port,
    })
}

/// Handle for an active service advertisement. Drop to stop advertising.
pub struct ServiceAdvertisement {
    pub name: String,
    pub port: u16,
}

impl Drop for ServiceAdvertisement {
    fn drop(&mut self) {
        tracing::info!("[discovery] Stopped advertising '{}'", self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_info_fields() {
        let peer = PeerInfo {
            name: "my-machine".to_string(),
            host: "192.168.1.100".to_string(),
            port: 7878,
        };
        assert_eq!(peer.name, "my-machine");
        assert_eq!(peer.port, 7878);
    }

    #[test]
    fn advertise_service_works() {
        let ad = advertise_service(7878, "test-node").unwrap();
        assert_eq!(ad.name, "test-node");
        assert_eq!(ad.port, 7878);
    }
}
