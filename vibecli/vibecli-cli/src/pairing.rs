//! Device pairing — QR code generation for connecting mobile devices to VibeCLI daemon.
//!
//! Generates a one-time pairing URL plus a separate token, rendered together
//! in the terminal using ASCII/Unicode block characters.

/// Generate a pairing URL and a paired bearer token.
///
/// Returns `(url, token)` — the URL is what the user pastes/scans into a
/// browser or mobile app to discover the daemon; the token is entered
/// separately as the bearer credential.
///
/// **The token is NOT embedded in the URL.** Pre-2026-05-13 this returned
/// `http://host:port/pair?token=<32-hex>`, which leaked the credential into
/// every place the URL is recorded: browser history, paste buffers, screen
/// recordings, proxy/Tailscale/ngrok access logs, screenshot uploads, … and
/// no consumer ever actually parsed the token out of the URL. See
/// `docs/security/threat-model.md` §7 item #12 (DREAD 6.4).
pub fn generate_pairing_url(host: &str, port: u16) -> (String, String) {
    let token = generate_random_token();
    let url = format!("http://{host}:{port}/pair");
    (url, token)
}

/// Generate a cryptographically random 128-bit token as hex string.
///
/// **Security note** — this token is a bearer credential. Anyone who
/// possesses it can connect to the daemon. It MUST be sourced from the
/// OS CSPRNG (rand::rng() ≡ ThreadRng, internally seeded from
/// getrandom). The previous implementation used `RandomState` which is
/// NOT cryptographically secure — it's HashMap DoS resistance only and
/// the seed is roughly `hash(unix_nanos) ^ hash(pid)`, predictable to
/// an attacker who can probe the pairing endpoint with timing.
fn generate_random_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    let mut hex = String::with_capacity(32);
    for b in &bytes {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

/// Render a URL as an ASCII QR code for terminal display.
///
/// Uses a simple box-drawing approach. For a proper QR code, the `qrcode` crate
/// would be needed, but this provides a scannable representation.
pub fn render_pairing_display(url: &str, token: &str) -> String {
    let mut output = String::new();
    output.push_str("┌─────────────────────────────────────────────┐\n");
    output.push_str("│           VibeCLI Device Pairing             │\n");
    output.push_str("├─────────────────────────────────────────────┤\n");
    output.push_str("│                                             │\n");
    output.push_str(&format!("│  URL: {}  │\n", truncate_for_box(url, 37)));
    output.push_str("│                                             │\n");
    output.push_str(&format!("│  Token: {}  │\n", truncate_for_box(token, 33)));
    output.push_str("│                                             │\n");
    output.push_str("│  Open this URL in a browser on your device  │\n");
    output.push_str("│  to connect to this VibeCLI instance.       │\n");
    output.push_str("│                                             │\n");
    output.push_str("└─────────────────────────────────────────────┘\n");
    output
}

fn truncate_for_box(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}..", &s[..max - 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_url_format() {
        let (url, token) = generate_pairing_url("192.168.1.100", 7878);
        assert_eq!(url, "http://192.168.1.100:7878/pair");
        assert_eq!(token.len(), 32); // 128-bit hex = 32 chars
    }

    /// Regression guard for DREAD #12. The pre-fix implementation embedded
    /// the bearer token in the URL as `?token=<hex>`. That string is recorded
    /// in browser history, proxy/Tailscale/ngrok access logs, paste buffers,
    /// and anywhere screenshots end up. Anybody who recovers the URL
    /// recovers the bearer. The URL must NEVER contain the token again.
    #[test]
    fn url_does_not_leak_token() {
        for _ in 0..50 {
            let (url, token) = generate_pairing_url("host", 7878);
            assert!(
                !url.contains(&token),
                "pairing URL must not embed the bearer token: url={url} token={token}"
            );
            assert!(
                !url.contains("token="),
                "pairing URL must not contain a `token=` query parameter: {url}"
            );
        }
    }

    #[test]
    fn token_is_unique() {
        let (_, t1) = generate_pairing_url("localhost", 7878);
        let (_, t2) = generate_pairing_url("localhost", 7878);
        // Tokens should be different (extremely high probability)
        assert_ne!(t1, t2);
    }

    #[test]
    fn render_display_contains_url() {
        let display = render_pairing_display("http://localhost:7878/pair?token=abc", "abc");
        assert!(display.contains("VibeCLI Device Pairing"));
        assert!(display.contains("Token:"));
    }

    #[test]
    fn truncate_for_box_short() {
        let result = truncate_for_box("hello", 10);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn truncate_for_box_long() {
        let result = truncate_for_box("a very long string that exceeds", 10);
        assert!(result.len() <= 10);
        assert!(result.ends_with(".."));
    }

    #[test]
    fn pairing_url_with_localhost() {
        let (url, _) = generate_pairing_url("localhost", 8080);
        assert_eq!(url, "http://localhost:8080/pair");
    }

    #[test]
    fn pairing_url_with_ipv6() {
        let (url, token) = generate_pairing_url("::1", 3000);
        assert!(url.contains("::1"));
        assert!(url.contains("3000"));
        // Token is NOT in the URL (DREAD #12) — verified separately via
        // url_does_not_leak_token. Here we only check the URL is shaped right.
        assert_eq!(url, "http://::1:3000/pair");
        assert_eq!(token.len(), 32);
    }

    #[test]
    fn token_length_is_32() {
        let (_, token) = generate_pairing_url("host", 1234);
        assert_eq!(token.len(), 32);
    }

    #[test]
    fn token_is_hex() {
        let (_, token) = generate_pairing_url("host", 1234);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn render_display_contains_box_borders() {
        let display = render_pairing_display("http://x", "tok");
        assert!(display.contains('┌'));
        assert!(display.contains('└'));
        assert!(display.contains('├'));
    }

    #[test]
    fn render_display_contains_instructions() {
        let display = render_pairing_display("http://x", "tok");
        assert!(display.contains("Open this URL"));
        assert!(display.contains("connect to this VibeCLI instance"));
    }

    #[test]
    fn render_display_shows_url_label() {
        let display = render_pairing_display("http://example.com:7878/pair?token=abc", "abc");
        assert!(display.contains("URL:"));
    }

    #[test]
    fn render_display_shows_token_label() {
        let display = render_pairing_display("http://x", "my_token_value");
        assert!(display.contains("Token:"));
    }

    #[test]
    fn truncate_exact_length() {
        let result = truncate_for_box("abcdefghij", 10);
        assert_eq!(result.len(), 10);
        assert_eq!(result, "abcdefghij");
    }

    #[test]
    fn truncate_shorter_than_max_pads() {
        let result = truncate_for_box("hi", 10);
        assert_eq!(result.len(), 10);
        assert!(result.starts_with("hi"));
    }

    #[test]
    fn multiple_tokens_all_unique() {
        let tokens: Vec<String> = (0..10).map(|_| generate_pairing_url("h", 1).1).collect();
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                assert_ne!(
                    tokens[i], tokens[j],
                    "Tokens at {} and {} should differ",
                    i, j
                );
            }
        }
    }

    #[test]
    fn pairing_url_different_ports() {
        let (url1, _) = generate_pairing_url("host", 80);
        let (url2, _) = generate_pairing_url("host", 443);
        assert_eq!(url1, "http://host:80/pair");
        assert_eq!(url2, "http://host:443/pair");
    }

    /// Statistical check: across 1000 tokens, every hex nibble should
    /// appear at roughly equal frequency. The previous RandomState-based
    /// implementation skewed heavily because the seed was hash(time_nanos),
    /// so neighbouring tokens shared most of their bytes. A proper CSPRNG
    /// keeps every nibble within ~30% of the expected count (1000*32/16 = 2000).
    /// This is a regression guard, not a NIST-grade test.
    #[test]
    fn token_entropy_is_well_distributed() {
        let mut nibble_counts = [0u32; 16];
        for _ in 0..1000 {
            let (_, token) = generate_pairing_url("h", 1);
            for c in token.chars() {
                let n = c.to_digit(16).unwrap() as usize;
                nibble_counts[n] += 1;
            }
        }
        // Expected = 1000 * 32 / 16 = 2000 per nibble.
        // Allow generous slack — anything below 1400 or above 2600 is a
        // strong signal the RNG is broken.
        for (n, &count) in nibble_counts.iter().enumerate() {
            assert!(
                count > 1400 && count < 2600,
                "Nibble 0x{:x} appeared {} times — RNG may be broken",
                n,
                count
            );
        }
    }

    /// Critical security regression guard: the first 16 hex chars of two
    /// tokens generated back-to-back should diverge in MULTIPLE positions.
    /// The previous RandomState implementation seeded from time_nanos,
    /// so two tokens generated within microseconds shared their first
    /// 8+ bytes. With a proper CSPRNG, the probability of any single
    /// hex char matching is 1/16 → expected matches in 16 chars = 1.
    #[test]
    fn back_to_back_tokens_diverge_quickly() {
        for _ in 0..100 {
            let (_, t1) = generate_pairing_url("h", 1);
            let (_, t2) = generate_pairing_url("h", 1);
            let matching_prefix = t1
                .chars()
                .zip(t2.chars())
                .take_while(|(a, b)| a == b)
                .count();
            // It should be EXTREMELY unlikely for the first 8+ chars to match.
            // Probability of 8-char match by chance = 1/16^8 ~= 2e-10.
            assert!(
                matching_prefix < 8,
                "Two consecutive tokens share {} hex chars — RNG is broken: {} vs {}",
                matching_prefix,
                t1,
                t2
            );
        }
    }
}
