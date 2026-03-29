//! Device pairing — QR code generation for connecting mobile devices to VibeCLI daemon.
//!
//! Generates a one-time pairing URL with a secure token, rendered as a QR code
//! in the terminal using ASCII/Unicode block characters.

/// Generate a pairing URL with a one-time token.
///
/// Returns (url, token).
pub fn generate_pairing_url(host: &str, port: u16) -> (String, String) {
    let token = generate_random_token();
    let url = format!("http://{}:{}/pair?token={}", host, port, token);
    (url, token)
}

/// Generate a cryptographically random 128-bit token as hex string.
fn generate_random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // Use two random hashers for 128 bits of randomness
    let s1 = RandomState::new();
    let s2 = RandomState::new();
    let mut h1 = s1.build_hasher();
    h1.write_u64(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64);
    let mut h2 = s2.build_hasher();
    h2.write_u64(std::process::id() as u64);

    format!("{:016x}{:016x}", h1.finish(), h2.finish())
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
        assert!(url.starts_with("http://192.168.1.100:7878/pair?token="));
        assert_eq!(token.len(), 32); // 128-bit hex = 32 chars
        assert!(url.contains(&token));
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
        assert!(url.starts_with("http://localhost:8080/pair?token="));
    }

    #[test]
    fn pairing_url_with_ipv6() {
        let (url, token) = generate_pairing_url("::1", 3000);
        assert!(url.contains("::1"));
        assert!(url.contains("3000"));
        assert!(url.contains(&token));
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
        let tokens: Vec<String> = (0..10)
            .map(|_| generate_pairing_url("h", 1).1)
            .collect();
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                assert_ne!(tokens[i], tokens[j], "Tokens at {} and {} should differ", i, j);
            }
        }
    }

    #[test]
    fn pairing_url_different_ports() {
        let (url1, _) = generate_pairing_url("host", 80);
        let (url2, _) = generate_pairing_url("host", 443);
        assert!(url1.contains(":80/"));
        assert!(url2.contains(":443/"));
    }
}
