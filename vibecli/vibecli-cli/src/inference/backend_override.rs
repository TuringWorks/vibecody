//! Shared backend-override resolution for HTTP routes that accept an
//! optional backend pin via header or body.
//!
//! Used by every route that dispatches through [`crate::inference::Router`]:
//! `/api/chat`, `/api/generate`, `/v1/messages`, …
//!
//! Precedence matches the daemon-wide rule:
//!   1. `X-VibeCLI-Backend: mistralrs|ollama` request header
//!   2. body field (e.g. `"backend": "mistralrs"`)
//!   3. caller's fallback (per-model pin → daemon default; not handled here)
//!
//! Header beats body on purpose — it lets a thin debug client pin a backend
//! without rewriting the body the upstream service expects.
//!
//! Centralizing here so adding a new [`BackendKind`] variant doesn't drift
//! between `/api/chat` (`inference_routes.rs`) and `/v1/messages`
//! (`v1_messages.rs`).

use axum::http::HeaderMap;

use super::backend::BackendKind;

/// Header name clients use to pin a backend per-request.
pub const HEADER_BACKEND: &str = "x-vibecli-backend";

/// Parse a free-form string into a [`BackendKind`]. Case-insensitive,
/// trims surrounding whitespace. Returns `None` for unknown values so
/// the caller can fall back to the body / pins / default.
pub fn parse_kind(s: &str) -> Option<BackendKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mistralrs" => Some(BackendKind::Mistralrs),
        "ollama" => Some(BackendKind::Ollama),
        _ => None,
    }
}

/// Resolve the backend choice from a header + body pair, with header
/// winning. Falls back to `body_kind` if the header is missing, malformed,
/// or names an unknown backend.
pub fn override_kind(headers: &HeaderMap, body_kind: Option<BackendKind>) -> Option<BackendKind> {
    if let Some(v) = headers.get(HEADER_BACKEND) {
        if let Ok(s) = v.to_str() {
            return parse_kind(s).or(body_kind);
        }
    }
    body_kind
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_recognizes_known_backends() {
        assert_eq!(parse_kind("mistralrs"), Some(BackendKind::Mistralrs));
        assert_eq!(parse_kind("ollama"), Some(BackendKind::Ollama));
        // Case + whitespace tolerance.
        assert_eq!(parse_kind("  Mistralrs "), Some(BackendKind::Mistralrs));
        assert_eq!(parse_kind("OLLAMA"), Some(BackendKind::Ollama));
        assert_eq!(parse_kind(""), None);
        assert_eq!(parse_kind("vllm"), None);
    }

    #[test]
    fn override_kind_header_beats_body() {
        let mut h = HeaderMap::new();
        h.insert(HEADER_BACKEND, "mistralrs".parse().unwrap());
        assert_eq!(
            override_kind(&h, Some(BackendKind::Ollama)),
            Some(BackendKind::Mistralrs),
            "header should override body"
        );
    }

    #[test]
    fn override_kind_falls_back_to_body_when_header_missing() {
        let h = HeaderMap::new();
        assert_eq!(
            override_kind(&h, Some(BackendKind::Ollama)),
            Some(BackendKind::Ollama),
        );
        assert_eq!(override_kind(&h, None), None);
    }

    #[test]
    fn override_kind_falls_back_to_body_on_unknown_header_value() {
        let mut h = HeaderMap::new();
        h.insert(HEADER_BACKEND, "vllm".parse().unwrap());
        assert_eq!(
            override_kind(&h, Some(BackendKind::Mistralrs)),
            Some(BackendKind::Mistralrs),
            "unknown header should not erase body choice"
        );
    }
}
