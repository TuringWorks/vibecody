//! Three-tier backend resolver.
//!
//! Picks `&dyn Backend` for a (model, request) pair via:
//!   1. **request override**  — `body.backend` or `X-VibeCLI-Backend` header
//!   2. **per-model pin**     — `VIBECLI_BACKEND_PINS` env or `[backends]`
//!                              table in `config.toml`
//!   3. **daemon default**    — `VIBECLI_DEFAULT_BACKEND` env, fallback `ollama`
//!
//! The tiers exist so a single deployment can mix-and-match without surgery:
//! a user can route the small TurboQuant-friendly model through mistralrs
//! while keeping their existing 70B llama on the ollama daemon they already
//! manage. Per-request override is the escape hatch for ad-hoc routing
//! (load testing, A/B comparisons, debugging a specific backend).
//!
//! ## Pin pattern syntax
//!
//! Each pin is `pattern=backend`. A `pattern` is one of:
//! - exact:   `qwen2.5:0.5b`
//! - prefix:  `Qwen/*`        (matches `Qwen/Qwen2.5-0.5B-Instruct`)
//! - suffix:  `*-Instruct`
//!
//! Pins are scanned in declaration order; first match wins. Use the most
//! specific pattern first.

use std::sync::Arc;

use super::backend::{Backend, BackendKind};
use super::mistralrs::MistralRsBackend;
use super::ollama::OllamaProxyBackend;

/// One pattern → backend mapping. See module docs for syntax.
#[derive(Debug, Clone)]
pub struct Pin {
    pattern: String,
    backend: BackendKind,
}

impl Pin {
    pub fn new(pattern: impl Into<String>, backend: BackendKind) -> Self {
        Self {
            pattern: pattern.into(),
            backend,
        }
    }

    fn matches(&self, model: &str) -> bool {
        let p = &self.pattern;
        match (p.starts_with('*'), p.ends_with('*')) {
            (true, true) if p.len() >= 2 => {
                let middle = &p[1..p.len() - 1];
                model.contains(middle)
            }
            (true, false) => model.ends_with(&p[1..]),
            (false, true) => model.starts_with(&p[..p.len() - 1]),
            _ => model == p,
        }
    }
}

/// Holds one instance of each backend (as trait objects) and resolves
/// requests to the right one. Cheap to clone; stored on the daemon's
/// `ServeState`.
#[derive(Clone)]
pub struct Router {
    mistralrs: Arc<dyn Backend>,
    ollama: Arc<dyn Backend>,
    default_kind: BackendKind,
    pins: Vec<Pin>,
}

impl Router {
    pub fn new(default_kind: BackendKind) -> Self {
        let mistralrs: Arc<dyn Backend> = Arc::new(MistralRsBackend::new());
        let ollama: Arc<dyn Backend> = Arc::new(OllamaProxyBackend::local());
        Self {
            mistralrs,
            ollama,
            default_kind,
            pins: Vec::new(),
        }
    }

    /// Build from environment:
    /// - `VIBECLI_DEFAULT_BACKEND=mistralrs|ollama` (default `ollama`)
    /// - `VIBECLI_BACKEND_PINS="Qwen/*=mistralrs,llama3:*=ollama"`
    pub fn from_env() -> Self {
        let default_kind = match std::env::var("VIBECLI_DEFAULT_BACKEND")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "mistralrs" => BackendKind::Mistralrs,
            _ => BackendKind::Ollama,
        };
        let mut router = Self::new(default_kind);
        if let Ok(spec) = std::env::var("VIBECLI_BACKEND_PINS") {
            router.pins = parse_pins(&spec);
        }
        router
    }

    /// Replace the pin list. Useful for tests and for loading from
    /// `config.toml` once that surface is wired.
    pub fn with_pins(mut self, pins: Vec<Pin>) -> Self {
        self.pins = pins;
        self
    }

    /// Return the backend instance for a given [`BackendKind`].
    pub fn by_kind(&self, kind: BackendKind) -> Arc<dyn Backend> {
        match kind {
            BackendKind::Mistralrs => Arc::clone(&self.mistralrs),
            BackendKind::Ollama => Arc::clone(&self.ollama),
        }
    }

    /// Resolve the backend for a chat/generate request following the 3-tier
    /// precedence in the module docs.
    pub fn resolve(&self, model: &str, override_kind: Option<BackendKind>) -> Arc<dyn Backend> {
        if let Some(k) = override_kind {
            return self.by_kind(k);
        }
        if let Some(pin) = self.pins.iter().find(|p| p.matches(model)) {
            return self.by_kind(pin.backend);
        }
        self.by_kind(self.default_kind)
    }

    pub fn default_kind(&self) -> BackendKind {
        self.default_kind
    }

    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

/// Parse `"pat=backend,pat2=backend2"`. Silently skips malformed entries —
/// startup shouldn't fail on a typo in an env var, but the daemon does log
/// each accepted pin so users can verify what landed.
fn parse_pins(spec: &str) -> Vec<Pin> {
    let mut out = Vec::new();
    for entry in spec.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some((pat, backend)) = entry.split_once('=') else {
            tracing::warn!("VIBECLI_BACKEND_PINS: skipping malformed entry `{entry}`");
            continue;
        };
        let kind = match backend.trim().to_ascii_lowercase().as_str() {
            "mistralrs" => BackendKind::Mistralrs,
            "ollama" => BackendKind::Ollama,
            other => {
                tracing::warn!(
                    "VIBECLI_BACKEND_PINS: unknown backend `{other}` in `{entry}`, skipping"
                );
                continue;
            }
        };
        let pin = Pin::new(pat.trim(), kind);
        tracing::info!(
            "VIBECLI_BACKEND_PINS: pinned `{}` → {}",
            pin.pattern,
            kind.as_str()
        );
        out.push(pin);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_match_exact() {
        assert!(Pin::new("qwen", BackendKind::Mistralrs).matches("qwen"));
        assert!(!Pin::new("qwen", BackendKind::Mistralrs).matches("qwen2"));
    }

    #[test]
    fn pin_match_prefix() {
        let p = Pin::new("Qwen/*", BackendKind::Mistralrs);
        assert!(p.matches("Qwen/Qwen2.5-0.5B-Instruct"));
        assert!(!p.matches("Mistral/foo"));
    }

    #[test]
    fn pin_match_suffix() {
        let p = Pin::new("*-Instruct", BackendKind::Mistralrs);
        assert!(p.matches("Qwen/Qwen2.5-0.5B-Instruct"));
        assert!(!p.matches("Qwen/Qwen2.5-0.5B"));
    }

    #[test]
    fn parse_pins_skips_bad_entries() {
        let pins = parse_pins("Qwen/*=mistralrs,nope,llama=ollama,foo=bogus");
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].pattern, "Qwen/*");
        assert!(matches!(pins[0].backend, BackendKind::Mistralrs));
        assert_eq!(pins[1].pattern, "llama");
        assert!(matches!(pins[1].backend, BackendKind::Ollama));
    }

    #[test]
    fn router_resolve_precedence() {
        let r = Router::new(BackendKind::Ollama).with_pins(vec![Pin::new(
            "Qwen/*",
            BackendKind::Mistralrs,
        )]);
        // Pin matches.
        assert_eq!(r.resolve("Qwen/foo", None).kind(), BackendKind::Mistralrs);
        // Pin doesn't match → default.
        assert_eq!(r.resolve("llama3", None).kind(), BackendKind::Ollama);
        // Override beats pin.
        assert_eq!(
            r.resolve("Qwen/foo", Some(BackendKind::Ollama)).kind(),
            BackendKind::Ollama
        );
    }
}
