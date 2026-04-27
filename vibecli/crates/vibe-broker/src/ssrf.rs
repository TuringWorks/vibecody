//! SSRF guard. Ports the rules from
//! `vibeui/src-tauri/src/agent_executor.rs:21-56` and extends them so they
//! cover every tier and every tool, not just `fetch_url`.

use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsrfVerdict {
    Allow,
    Block,
}

#[derive(Debug, Default, Clone)]
pub struct SsrfGuard {
    pub allow_imds: bool,
    pub extra_blocked_hosts: Vec<String>,
    /// Hosts (literal IPs or hostnames) that bypass the default block list.
    /// Use sparingly: needed for cluster-internal endpoints reachable only
    /// over RFC1918 (after a deliberate operator decision), and used by
    /// tests that point at a loopback stub upstream.
    pub allow_hosts: Vec<String>,
}

impl SsrfGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_imds_allowed(mut self) -> Self {
        self.allow_imds = true;
        self
    }

    pub fn with_allow_host(mut self, host: impl Into<String>) -> Self {
        self.allow_hosts.push(host.into());
        self
    }

    /// Verdict for a parsed URL string. `Block` means refuse outright.
    pub fn check(&self, url: &str) -> SsrfVerdict {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return SsrfVerdict::Block,
        };
        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return SsrfVerdict::Block;
        }
        let host = match parsed.host_str() {
            Some(h) => h,
            None => return SsrfVerdict::Block,
        };
        if is_metadata_hostname(host) {
            return SsrfVerdict::Block;
        }
        if self
            .extra_blocked_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
        {
            return SsrfVerdict::Block;
        }
        if self
            .allow_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
        {
            return SsrfVerdict::Allow;
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            return self.check_ip(ip);
        }
        // Bracketed IPv6 ([::1])
        if host.starts_with('[') && host.ends_with(']') {
            if let Ok(ip) = host[1..host.len() - 1].parse::<IpAddr>() {
                return self.check_ip(ip);
            }
        }
        SsrfVerdict::Allow
    }

    fn check_ip(&self, ip: IpAddr) -> SsrfVerdict {
        if is_imds_ip(ip) {
            return if self.allow_imds {
                SsrfVerdict::Allow
            } else {
                SsrfVerdict::Block
            };
        }
        if is_blocked_ip(ip) {
            SsrfVerdict::Block
        } else {
            SsrfVerdict::Allow
        }
    }
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || is_ipv6_ula(v6)
                || is_ipv6_link_local(v6)
        }
    }
}

fn is_imds_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.octets() == [169, 254, 169, 254],
        _ => false,
    }
}

fn is_ipv6_ula(v6: std::net::Ipv6Addr) -> bool {
    let octets = v6.octets();
    (octets[0] & 0xfe) == 0xfc
}

fn is_ipv6_link_local(v6: std::net::Ipv6Addr) -> bool {
    let octets = v6.octets();
    octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80
}

fn is_metadata_hostname(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    matches!(
        h.as_str(),
        "metadata.google.internal"
            | "metadata"
            | "metadata.azure.com"
            | "metadata.azure.net"
            | "instance-data"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn blocks_localhost() {
        let g = SsrfGuard::new();
        assert_eq!(g.check("http://127.0.0.1/"), SsrfVerdict::Block);
        assert_eq!(g.check("http://localhost/"), SsrfVerdict::Allow);
        // We rely on DNS-time recheck for "localhost" to be enforced; the
        // pure-string check only blocks literal IPs and known metadata names.
    }

    #[test]
    fn blocks_rfc1918() {
        let g = SsrfGuard::new();
        for ip in ["10.0.0.1", "192.168.1.1", "172.16.0.5"] {
            assert_eq!(
                g.check(&format!("http://{ip}/")),
                SsrfVerdict::Block,
                "expected block for {ip}"
            );
        }
    }

    #[test]
    fn blocks_imds_by_default() {
        let g = SsrfGuard::new();
        assert_eq!(
            g.check("http://169.254.169.254/latest/"),
            SsrfVerdict::Block
        );
    }

    #[test]
    fn allows_imds_when_opted_in() {
        let g = SsrfGuard::new().with_imds_allowed();
        assert_eq!(
            g.check("http://169.254.169.254/latest/"),
            SsrfVerdict::Allow
        );
    }

    #[test]
    fn blocks_metadata_hostnames() {
        let g = SsrfGuard::new();
        assert_eq!(
            g.check("http://metadata.google.internal/"),
            SsrfVerdict::Block
        );
    }

    #[test]
    fn blocks_ipv6_loopback() {
        let g = SsrfGuard::new();
        assert_eq!(g.check("http://[::1]/"), SsrfVerdict::Block);
    }

    #[test]
    fn blocks_non_http_schemes() {
        let g = SsrfGuard::new();
        assert_eq!(g.check("file:///etc/passwd"), SsrfVerdict::Block);
        assert_eq!(g.check("ssh://example.com/"), SsrfVerdict::Block);
    }

    #[test]
    fn allows_public_host() {
        let g = SsrfGuard::new();
        assert_eq!(
            g.check("https://api.openai.com/v1/messages"),
            SsrfVerdict::Allow
        );
    }

    #[test]
    fn blocks_link_local_v4() {
        let ip = IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1));
        assert!(is_blocked_ip(ip));
    }
}
