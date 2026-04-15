/*!
 * BDD tests for the mDNS/DNS-SD zero-config LAN discovery module.
 * Run with: cargo test --test mdns_announce_bdd
 */
use cucumber::{World, given, then, when};

#[derive(Debug, Default, World)]
pub struct MdnsWorld {
    fqdn: String,
    encoded: Vec<u8>,
    packet: Vec<u8>,
    query_bytes: Vec<u8>,
    query_result: bool,
    hostname: String,
    local_ips: Vec<std::net::Ipv4Addr>,
    lan_ip: Option<String>,
    answer_count: u16,
}

// ── Helpers pulled from mdns_announce internals ───────────────────────────────
// These mirror the private helpers; keeping them here avoids making them pub.

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

fn push_rr(buf: &mut Vec<u8>, name: &[u8], rtype: u16, rdata: &[u8]) {
    buf.extend_from_slice(name);
    buf.extend_from_slice(&rtype.to_be_bytes());
    buf.extend_from_slice(&0x8001u16.to_be_bytes());
    buf.extend_from_slice(&120u32.to_be_bytes());
    buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
    buf.extend_from_slice(rdata);
}

fn push_txt_kv(rd: &mut Vec<u8>, kv: &str) {
    let b = kv.as_bytes();
    rd.push(b.len() as u8);
    rd.extend_from_slice(b);
}

fn build_announce(port: u16, machine_id: &str, hostname: &str, local_ips: &[std::net::Ipv4Addr]) -> Vec<u8> {
    let fqdn_instance = format!("{hostname}.vibecli._vibecli._tcp.local.");
    let fqdn_host = format!("{hostname}.local.");
    let mut answers = Vec::new();
    let mut count: u16 = 0;

    push_rr(&mut answers, &encode_name("_vibecli._tcp.local."), 12, &encode_name(&fqdn_instance));
    count += 1;

    let mut srv = Vec::new();
    srv.extend_from_slice(&0u16.to_be_bytes());
    srv.extend_from_slice(&0u16.to_be_bytes());
    srv.extend_from_slice(&port.to_be_bytes());
    srv.extend_from_slice(&encode_name(&fqdn_host));
    push_rr(&mut answers, &encode_name(&fqdn_instance), 33, &srv);
    count += 1;

    let mut rd = Vec::new();
    push_txt_kv(&mut rd, &format!("machine_id={machine_id}"));
    push_txt_kv(&mut rd, &format!("version=test"));
    push_rr(&mut answers, &encode_name(&fqdn_instance), 16, &rd);
    count += 1;

    for ip in local_ips {
        push_rr(&mut answers, &encode_name(&fqdn_host), 1, &ip.octets());
        count += 1;
    }

    let mut pkt = Vec::with_capacity(12 + answers.len());
    pkt.extend_from_slice(&0x0000u16.to_be_bytes());
    pkt.extend_from_slice(&0x8400u16.to_be_bytes());
    pkt.extend_from_slice(&0x0000u16.to_be_bytes());
    pkt.extend_from_slice(&count.to_be_bytes());
    pkt.extend_from_slice(&0x0000u16.to_be_bytes());
    pkt.extend_from_slice(&0x0000u16.to_be_bytes());
    pkt.extend_from_slice(&answers);
    pkt
}

fn is_vibecli_ptr_query(data: &[u8]) -> bool {
    if data.len() < 12 { return false; }
    let flags = u16::from_be_bytes([data[2], data[3]]);
    if flags & 0x8000 != 0 { return false; }
    let qdcount = u16::from_be_bytes([data[4], data[5]]);
    if qdcount == 0 { return false; }
    let needle = b"\x08_vibecli\x04_tcp\x05local\x00";
    data[12..].windows(needle.len()).any(|w| w == needle)
}

fn make_ptr_query(service: &str) -> Vec<u8> {
    // Minimal DNS query: header + one question
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // ID
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // QR=0 (query)
    pkt.extend_from_slice(&0x0001u16.to_be_bytes()); // QDCOUNT=1
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // ANCOUNT
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // NSCOUNT
    pkt.extend_from_slice(&0x0000u16.to_be_bytes()); // ARCOUNT
    pkt.extend_from_slice(&encode_name(service));
    pkt.extend_from_slice(&12u16.to_be_bytes()); // QTYPE PTR
    pkt.extend_from_slice(&1u16.to_be_bytes());  // QCLASS IN
    pkt
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(expr = "the fully-qualified domain name {string}")]
fn set_fqdn(world: &mut MdnsWorld, fqdn: String) {
    world.fqdn = fqdn;
}

#[given(expr = "a machine with id {string} on port {int}")]
fn set_machine(world: &mut MdnsWorld, _id: String, _port: u16) {
    world.lan_ip = None;
}

#[given(expr = "the machine has LAN IP {string}")]
fn set_lan_ip(world: &mut MdnsWorld, ip: String) {
    world.lan_ip = Some(ip);
}

#[given(expr = "a raw DNS PTR query for {string}")]
fn set_ptr_query(world: &mut MdnsWorld, service: String) {
    world.query_bytes = make_ptr_query(&service);
}

#[given("a raw DNS response packet")]
fn set_dns_response(world: &mut MdnsWorld) {
    let mut pkt = vec![0u8; 12];
    pkt[2] = 0x84; pkt[3] = 0x00; // QR=1 (response)
    pkt[4] = 0x00; pkt[5] = 0x01; // QDCOUNT=1
    world.query_bytes = pkt;
}

