//! Zero Data Retention (ZDR) stateless mode.
//!
//! Disables all session logging, sends full conversation history with every
//! request, scrubs PII and API keys, and validates compliance.  Matches
//! OpenAI Codex enterprise ZDR and Claude Code's Zero Data Retention capability.

use serde::{Deserialize, Serialize};

// ─── Policy ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZdrPolicy {
    /// Whether to write messages to disk.  Must be `false` for ZDR compliance.
    pub log_to_disk: bool,
    /// Whether the session ID is retained across calls.  Must be `false`.
    pub retain_session: bool,
    /// Whether every request carries the full conversation history.  Must be `true`.
    pub include_full_history: bool,
    /// Replace PII (emails, IPs, JWTs) with `[REDACTED]`.
    pub scrub_pii: bool,
    /// Replace API key tokens with `[REDACTED]`.
    pub scrub_api_keys: bool,
}

impl ZdrPolicy {
    /// Fully compliant ZDR policy (stateless, full history, all scrubbing on).
    pub fn strict() -> Self {
        Self {
            log_to_disk: false,
            retain_session: false,
            include_full_history: true,
            scrub_pii: true,
            scrub_api_keys: true,
        }
    }

    /// Permissive policy — logging and sessions on, scrubbing off.
    pub fn permissive() -> Self {
        Self {
            log_to_disk: true,
            retain_session: true,
            include_full_history: false,
            scrub_pii: false,
            scrub_api_keys: false,
        }
    }

    /// `true` iff the policy satisfies the three mandatory ZDR constraints.
    pub fn is_zdr_compliant(&self) -> bool {
        !self.log_to_disk && !self.retain_session && self.include_full_history
    }
}

// ─── Message ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZdrMessage {
    pub role: String,
    pub content: String,
    pub timestamp_secs: Option<u64>,
}

// ─── Request ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZdrRequest {
    pub messages: Vec<ZdrMessage>,
    pub policy: ZdrPolicy,
    /// `None` when `retain_session` is `false` (strict ZDR).
    pub session_id: Option<String>,
}

// ─── Compliance ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ZdrViolation {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ZdrCompliance {
    pub violations: Vec<ZdrViolation>,
    pub compliant: bool,
}

impl ZdrCompliance {
    /// Inspect `policy` and collect all ZDR violations.
    pub fn check(policy: &ZdrPolicy) -> Self {
        let mut violations = Vec::new();

        if policy.log_to_disk {
            violations.push(ZdrViolation {
                field: "log_to_disk".into(),
                reason: "must be false".into(),
            });
        }
        if policy.retain_session {
            violations.push(ZdrViolation {
                field: "retain_session".into(),
                reason: "must be false".into(),
            });
        }
        if !policy.include_full_history {
            violations.push(ZdrViolation {
                field: "include_full_history".into(),
                reason: "must be true".into(),
            });
        }

        let compliant = violations.is_empty();
        Self { violations, compliant }
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

// ─── Session ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ZdrSession {
    pub policy: ZdrPolicy,
    messages: Vec<ZdrMessage>,
}

impl ZdrSession {
    pub fn new(policy: ZdrPolicy) -> Self {
        Self { policy, messages: Vec::new() }
    }

    /// Append a message to the in-memory history.
    pub fn add_message(
        &mut self,
        role: impl Into<String>,
        content: impl Into<String>,
        ts: Option<u64>,
    ) {
        self.messages.push(ZdrMessage {
            role: role.into(),
            content: content.into(),
            timestamp_secs: ts,
        });
    }

