//! OAuth login — subscription-based auth without API keys.
//! Pi-mono gap bridge: Phase A3.
//!
//! Supported providers: Anthropic Claude Pro/Max, GitHub Copilot,
//! Google Gemini CLI, and OpenAI Codex (ChatGPT Plus/Pro).
//!
//! Credentials are persisted via ProfileStore. Dynamic refresh callbacks
//! allow callers to obtain a fresh token on each request without re-running
//! the full OAuth flow.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Provider enum
// ---------------------------------------------------------------------------

/// Supported OAuth providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OAuthProvider {
    /// Claude Pro / Max subscription (Anthropic).
    AnthropicClaude,
    /// GitHub Copilot subscription.
    GitHubCopilot,
    /// Google Gemini CLI free tier.
    GoogleGeminiCli,
    /// ChatGPT Plus / Pro (OpenAI Codex).
    OpenAICodex,
}

impl OAuthProvider {
    /// Human-readable provider name.
    pub fn name(&self) -> &str {
        match self {
            OAuthProvider::AnthropicClaude => "Anthropic Claude",
            OAuthProvider::GitHubCopilot => "GitHub Copilot",
            OAuthProvider::GoogleGeminiCli => "Google Gemini CLI",
            OAuthProvider::OpenAICodex => "OpenAI Codex",
        }
    }

    /// OAuth authorization endpoint URL.
    pub fn auth_url(&self) -> &str {
        match self {
            OAuthProvider::AnthropicClaude => {
                "https://claude.ai/oauth/authorize"
            }
            OAuthProvider::GitHubCopilot => {
                "https://github.com/login/oauth/authorize"
            }
            OAuthProvider::GoogleGeminiCli => {
                "https://accounts.google.com/o/oauth2/v2/auth"
            }
            OAuthProvider::OpenAICodex => {
                "https://auth.openai.com/authorize"
            }
        }
    }

    /// Token exchange endpoint URL.
    pub fn token_url(&self) -> &str {
        match self {
            OAuthProvider::AnthropicClaude => {
                "https://claude.ai/oauth/token"
            }
            OAuthProvider::GitHubCopilot => {
                "https://github.com/login/oauth/access_token"
            }
            OAuthProvider::GoogleGeminiCli => {
                "https://oauth2.googleapis.com/token"
            }
            OAuthProvider::OpenAICodex => {
                "https://auth.openai.com/oauth/token"
            }
        }
    }

    /// Device authorization endpoint, if the provider supports the device flow.
    pub fn device_code_url(&self) -> Option<&str> {
        match self {
            OAuthProvider::AnthropicClaude => {
                Some("https://claude.ai/oauth/device/code")
            }
            OAuthProvider::GitHubCopilot => {
                Some("https://github.com/login/device/code")
            }
            OAuthProvider::GoogleGeminiCli => {
                Some("https://oauth2.googleapis.com/device/code")
            }
            OAuthProvider::OpenAICodex => None,
        }
    }

    /// Prompt-cache retention in seconds.
    ///
    /// - Anthropic providers: 1 hour (3600 s)
    /// - OpenAI providers:    24 hours (86400 s)
    /// - Others:              default 1 hour
    pub fn cache_retention_seconds(&self) -> u64 {
        match self {
            OAuthProvider::AnthropicClaude => 3_600,
            OAuthProvider::GitHubCopilot => 3_600,
            OAuthProvider::GoogleGeminiCli => 3_600,
            OAuthProvider::OpenAICodex => 86_400,
        }
    }

    /// Whether this provider uses the RFC 8628 device authorization flow.
    pub fn uses_device_flow(&self) -> bool {
        self.device_code_url().is_some()
    }