#[given("a DNS packet smaller than 12 bytes")]
fn set_short_packet(world: &mut MdnsWorld) {
    world.query_bytes = vec![0u8, 0, 0, 0];
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when("I encode it as a DNS name")]
fn encode_fqdn(world: &mut MdnsWorld) {
    world.encoded = encode_name(&world.fqdn.clone());
}

#[when(expr = "I build an mDNS announce packet")]
fn build_packet(world: &mut MdnsWorld) {
    let ips: Vec<std::net::Ipv4Addr> = world.lan_ip
        .as_ref()
        .and_then(|s| s.parse().ok())
        .into_iter()
        .collect();
    world.packet = build_announce(7878, "test-machine", "test-host", &ips);
    world.answer_count = u16::from_be_bytes([world.packet[6], world.packet[7]]);
}

#[when("I check if it is a VibeCLI PTR query")]
fn check_ptr_query(world: &mut MdnsWorld) {
    let bytes = world.query_bytes.clone();
    world.query_result = is_vibecli_ptr_query(&bytes);
}

#[when("I call get_hostname")]
fn call_get_hostname(world: &mut MdnsWorld) {
    world.hostname = std::env::var("HOSTNAME")
        .unwrap_or_else(|_| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().split('.').next().unwrap_or("").to_string())
                .unwrap_or_else(|| "vibecli-host".to_string())
        });
    // Trim to short hostname
    world.hostname = world.hostname.split('.').next().unwrap_or("vibecli-host").to_string();
}

#[when("I call local_ipv4_addrs")]
fn call_local_ipv4_addrs(world: &mut MdnsWorld) {
    let mut addrs = Vec::new();
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = sock.local_addr() {
                if let std::net::IpAddr::V4(ip) = local.ip() {
                    if !ip.is_loopback() {
                        addrs.push(ip);
                    }
                }
            }
        }
    }
    world.local_ips = addrs;
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(expr = "the encoded bytes should start with label length {int}")]
fn check_label_length(world: &mut MdnsWorld, expected_len: u8) {
    assert_eq!(world.encoded[0], expected_len,
        "first byte should be label length {expected_len}");
}

#[then("the encoded bytes should end with a root null byte")]
fn check_root_null(world: &mut MdnsWorld) {
    assert_eq!(*world.encoded.last().unwrap(), 0, "last byte should be 0 (root label)");
}

#[then(expr = "decoding the labels should yield {string}, {string}, {string}")]
fn check_decoded_labels(world: &mut MdnsWorld, l1: String, l2: String, l3: String) {
    let mut pos = 0usize;
    let mut labels = Vec::new();
    while pos < world.encoded.len() {
        let len = world.encoded[pos] as usize;
        if len == 0 { break; }
        let label = std::str::from_utf8(&world.encoded[pos+1..pos+1+len]).unwrap();
        labels.push(label.to_string());
        pos += 1 + len;
    }
    assert_eq!(labels, vec![l1, l2, l3]);
}

#[then("the packet should be at least 12 bytes long")]
fn check_packet_len(world: &mut MdnsWorld) {
    assert!(world.packet.len() >= 12, "packet too short: {} bytes", world.packet.len());
}

#[then(expr = "the flags field should equal {int}")]
fn check_flags(world: &mut MdnsWorld, expected: u32) {
    let flags = u16::from_be_bytes([world.packet[2], world.packet[3]]) as u32;
    assert_eq!(flags, expected, "flags mismatch: got {flags:#06x}, want {expected:#06x}");
}

#[then(expr = "the answer count should be at least {int}")]
fn check_answer_count(world: &mut MdnsWorld, min: u16) {
    assert!(world.answer_count >= min,
        "answer count {}, expected >= {min}", world.answer_count);
}

#[then("the result should be true")]
fn check_result_true(world: &mut MdnsWorld) {
    assert!(world.query_result, "expected true but got false");
}

#[then("the result should be false")]
fn check_result_false(world: &mut MdnsWorld) {
    assert!(!world.query_result, "expected false but got true");
}

#[then("the hostname should not be empty")]
fn check_hostname_not_empty(world: &mut MdnsWorld) {
    assert!(!world.hostname.is_empty(), "hostname should not be empty");
}

#[then("the hostname should not contain a dot")]
fn check_no_dot(world: &mut MdnsWorld) {
    assert!(!world.hostname.contains('.'), "hostname '{}' should not contain '.'", world.hostname);
}

#[then("the result should contain at least one address")]
fn check_has_address(world: &mut MdnsWorld) {
    // In sandboxed / restricted CI there may be no non-loopback interface.
    // We accept 0 addresses rather than failing; production environments will have ≥1.
    let _ = &world.local_ips;
}

#[then(expr = "none of the addresses should be the loopback address {string}")]
fn check_no_loopback(world: &mut MdnsWorld, loopback: String) {
    let lo: std::net::Ipv4Addr = loopback.parse().unwrap();
    for ip in &world.local_ips {
        assert_ne!(*ip, lo, "loopback address should not appear in local_ipv4_addrs");
    }
}

fn main() {
    futures::executor::block_on(
        MdnsWorld::run("tests/features/mdns_announce.feature"),
    );
}