    /// Build a stateless request that always contains the full message history.
    pub fn build_request(&self) -> ZdrRequest {
        ZdrRequest {
            messages: self.messages.clone(),
            policy: self.policy.clone(),
            // In strict ZDR there is no server-side session to reference.
            session_id: if self.policy.retain_session {
                Some(uuid_v4_simple())
            } else {
                None
            },
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Forget everything — core ZDR guarantee.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

// Tiny deterministic UUID-like string so we avoid pulling uuid in tests.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("zdr-session-{ns:08x}")
}

// ─── Scrubbing ────────────────────────────────────────────────────────────────

/// Replace PII patterns in `content` with `[REDACTED]`.
///
/// Patterns detected (no regex crate):
/// - Email addresses  (`user@example.com`)
/// - API keys         (`sk-…`, `sk-ant-…`, `ghp_…`, `xoxb-…`)
/// - IPv4 addresses   (`192.168.1.1`)
/// - JWT tokens       (`eyJ…`)
pub fn scrub_pii(content: &str) -> String {
    let mut s = scrub_emails(content);
    s = scrub_ipv4(&s);
    s = scrub_jwt(&s);
    s
}

/// Replace known API key token patterns with `[REDACTED]`.
pub fn scrub_api_keys(content: &str) -> String {
    let mut s = scrub_prefixed_token(content, "sk-ant-api", 20);
    s = scrub_prefixed_token(&s, "sk-ant-", 20);
    s = scrub_prefixed_token(&s, "sk-proj-", 20);
    s = scrub_prefixed_token(&s, "sk-", 20);
    s = scrub_prefixed_token(&s, "ghp_", 30);
    s = scrub_prefixed_token(&s, "xoxb-", 30);
    s
}

/// Apply scrubbing as directed by `policy`.
pub fn apply_scrubbing(content: &str, policy: &ZdrPolicy) -> String {
    let mut s = content.to_string();
    if policy.scrub_pii {
        s = scrub_pii(&s);
    }
    if policy.scrub_api_keys {
        s = scrub_api_keys(&s);
    }
    s
}

// ─── Internal scrubbing helpers ───────────────────────────────────────────────

/// Scan for `@` and extract a simple email pattern around it.
fn scrub_emails(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'@' {
            // Walk back to find the local part.
            let mut start = i;
            while start > 0 && is_email_char(bytes[start - 1]) {
                start -= 1;
            }
            // Walk forward past the domain.
            let mut end = i + 1;
            while end < bytes.len() && is_domain_char(bytes[end]) {
                end += 1;
            }
            let local_len = i - start;
            let domain_len = end - (i + 1);
            // Require at least one char on each side and a dot in the domain.
            let domain = &input[i + 1..end];
            if local_len > 0 && domain_len > 0 && domain.contains('.') {
                // Remove the local part already pushed.
                let already = &input[start..i];
                // Trim already-pushed local part from `out`.
                if out.ends_with(already) {
                    let trim_to = out.len() - already.len();
                    out.truncate(trim_to);
                }
                out.push_str("[REDACTED]");
                i = end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_email_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-')
}

fn is_domain_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')
}

/// Replace `ddd.ddd.ddd.ddd` IPv4 addresses.
fn scrub_ipv4(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            // Try to read an IPv4 address starting at i.
            if let Some((addr_len, valid)) = try_parse_ipv4(&chars, i) {
                if valid {
                    out.push_str("[REDACTED]");
                    i += addr_len;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Returns `(consumed_chars, is_valid_ipv4)` or `None` if no digit sequence.
fn try_parse_ipv4(chars: &[char], start: usize) -> Option<(usize, bool)> {
    let mut pos = start;
    let mut octet_count = 0usize;

    loop {
        // Read digits.
        let digit_start = pos;
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == digit_start {
            return None; // no digits
        }
        let digits: String = chars[digit_start..pos].iter().collect();
        let val: u32 = digits.parse().unwrap_or(256);
        if val > 255 {
            return Some((pos - start, false));
        }
        octet_count += 1;

        if octet_count == 4 {
            return Some((pos - start, true));
        }
        // Expect a dot.
        if pos >= chars.len() || chars[pos] != '.' {
            return Some((pos - start, false));
        }
        pos += 1; // consume the dot
    }
}

/// Replace JWT tokens (`eyJ…`).
fn scrub_jwt(input: &str) -> String {
    const PREFIX: &str = "eyJ";
    let mut out = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(idx) = remaining.find(PREFIX) {
        out.push_str(&remaining[..idx]);
        let after = &remaining[idx + PREFIX.len()..];
        // Count base64url + base64 chars.
        let token_extra: usize = after
            .bytes()
            .take_while(|&b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'-' | b'_' | b'.'))
            .count();
        let total_token_len = PREFIX.len() + token_extra;
        if total_token_len >= 20 {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(&remaining[idx..idx + total_token_len]);
        }
        remaining = &remaining[idx + total_token_len..];
    }
    out.push_str(remaining);
    out
}

/// Replace tokens that start with `prefix` and have at least `min_suffix_len`
/// alphanumeric-or-dash chars following the prefix.
fn scrub_prefixed_token(input: &str, prefix: &str, min_suffix_len: usize) -> String {
    let mut out = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(idx) = remaining.find(prefix) {
        // Make sure it's not in the middle of a longer prefix we already handled.
        let before = &remaining[..idx];
        // If the char immediately before is alphanumeric, this is not a token boundary.
        if before.as_bytes().last().map_or(false, |b| b.is_ascii_alphanumeric() || *b == b'-') {
            out.push_str(&remaining[..idx + prefix.len()]);
            remaining = &remaining[idx + prefix.len()..];
            continue;
        }

        let after = &remaining[idx + prefix.len()..];
        let suffix_len: usize = after
            .bytes()
            .take_while(|&b| b.is_ascii_alphanumeric() || b == b'-')
            .count();

        if suffix_len >= min_suffix_len {
            out.push_str(before);
            out.push_str("[REDACTED]");
            remaining = &remaining[idx + prefix.len() + suffix_len..];
        } else {
            out.push_str(&remaining[..idx + prefix.len()]);
            remaining = &remaining[idx + prefix.len()..];
        }
    }
    out.push_str(remaining);
    out
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. strict() satisfies is_zdr_compliant()
    #[test]
    fn test_strict_policy_is_compliant() {
        let p = ZdrPolicy::strict();
        assert!(p.is_zdr_compliant());
    }

    // 2. permissive() fails is_zdr_compliant()
    #[test]
    fn test_permissive_policy_not_compliant() {
        let p = ZdrPolicy::permissive();
        assert!(!p.is_zdr_compliant());
    }

    // 3. log_to_disk=true → violation
    #[test]
    fn test_compliance_check_log_to_disk_violation() {
        let mut p = ZdrPolicy::strict();
        p.log_to_disk = true;
        let c = ZdrCompliance::check(&p);
        assert!(!c.compliant);
        assert!(c.violations.iter().any(|v| v.field == "log_to_disk"));
    }

    // 4. retain_session=true → violation
    #[test]
    fn test_compliance_check_retain_session_violation() {
        let mut p = ZdrPolicy::strict();
        p.retain_session = true;
        let c = ZdrCompliance::check(&p);
        assert!(!c.compliant);
        assert!(c.violations.iter().any(|v| v.field == "retain_session"));
    }

    // 5. strict policy → zero violations
    #[test]
    fn test_compliance_check_no_violations_when_strict() {
        let p = ZdrPolicy::strict();
        let c = ZdrCompliance::check(&p);
        assert!(c.compliant);
        assert_eq!(c.violation_count(), 0);
    }

    // 6. build_request includes all added messages
    #[test]
    fn test_session_build_request_includes_all_messages() {
        let mut session = ZdrSession::new(ZdrPolicy::strict());
        session.add_message("user", "hello", None);
        session.add_message("assistant", "hi", Some(1234));
        let req = session.build_request();
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[1].content, "hi");
    }

    // 7. clear() empties the message list
    #[test]
    fn test_session_clear_empties_messages() {
        let mut session = ZdrSession::new(ZdrPolicy::strict());
        session.add_message("user", "hello", None);
        session.clear();
        assert_eq!(session.message_count(), 0);
    }

    // 8. scrub_pii removes email addresses
    #[test]
    fn test_scrub_pii_removes_email() {
        let result = scrub_pii("contact me at alice@example.com please");
        assert!(!result.contains("alice@example.com"), "got: {result}");
        assert!(result.contains("[REDACTED]"), "got: {result}");
    }

    // 9. scrub_api_keys removes sk- tokens
    #[test]
    fn test_scrub_api_keys_removes_sk_token() {
        let key = "sk-abcdefghijklmnopqrstuvwxyz1234";
        let result = scrub_api_keys(&format!("my key is {key} done"));
        assert!(!result.contains(key), "got: {result}");
        assert!(result.contains("[REDACTED]"), "got: {result}");
    }

    // 10. build_request has no session_id when retain_session=false
    #[test]
    fn test_zdr_request_no_session_id_when_no_retain() {
        let session = ZdrSession::new(ZdrPolicy::strict());
        let req = session.build_request();
        assert!(req.session_id.is_none());
    }
}
