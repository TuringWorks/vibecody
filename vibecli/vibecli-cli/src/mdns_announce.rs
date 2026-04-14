//! Zero-config mDNS/DNS-SD announcer for VibeCLI daemon.
//!
//! Announces `_vibecli._tcp.local.` on the mDNS multicast address
//! (224.0.0.251:5353) so that mobile clients on the same LAN can discover
//! the daemon without manual IP configuration — regardless of the 10.x.x.x /
//! 192.168.x.x / 172.16.x.x subnet in use.
//!
//! No external crates are required beyond `socket2` (for `SO_REUSEPORT` on the
//! listener socket) and the `tokio` that is already in scope via the workspace.
//!
//! # Protocol overview
//! We send a Multicast DNS "announcement" (unsolicited response) containing:
//!   PTR  _vibecli._tcp.local.     → <host>.vibecli._vibecli._tcp.local.
//!   SRV  <host>.vibecli…          → priority 0, weight 0, port <port>, <host>.local.
//!   TXT  <host>.vibecli…          → machine_id=<id> version=<ver>
//!   A    <host>.local.            → <ip>  (primary outbound interface)
//!
//! Announcements are sent on startup and every 60 s thereafter.  The listener
//! goroutine also answers incoming PTR queries from mobile clients.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket as StdUdpSocket};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::interval;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const ANNOUNCE_INTERVAL_SECS: u64 = 60;

// ── Public API ────────────────────────────────────────────────────────────────

/// Spawns the mDNS announcer + listener as background Tokio tasks.
/// Must be called inside a Tokio runtime.
pub fn start(port: u16, machine_id: String) {
    let mid = machine_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_announcer(port, &mid).await {
            eprintln!("[mdns] announcer stopped: {e}");
        }
    });
    tokio::spawn(async move {
        if let Err(e) = run_listener(port, &machine_id).await {
            // Non-fatal: another process (avahi, mDNSResponder) may hold port 5353.
            eprintln!("[mdns] listener not started: {e}");
        }
    });
}

// ── Announcer (periodic unsolicited responses) ────────────────────────────────

async fn run_announcer(port: u16, machine_id: &str) -> std::io::Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    sock.set_multicast_ttl_v4(255)?;

    let dest = SocketAddr::V4(SocketAddrV4::new(MDNS_ADDR, MDNS_PORT));
    let hostname = get_hostname();
    let version = env!("CARGO_PKG_VERSION");
    let local_ips = local_ipv4_addrs();

    let mut ticker = interval(Duration::from_secs(ANNOUNCE_INTERVAL_SECS));
    loop {
        ticker.tick().await; // first tick is immediate
        let packet = build_announce(port, machine_id, &hostname, version, &local_ips);
        if let Err(e) = sock.send_to(&packet, dest).await {
            eprintln!("[mdns] send error: {e}");
        }
    }
}

// ── Listener (answers incoming PTR queries) ───────────────────────────────────

async fn run_listener(port: u16, machine_id: &str) -> std::io::Result<()> {
    // We need SO_REUSEPORT so multiple processes can bind port 5353.
    let std_sock: StdUdpSocket = {
        use socket2::{Domain, Protocol, Socket, Type};
        let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        s.set_reuse_address(true)?;
        #[cfg(unix)]
        s.set_reuse_port(true)?;
        s.set_nonblocking(true)?;
        s.bind(&socket2::SockAddr::from(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            MDNS_PORT,
        )))?;
        let std: StdUdpSocket = s.into();
        std.join_multicast_v4(&MDNS_ADDR, &Ipv4Addr::UNSPECIFIED)?;
        std
    };

    let sock = UdpSocket::from_std(std_sock)?;
    let hostname = get_hostname();
    let version = env!("CARGO_PKG_VERSION");
    let local_ips = local_ipv4_addrs();
    let mut buf = [0u8; 1500];

    loop {
        let (len, _src) = match sock.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if is_vibecli_ptr_query(&buf[..len]) {
            let packet = build_announce(port, machine_id, &hostname, version, &local_ips);
            let dest = SocketAddr::V4(SocketAddrV4::new(MDNS_ADDR, MDNS_PORT));
            let _ = sock.send_to(&packet, dest).await;
        }
    }
}

// ── DNS packet builder ────────────────────────────────────────────────────────