    /// Parse a provider from a case-insensitive string slug.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "anthropic" | "anthropic_claude" | "claude" => {
                Some(OAuthProvider::AnthropicClaude)
            }
            "github" | "github_copilot" | "copilot" => {
                Some(OAuthProvider::GitHubCopilot)
            }
            "google" | "google_gemini_cli" | "gemini" | "gemini_cli" => {
                Some(OAuthProvider::GoogleGeminiCli)
            }
            "openai" | "openai_codex" | "codex" | "chatgpt" => {
                Some(OAuthProvider::OpenAICodex)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// Persisted OAuth credentials (intended for storage in ProfileStore).
#[derive(Debug, Clone)]
pub struct OAuthCredentials {
    pub provider: OAuthProvider,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Absolute expiry as milliseconds since UNIX epoch, or `None` if the
    /// token never expires (e.g. long-lived personal access tokens).
    pub expires_at_ms: Option<u64>,
    pub scopes: Vec<String>,
    pub account_email: Option<String>,
}

impl OAuthCredentials {
    /// Returns `true` if the token has already passed its expiry time.
    pub fn is_expired(&self) -> bool {
        match self.expires_at_ms {
            None => false,
            Some(exp) => {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                now_ms >= exp
            }
        }
    }

    /// Returns `true` if the token will expire within the next 5 minutes
    /// (300 000 ms) — callers should proactively refresh in this window.
    pub fn needs_refresh(&self) -> bool {
        match self.expires_at_ms {
            None => false,
            Some(exp) => {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let five_min_ms: u64 = 5 * 60 * 1_000;
                now_ms + five_min_ms >= exp
            }
        }
    }

    /// Returns the access token only if it is still valid (not expired).
    pub fn access_token_if_valid(&self) -> Option<&str> {
        if self.is_expired() {
            None
        } else {
            Some(&self.access_token)
        }
    }

    /// Returns a redacted representation suitable for logging:
    /// `"Bearer ****...{last4}"`.
    pub fn redacted(&self) -> String {
        let token = &self.access_token;
        let last4: String = token.chars().rev().take(4).collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("Bearer ****...{}", last4)
    }
}

// ---------------------------------------------------------------------------
// Callbacks trait
// ---------------------------------------------------------------------------

/// Callbacks invoked during the OAuth flow for UI/TUI integration.
pub trait OAuthLoginCallbacks: Send + Sync {
    /// Called when a device code is obtained; the user must visit
    /// `verification_url` and enter `user_code` within `expires_in_s` seconds.
    fn on_device_code(
        &self,
        user_code: &str,
        verification_url: &str,
        expires_in_s: u64,
    );

    /// Called each polling iteration while waiting for the user to authorize.
    fn on_polling(&self, attempt: u32);

    /// Called on successful authorization.  `email` may be `None` if the
    /// provider does not return account identity in the token response.
    fn on_success(&self, email: Option<&str>);

    /// Called when the flow terminates with an error or user cancellation.
    fn on_error(&self, reason: &str);
}

/// No-op callbacks — useful in tests where side-effects are not needed.
pub struct NoopCallbacks;

impl OAuthLoginCallbacks for NoopCallbacks {
    fn on_device_code(&self, _user_code: &str, _url: &str, _expires_in_s: u64) {}
    fn on_polling(&self, _attempt: u32) {}
    fn on_success(&self, _email: Option<&str>) {}
    fn on_error(&self, _reason: &str) {}
}

// ---------------------------------------------------------------------------
// Flow result
// ---------------------------------------------------------------------------

/// The outcome of an OAuth flow execution.
#[derive(Debug)]
pub enum OAuthFlowResult {
    /// The flow completed successfully; credentials are ready.
    Success(OAuthCredentials),
    /// The user explicitly cancelled the flow.
    Cancelled,
    /// The flow failed with a human-readable error message.
    Error(String),
}

// ---------------------------------------------------------------------------
// OAuthManager
// ---------------------------------------------------------------------------

/// Manages OAuth flows and credential storage for all supported providers.
#[derive(Debug)]
pub struct OAuthManager {
    store: HashMap<OAuthProvider, OAuthCredentials>,
}

impl OAuthManager {
    /// Create a new, empty manager.
    pub fn new() -> Self {
        OAuthManager {
            store: HashMap::new(),
        }
    }

    /// Persist credentials for a provider, replacing any previous entry.
    pub fn store_credentials(&mut self, creds: OAuthCredentials) {
        self.store.insert(creds.provider.clone(), creds);
    }

    /// Retrieve credentials for a provider, if they exist.
    pub fn get_credentials(
        &self,
        provider: &OAuthProvider,
    ) -> Option<&OAuthCredentials> {
        self.store.get(provider)
    }

    /// Remove credentials for a provider.
    ///
    /// Returns `true` if credentials were present and removed.
    pub fn remove_credentials(&mut self, provider: &OAuthProvider) -> bool {
        self.store.remove(provider).is_some()
    }

    /// Returns `true` if valid (non-expired) credentials exist for `provider`.
    pub fn is_logged_in(&self, provider: &OAuthProvider) -> bool {
        self.store
            .get(provider)
            .map(|c| !c.is_expired())
            .unwrap_or(false)
    }

    /// Returns the raw access token string if the stored token is still valid.
    pub fn valid_token(&self, provider: &OAuthProvider) -> Option<&str> {
        self.store
            .get(provider)
            .and_then(|c| c.access_token_if_valid())
    }

    /// Returns a list of providers for which valid credentials are stored.
    pub fn list_logged_in(&self) -> Vec<&OAuthProvider> {
        self.store
            .iter()
            .filter(|(_, c)| !c.is_expired())
            .map(|(p, _)| p)
            .collect()
    }

    /// Simulate a device-code flow for testing purposes.
    ///
    /// This method does **not** perform any real HTTP requests.  It fires the
    /// appropriate callbacks, constructs synthetic credentials using
    /// `mock_token`, stores them, and returns `OAuthFlowResult::Success`.
    pub fn simulate_device_flow(
        &mut self,
        provider: OAuthProvider,
        mock_token: &str,
        callbacks: &dyn OAuthLoginCallbacks,
    ) -> OAuthFlowResult {
        // Simulate: device code issued
        callbacks.on_device_code(
            "VIBE-1234",
            "https://example.com/activate",
            300,
        );

        // Simulate: two polling attempts
        callbacks.on_polling(1);
        callbacks.on_polling(2);

        // Build synthetic credentials that expire 1 hour from now
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let expires_at_ms = now_ms + 3_600_000; // +1 h

        let email = Some(format!(
            "test-user@{}.example.com",
            provider.name().to_ascii_lowercase().replace(' ', "-")
        ));

        let creds = OAuthCredentials {
            provider: provider.clone(),
            access_token: mock_token.to_string(),
            refresh_token: Some(format!("refresh_{}", mock_token)),
            expires_at_ms: Some(expires_at_ms),
            scopes: vec!["read".to_string(), "write".to_string()],
            account_email: email.clone(),
        };

        callbacks.on_success(email.as_deref());
        self.store_credentials(creds.clone());
        OAuthFlowResult::Success(creds)
    }

    /// Build an `Authorization` header value for the given provider.
    ///
    /// - If a valid OAuth token is stored, returns `"Bearer <token>"`.
    /// - Otherwise, if `fallback_api_key` is supplied, returns
    ///   `"Bearer <api_key>"` (or the key as-is if it already starts with
    ///   `"Bearer "`).
    /// - Returns `None` if neither is available.
    pub fn auth_header(
        &self,
        provider: &OAuthProvider,
        fallback_api_key: Option<&str>,
    ) -> Option<String> {
        if let Some(token) = self.valid_token(provider) {
            return Some(format!("Bearer {}", token));
        }
        fallback_api_key.map(|key| {
            if key.starts_with("Bearer ") {
                key.to_string()
            } else {
                format!("Bearer {}", key)
            }
        })
    }
}

impl Default for OAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Cache retention config
// ---------------------------------------------------------------------------

/// Extended prompt-cache retention configuration for a provider.
#[derive(Debug, Clone)]
pub struct CacheRetentionConfig {
    pub provider: OAuthProvider,
    pub retention_seconds: u64,
    pub enabled: bool,
}

impl CacheRetentionConfig {
    /// Build the default cache retention config for `p`.
    pub fn for_provider(p: &OAuthProvider) -> Self {
        CacheRetentionConfig {
            provider: p.clone(),
            retention_seconds: p.cache_retention_seconds(),
            enabled: true,
        }
    }

    /// Returns `true` when retention is longer than 1 hour (3600 s).
    pub fn is_long_retention(&self) -> bool {
        self.retention_seconds > 3_600
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn fresh_creds(provider: OAuthProvider) -> OAuthCredentials {
        OAuthCredentials {
            provider,
            access_token: "tok_abcdefgh1234".to_string(),
            refresh_token: Some("refresh_xyz".to_string()),
            expires_at_ms: Some(now_ms() + 3_600_000), // 1 h from now
            scopes: vec!["read".to_string()],
            account_email: Some("user@example.com".to_string()),
        }
    }

    fn expired_creds(provider: OAuthProvider) -> OAuthCredentials {
        OAuthCredentials {
            provider,
            access_token: "tok_expired".to_string(),
            refresh_token: None,
            expires_at_ms: Some(now_ms() - 1_000), // 1 s ago
            scopes: vec![],
            account_email: None,
        }
    }

    // ------------------------------------------------------------------
    // is_expired
    // ------------------------------------------------------------------

    #[test]
    fn test_fresh_not_expired() {
        let c = fresh_creds(OAuthProvider::AnthropicClaude);
        assert!(!c.is_expired());
    }

    #[test]
    fn test_past_expiry_is_expired() {
        let c = expired_creds(OAuthProvider::AnthropicClaude);
        assert!(c.is_expired());
    }

    #[test]
    fn test_no_expiry_never_expired() {
        let c = OAuthCredentials {
            provider: OAuthProvider::GitHubCopilot,
            access_token: "tok_permanent".to_string(),
            refresh_token: None,
            expires_at_ms: None,
            scopes: vec![],
            account_email: None,
        };
        assert!(!c.is_expired());
    }

    // ------------------------------------------------------------------
    // needs_refresh
    // ------------------------------------------------------------------

    #[test]
    fn test_needs_refresh_when_nearly_expired() {
        // Expires in 2 minutes — within the 5-minute refresh window.
        let c = OAuthCredentials {
            provider: OAuthProvider::AnthropicClaude,
            access_token: "tok_soon".to_string(),
            refresh_token: Some("r".to_string()),
            expires_at_ms: Some(now_ms() + 2 * 60 * 1_000),
            scopes: vec![],
            account_email: None,
        };
        assert!(c.needs_refresh());
    }

    #[test]
    fn test_no_refresh_needed_when_plenty_of_time() {
        let c = fresh_creds(OAuthProvider::GoogleGeminiCli);
        assert!(!c.needs_refresh());
    }

    // ------------------------------------------------------------------
    // access_token_if_valid
    // ------------------------------------------------------------------

    #[test]
    fn test_valid_token_returned_for_fresh_creds() {
        let c = fresh_creds(OAuthProvider::AnthropicClaude);
        assert_eq!(c.access_token_if_valid(), Some("tok_abcdefgh1234"));
    }

    #[test]
    fn test_no_valid_token_for_expired_creds() {
        let c = expired_creds(OAuthProvider::OpenAICodex);
        assert!(c.access_token_if_valid().is_none());
    }

    // ------------------------------------------------------------------
    // redacted
    // ------------------------------------------------------------------

    #[test]
    fn test_redacted_shows_last_four() {
        let c = fresh_creds(OAuthProvider::AnthropicClaude);
        let r = c.redacted();
        assert!(r.starts_with("Bearer ****..."));
        assert!(r.ends_with("1234"));
    }

    // ------------------------------------------------------------------
    // OAuthManager — list_logged_in
    // ------------------------------------------------------------------

    #[test]
    fn test_list_logged_in_excludes_expired() {
        let mut mgr = OAuthManager::new();
        mgr.store_credentials(fresh_creds(OAuthProvider::AnthropicClaude));
        mgr.store_credentials(expired_creds(OAuthProvider::GitHubCopilot));
        mgr.store_credentials(fresh_creds(OAuthProvider::GoogleGeminiCli));

        let logged = mgr.list_logged_in();
        assert_eq!(logged.len(), 2);
        assert!(logged.contains(&&OAuthProvider::AnthropicClaude));
        assert!(logged.contains(&&OAuthProvider::GoogleGeminiCli));
        assert!(!logged.contains(&&OAuthProvider::GitHubCopilot));
    }

    // ------------------------------------------------------------------
    // OAuthManager — simulate_device_flow success
    // ------------------------------------------------------------------

    #[test]
    fn test_simulate_device_flow_stores_credentials() {
        let mut mgr = OAuthManager::new();
        let result = mgr.simulate_device_flow(
            OAuthProvider::GitHubCopilot,
            "mock_token_abc",
            &NoopCallbacks,
        );

        assert!(matches!(result, OAuthFlowResult::Success(_)));
        assert!(mgr.is_logged_in(&OAuthProvider::GitHubCopilot));
        assert_eq!(
            mgr.valid_token(&OAuthProvider::GitHubCopilot),
            Some("mock_token_abc")
        );
    }

    // ------------------------------------------------------------------
    // OAuthManager — auth_header
    // ------------------------------------------------------------------

    #[test]
    fn test_auth_header_uses_oauth_token_when_available() {
        let mut mgr = OAuthManager::new();
        mgr.store_credentials(fresh_creds(OAuthProvider::AnthropicClaude));

        let header = mgr
            .auth_header(&OAuthProvider::AnthropicClaude, Some("fallback_key"))
            .unwrap();
        assert_eq!(header, "Bearer tok_abcdefgh1234");
    }

    #[test]
    fn test_auth_header_falls_back_to_api_key() {
        let mgr = OAuthManager::new(); // no stored credentials

        let header = mgr
            .auth_header(&OAuthProvider::OpenAICodex, Some("sk-myapikey"))
            .unwrap();
        assert_eq!(header, "Bearer sk-myapikey");
    }

    #[test]
    fn test_auth_header_none_without_any_credentials() {
        let mgr = OAuthManager::new();
        let header = mgr.auth_header(&OAuthProvider::OpenAICodex, None);
        assert!(header.is_none());
    }

    #[test]
    fn test_auth_header_preserves_bearer_prefix_in_fallback() {
        let mgr = OAuthManager::new();
        let header = mgr
            .auth_header(
                &OAuthProvider::OpenAICodex,
                Some("Bearer already-prefixed"),
            )
            .unwrap();
        assert_eq!(header, "Bearer already-prefixed");
    }

    // ------------------------------------------------------------------
    // CacheRetentionConfig
    // ------------------------------------------------------------------

    #[test]
    fn test_openai_has_long_retention() {
        let cfg = CacheRetentionConfig::for_provider(&OAuthProvider::OpenAICodex);
        assert_eq!(cfg.retention_seconds, 86_400);
        assert!(cfg.is_long_retention());
    }

    #[test]
    fn test_anthropic_not_long_retention() {
        let cfg =
            CacheRetentionConfig::for_provider(&OAuthProvider::AnthropicClaude);
        assert_eq!(cfg.retention_seconds, 3_600);
        assert!(!cfg.is_long_retention());
    }

    // ------------------------------------------------------------------
    // OAuthProvider helpers
    // ------------------------------------------------------------------

    #[test]
    fn test_from_str_variants() {
        assert_eq!(
            OAuthProvider::from_str("claude"),
            Some(OAuthProvider::AnthropicClaude)
        );
        assert_eq!(
            OAuthProvider::from_str("copilot"),
            Some(OAuthProvider::GitHubCopilot)
        );
        assert_eq!(
            OAuthProvider::from_str("gemini"),
            Some(OAuthProvider::GoogleGeminiCli)
        );
        assert_eq!(
            OAuthProvider::from_str("chatgpt"),
            Some(OAuthProvider::OpenAICodex)
        );
        assert_eq!(OAuthProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_device_flow_support() {
        assert!(OAuthProvider::AnthropicClaude.uses_device_flow());
        assert!(OAuthProvider::GitHubCopilot.uses_device_flow());
        assert!(OAuthProvider::GoogleGeminiCli.uses_device_flow());
        assert!(!OAuthProvider::OpenAICodex.uses_device_flow());
    }

    #[test]
    fn test_remove_credentials() {
        let mut mgr = OAuthManager::new();
        mgr.store_credentials(fresh_creds(OAuthProvider::AnthropicClaude));
        assert!(mgr.is_logged_in(&OAuthProvider::AnthropicClaude));

        let removed = mgr.remove_credentials(&OAuthProvider::AnthropicClaude);
        assert!(removed);
        assert!(!mgr.is_logged_in(&OAuthProvider::AnthropicClaude));

        // Second remove returns false
        assert!(!mgr.remove_credentials(&OAuthProvider::AnthropicClaude));
    }
}
