//! Constant-time comparison helpers for bearer-token / device-token auth.
//!
//! `==` on `String`/`&str` short-circuits at the first mismatched byte, which
//! gives a remote attacker a side channel for guessing the token one byte at
//! a time. The risk is mostly theoretical for our 128-bit hex tokens (entropy
//! makes the attack impractical), but the fix is one line and we want
//! defense-in-depth on the daemon's primary auth gate. Tracked as DREAD #6 in
//! `docs/security/threat-model.md`.
//!
//! All callers in `serve.rs` and `watch_bridge.rs` route through these helpers.
//! Do not write `received == format!("Bearer {token}")` directly.

use subtle::ConstantTimeEq;

/// Constant-time comparison of two byte slices. Returns `false` if lengths
/// differ — the length leak is benign for fixed-format tokens.
#[inline]
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// Returns true iff `header_value` is exactly `Bearer <api_token>`, compared
/// in constant time. Pass the raw `Authorization` header value (already
/// stringified with `to_str().ok()`); `None` and the empty string both yield
/// false.
#[inline]
pub fn bearer_matches(header_value: Option<&str>, api_token: &str) -> bool {
    let received = header_value.unwrap_or("");
    // Pre-check the literal prefix in variable time — the prefix is public,
    // not secret, so no side channel on it. Then constant-time compare the
    // token portion against the expected token.
    let Some(token) = received.strip_prefix("Bearer ") else {
        return false;
    };
    ct_eq(token.as_bytes(), api_token.as_bytes())
}

/// Constant-time equality of a received raw token against the configured
/// `api_token`. Use when the token comes from a query parameter or a
/// non-`Authorization` header (e.g. `?token=...` on a WebSocket upgrade).
#[inline]
pub fn token_matches(received: &str, api_token: &str) -> bool {
    ct_eq(received.as_bytes(), api_token.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "fbbbecf637b2c4a4f0c0b08d0e2d3a1e";

    #[test]
    fn bearer_matches_accepts_correct_token() {
        assert!(bearer_matches(Some(&format!("Bearer {TOKEN}")), TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_wrong_token() {
        assert!(!bearer_matches(Some("Bearer wrong"), TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_missing_header() {
        assert!(!bearer_matches(None, TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_empty_header() {
        assert!(!bearer_matches(Some(""), TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_missing_prefix() {
        assert!(!bearer_matches(Some(TOKEN), TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_wrong_prefix_case() {
        // RFC 6750 says the scheme is case-insensitive, but our daemon has
        // always required the exact "Bearer " prefix. Document the choice.
        assert!(!bearer_matches(Some(&format!("bearer {TOKEN}")), TOKEN));
    }

    #[test]
    fn bearer_matches_rejects_extra_suffix() {
        assert!(!bearer_matches(
            Some(&format!("Bearer {TOKEN}extra")),
            TOKEN
        ));
    }

    #[test]
    fn bearer_matches_rejects_truncated_token() {
        let truncated = &TOKEN[..TOKEN.len() - 1];
        assert!(!bearer_matches(Some(&format!("Bearer {truncated}")), TOKEN));
    }

    #[test]
    fn token_matches_accepts_correct() {
        assert!(token_matches(TOKEN, TOKEN));
    }

    #[test]
    fn token_matches_rejects_wrong() {
        assert!(!token_matches("nope", TOKEN));
    }

    #[test]
    fn token_matches_rejects_length_mismatch() {
        assert!(!token_matches(&TOKEN[..16], TOKEN));
    }
}