fn build_announce(
    port: u16,
    machine_id: &str,
    hostname: &str,
    version: &str,
    local_ips: &[Ipv4Addr],
) -> Vec<u8> {
    let instance = format!("{hostname}.vibecli");
    let fqdn_instance = format!("{instance}._vibecli._tcp.local.");
    let fqdn_host = format!("{hostname}.local.");

    let mut answers: Vec<u8> = Vec::new();
    let mut count: u16 = 0;

    // PTR _vibecli._tcp.local. → <instance>._vibecli._tcp.local.
    push_rr(&mut answers, &encode_name("_vibecli._tcp.local."), 12, &encode_name(&fqdn_instance));
    count += 1;

    // SRV <instance>._vibecli._tcp.local. → <host>.local.  port
    {
        let mut rd = Vec::new();
        rd.extend_from_slice(&0u16.to_be_bytes()); // priority
        rd.extend_from_slice(&0u16.to_be_bytes()); // weight
        rd.extend_from_slice(&port.to_be_bytes());
        rd.extend_from_slice(&encode_name(&fqdn_host));
        push_rr(&mut answers, &encode_name(&fqdn_instance), 33, &rd);
        count += 1;
    }

    // TXT machine_id + version
    {
        let mut rd = Vec::new();
        push_txt_kv(&mut rd, &format!("machine_id={machine_id}"));
        push_txt_kv(&mut rd, &format!("version={version}"));
        push_rr(&mut answers, &encode_name(&fqdn_instance), 16, &rd);
        count += 1;
    }

    // A records
    for ip in local_ips {
        push_rr(&mut answers, &encode_name(&fqdn_host), 1, &ip.octets());
        count += 1;
    }

    // DNS header (12 bytes): ID=0, QR=1 AA=1, ANCOUNT=count
    let mut pkt = Vec::with_capacity(12 + answers.len());
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // ID
    pkt.extend_from_slice(&0x8400u16.to_be_bytes()); // QR=1 AA=1
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // QDCOUNT
    pkt.extend_from_slice(&count.to_be_bytes());     // ANCOUNT
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // NSCOUNT
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // ARCOUNT
    pkt.extend_from_slice(&answers);
    pkt
}

fn push_rr(buf: &mut Vec<u8>, name: &[u8], rtype: u16, rdata: &[u8]) {
    buf.extend_from_slice(name);
    buf.extend_from_slice(&rtype.to_be_bytes());
    buf.extend_from_slice(&0x8001u16.to_be_bytes()); // IN + cache-flush
    buf.extend_from_slice(&120u32.to_be_bytes());    // TTL 120 s
    buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
    buf.extend_from_slice(rdata);
}

fn encode_name(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for label in name.trim_end_matches('.').split('.') {
        let b = label.as_bytes();
        out.push(b.len() as u8);
        out.extend_from_slice(b);
    }
    out.push(0);
    out
}

fn push_txt_kv(rd: &mut Vec<u8>, kv: &str) {
    let b = kv.as_bytes();
    rd.push(b.len() as u8);
    rd.extend_from_slice(b);
}

/// Returns true if the DNS message is a PTR query for `_vibecli._tcp.local.`
fn is_vibecli_ptr_query(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    // QR bit must be 0 (query)
    let flags = u16::from_be_bytes([data[2], data[3]]);
    if flags & 0x8000 != 0 {
        return false;
    }
    let qdcount = u16::from_be_bytes([data[4], data[5]]);
    if qdcount == 0 {
        return false;
    }
    // Byte-search for `\x08_vibecli\x04_tcp\x05local\x00`
    let needle = b"\x08_vibecli\x04_tcp\x05local\x00";
    data[12..].windows(needle.len()).any(|w| w == needle)
}

// ── System helpers ────────────────────────────────────────────────────────────

fn get_hostname() -> String {
    // Try env var first (set by most shells)
    if let Ok(h) = std::env::var("HOSTNAME") {
        let short = h.split('.').next().unwrap_or(&h).to_string();
        if !short.is_empty() {
            return short;
        }
    }
    // Fall back to the `hostname` binary (available on macOS, Linux, Windows WSL)
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            let trimmed = s.trim();
            trimmed.split('.').next().unwrap_or(trimmed).to_string()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "vibecli-host".to_string())
}

/// Returns all non-loopback IPv4 addresses on this host.
/// Uses a UDP connect trick (no packets sent) to get the primary outbound
/// interface IP, then optionally enriches with `ip addr` / `ifconfig` output.
fn local_ipv4_addrs() -> Vec<Ipv4Addr> {
    let mut addrs = Vec::new();

    // Primary outbound IP via UDP connect trick
    if let Ok(sock) = StdUdpSocket::bind("0.0.0.0:0") {
        // Connect to a routable address — no packet is actually sent.
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = sock.local_addr() {
                if let IpAddr::V4(ip) = local.ip() {
                    if !ip.is_loopback() {
                        addrs.push(ip);
                    }
                }
            }
        }
    }

    // Enumerate additional IPs from `ip addr show` (Linux) or `ifconfig` (macOS)
    let extra = parse_ip_addrs_from_cmd();
    for ip in extra {
        if !addrs.contains(&ip) {
            addrs.push(ip);
        }
    }

    addrs
}

fn parse_ip_addrs_from_cmd() -> Vec<Ipv4Addr> {
    let mut result = Vec::new();

    // Try `ip addr show` (Linux / WSL)
    let output = std::process::Command::new("ip")
        .args(["addr", "show"])
        .output()
        .or_else(|_| {
            // Fall back to `ifconfig` (macOS / BSD)
            std::process::Command::new("ifconfig").output()
        });

    let text = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => return result,
    };

    for line in text.lines() {
        let trimmed = line.trim();
        // `ip addr` format: "inet 10.0.0.5/24 brd ..."
        // `ifconfig` format: "inet 10.0.0.5 netmask ..."
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            let addr_str = rest.split(['/', ' ']).next().unwrap_or("").trim();
            if let Ok(ip) = addr_str.parse::<Ipv4Addr>() {
                if !ip.is_loopback() && !ip.is_link_local() {
                    result.push(ip);
                }
            }
        }
    }

    result
}
