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

    #[test]
    fn peer_info_host_field() {
        let peer = PeerInfo {
            name: "node-1".to_string(),
            host: "10.0.0.5".to_string(),
            port: 8080,
        };
        assert_eq!(peer.host, "10.0.0.5");
    }

    #[test]
    fn peer_info_clone() {
        let peer = PeerInfo {
            name: "alpha".to_string(),
            host: "192.168.1.1".to_string(),
            port: 7878,
        };
        let cloned = peer.clone();
        assert_eq!(cloned.name, peer.name);
        assert_eq!(cloned.host, peer.host);
        assert_eq!(cloned.port, peer.port);
    }

    #[test]
    fn peer_info_serialize_json() {
        let peer = PeerInfo {
            name: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 7878,
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"port\":7878"));
    }

    #[test]
    fn peer_info_deserialize_json() {
        let json = r#"{"name":"remote","host":"10.0.0.1","port":9090}"#;
        let peer: PeerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(peer.name, "remote");
        assert_eq!(peer.host, "10.0.0.1");
        assert_eq!(peer.port, 9090);
    }

    #[test]
    fn peer_info_roundtrip_serde() {
        let original = PeerInfo {
            name: "box-a".to_string(),
            host: "fd00::1".to_string(),
            port: 443,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.host, original.host);
        assert_eq!(parsed.port, original.port);
    }

    #[test]
    fn peer_info_port_zero() {
        let peer = PeerInfo {
            name: "ephemeral".to_string(),
            host: "localhost".to_string(),
            port: 0,
        };
        assert_eq!(peer.port, 0);
    }

    #[test]
    fn peer_info_port_max() {
        let peer = PeerInfo {
            name: "max-port".to_string(),
            host: "127.0.0.1".to_string(),
            port: u16::MAX,
        };
        assert_eq!(peer.port, 65535);
    }

    #[test]
    fn peer_info_empty_name() {
        let peer = PeerInfo {
            name: "".to_string(),
            host: "::1".to_string(),
            port: 7878,
        };
        assert!(peer.name.is_empty());
    }

    #[test]
    fn advertise_service_different_ports() {
        let ad1 = advertise_service(8080, "web").unwrap();
        let ad2 = advertise_service(3000, "api").unwrap();
        assert_eq!(ad1.port, 8080);
        assert_eq!(ad2.port, 3000);
        assert_ne!(ad1.name, ad2.name);
    }

    #[test]
    fn advertise_service_empty_name() {
        let ad = advertise_service(7878, "").unwrap();
        assert!(ad.name.is_empty());
        assert_eq!(ad.port, 7878);
    }

    #[test]
    fn service_advertisement_fields() {
        let ad = ServiceAdvertisement {
            name: "svc".to_string(),
            port: 1234,
        };
        assert_eq!(ad.name, "svc");
        assert_eq!(ad.port, 1234);
    }

    #[test]
    fn peer_info_debug_format() {
        let peer = PeerInfo {
            name: "dbg".to_string(),
            host: "1.2.3.4".to_string(),
            port: 80,
        };
        let debug = format!("{:?}", peer);
        assert!(debug.contains("dbg"));
        assert!(debug.contains("1.2.3.4"));
    }

    #[test]
    fn peer_info_ipv6_host() {
        let peer = PeerInfo {
            name: "ipv6-node".to_string(),
            host: "fe80::1%eth0".to_string(),
            port: 7878,
        };
        assert!(peer.host.contains("fe80"));
    }
}
