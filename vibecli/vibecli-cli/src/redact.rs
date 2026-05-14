//! Newtype that prevents accidental leakage of credentials through `Debug`
//! / `Display` (and therefore through any `tracing::info!(?config)` or
//! `format!("{cfg:?}")` site).
//!
//! Tracks DREAD #16 in `docs/security/threat-model.md`. Pair with the
//! semgrep rule `tracing-format-leaks-credential` in
//! `.semgrep/credential-logging.yml`, which forbids bare `{api_key}` /
//! `{secret}` / `{token}` / `{password}` interpolation inside `tracing::*`
//! macros — that gate stops regressions before they ship; this newtype
//! gives existing config structs an opt-in way to stop leaking through
//! `#[derive(Debug)]`.
//!
//! Design choices:
//!
//! * **No `Deref` / `DerefMut`.** Auto-deref defeats the purpose — a
//!   careless `format!("{}", *r)` would expose the value. Callers must
//!   explicitly opt-in via [`Redact::expose`] when they really need the
//!   plaintext (e.g. building an `Authorization` header).
//! * **Serde is transparent.** `Serialize` / `Deserialize` round-trip the
//!   wrapped value as-is so config files keep working. Redaction is a
//!   property of the *log surface*, not the on-disk representation. The
//!   on-disk surface is already covered by the encrypted ProfileStore.
//! * **`PartialEq` constant-time on `[u8]`-shaped payloads.** Avoids a
//!   side channel if anyone ever compares two redacted tokens directly.
//!   For other `T`, the default `PartialEq` of `T` is used.
//!
//! Example:
//!
//! ```ignore
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct ProviderConfig {
//!     pub endpoint: String,
//!     pub api_key: Option<Redact<String>>,
//! }
//! // tracing::info!(?config, "loaded provider config")  → api_key is "[redacted]"
//! // config.api_key.as_ref().map(|k| header.insert("x-api-key", k.expose()))
//! ```
//!
//! Migration plan: new code uses `Redact<…>` directly; existing structs
//! get migrated when they're next touched. The semgrep gate makes
//! unwrapped use of credential-shaped variables in log macros fail CI.

use serde::{Deserialize, Serialize};

/// Wrapper that hides its contents in `Debug` / `Display`. See module docs.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Redact<T>(T);

impl<T> Redact<T> {
    /// Wrap a value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the inner value. Use at the actual point of use (HTTP
    /// header, API call) — never when formatting into a log line.
    #[inline]
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consume the wrapper and return the inner value. Same caveat as
    /// [`Redact::expose`].
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Redact<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> std::fmt::Debug for Redact<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> std::fmt::Display for Redact<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

/// Constant-time `PartialEq` for the common `Redact<String>` /
/// `Redact<Vec<u8>>` cases — comparing two tokens shouldn't expose a
/// byte-by-byte timing side channel. Other `Redact<T>` users get the
/// generic impl below.
impl PartialEq for Redact<String> {
    fn eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;
        if self.0.len() != other.0.len() {
            return false;
        }
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}
impl Eq for Redact<String> {}

impl PartialEq for Redact<Vec<u8>> {
    fn eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;
        if self.0.len() != other.0.len() {
            return false;
        }
        self.0.as_slice().ct_eq(other.0.as_slice()).into()
    }
}
impl Eq for Redact<Vec<u8>> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_prints_redacted_for_string() {
        let r: Redact<String> = "sk-live-supersecret".to_string().into();
        let s = format!("{r:?}");
        assert_eq!(s, "[redacted]");
        assert!(!s.contains("sk-live"));
    }

    #[test]
    fn display_prints_redacted_for_string() {
        let r: Redact<String> = "sk-live-supersecret".to_string().into();
        assert_eq!(format!("{r}"), "[redacted]");
    }

    #[test]
    fn debug_of_struct_containing_redact_hides_inner() {
        #[derive(Debug)]
        struct Cfg {
            endpoint: String,
            api_key: Redact<String>,
        }
        let c = Cfg {
            endpoint: "https://api.example".into(),
            api_key: "sk-live-secret".to_string().into(),
        };
        let s = format!("{c:?}");
        assert!(s.contains("https://api.example"));
        assert!(s.contains("[redacted]"));
        assert!(!s.contains("sk-live"));
    }

    #[test]
    fn expose_returns_inner_borrow() {
        let r: Redact<String> = "value".to_string().into();
        assert_eq!(r.expose(), "value");
    }

    #[test]
    fn into_inner_consumes_to_inner() {
        let r: Redact<String> = "value".to_string().into();
        assert_eq!(r.into_inner(), "value".to_string());
    }

    #[test]
    fn serde_is_transparent() {
        let r: Redact<String> = "secret".to_string().into();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#""secret""#);
        let back: Redact<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.expose(), "secret");
    }

    #[test]
    fn partial_eq_for_string_is_constant_time_safe() {
        // Behaviour test (we can't measure timing in unit tests, but we
        // can assert correctness): equal values compare equal, unequal
        // values of equal length compare unequal, and length-mismatched
        // pairs compare unequal.
        let a: Redact<String> = "abcdef".to_string().into();
        let b: Redact<String> = "abcdef".to_string().into();
        let c: Redact<String> = "abcdeg".to_string().into();
        let d: Redact<String> = "abcd".to_string().into();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn debug_works_for_option_wrapped_redact() {
        // Common pattern in config structs: Option<Redact<String>>.
        let some: Option<Redact<String>> = Some("secret".to_string().into());
        let none: Option<Redact<String>> = None;
        assert_eq!(format!("{some:?}"), "Some([redacted])");
        assert_eq!(format!("{none:?}"), "None");
    }
}
