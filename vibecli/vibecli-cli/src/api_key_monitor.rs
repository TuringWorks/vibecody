//! Periodic API key health monitor for the VibeCLI REPL.
//!
//! Spawns a background `tokio::task` that validates all configured provider API
//! keys on a fixed interval. When a key's status changes (valid → invalid or
//! recovered), it prints a colour-coded warning/success line to stderr so the
//! REPL user sees it between prompts without interrupting their typing.
//!
//! Usage (in main.rs REPL setup):
//! ```ignore
//! let monitor = ApiKeyMonitor::start(config.clone());
//! // later …
//! monitor.stop();
//! ```

use crate::config::{Config, ProviderConfig as CfgProviderConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// Human-readable labels for provider keys.
const PROVIDER_LABELS: &[(&str, &str)] = &[
    ("anthropic", "Anthropic (Claude)"),
    ("claude", "Anthropic (Claude)"),
    ("openai", "OpenAI"),
    ("gemini", "Google Gemini"),
    ("grok", "xAI (Grok)"),
    ("groq", "Groq"),
    ("openrouter", "OpenRouter"),
    ("azure_openai", "Azure OpenAI"),
    ("mistral", "Mistral AI"),
    ("cerebras", "Cerebras"),
    ("deepseek", "DeepSeek"),
    ("zhipu", "Zhipu (GLM)"),
    ("vercel_ai", "Vercel AI"),
    ("minimax", "MiniMax"),
    ("perplexity", "Perplexity"),
    ("together", "Together AI"),
    ("fireworks", "Fireworks AI"),
    ("sambanova", "SambaNova"),
    ("ollama", "Ollama"),
];

pub fn provider_label(name: &str) -> &str {
    PROVIDER_LABELS
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .unwrap_or(name)
}

/// Result of checking a single provider.
#[derive(Debug, Clone)]
pub struct ProviderHealthResult {
    pub provider: String,
    pub available: bool,
    pub error: Option<String>,
    pub latency_ms: u64,
}

/// Manages the background health-check task.
pub struct ApiKeyMonitor {
    stop_tx: watch::Sender<bool>,
}

impl ApiKeyMonitor {
    /// Start the background monitor.  Returns immediately.
    ///
    /// `interval` — how often to check (default: 5 min).
    /// `initial_delay` — wait before the first check so the REPL is ready.
    pub fn start(interval: Duration, initial_delay: Duration) -> Self {
        let (stop_tx, stop_rx) = watch::channel(false);
        tokio::spawn(monitor_loop(interval, initial_delay, stop_rx));
        Self { stop_tx }
    }

    /// Signal the background task to stop.
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

impl Drop for ApiKeyMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Build a provider from a provider config entry.
fn build_provider(name: &str, pc: &CfgProviderConfig) -> Option<Arc<dyn vibe_ai::provider::AIProvider>> {
    let api_key = pc.api_key.clone().or_else(|| resolve_env_key(name));
    let config = vibe_ai::provider::ProviderConfig {
        provider_type: name.to_string(),
        api_key,
        model: pc.model.clone().unwrap_or_default(),
        api_url: pc.api_url.clone(),
        max_tokens: None,
        temperature: None,
        ..Default::default()
    };
    match name {
        "anthropic" | "claude" => Some(Arc::new(vibe_ai::providers::claude::ClaudeProvider::new(config))),
        "openai" => Some(Arc::new(vibe_ai::providers::openai::OpenAIProvider::new(config))),
        "gemini" => Some(Arc::new(vibe_ai::providers::gemini::GeminiProvider::new(config))),
        "grok" => Some(Arc::new(vibe_ai::providers::grok::GrokProvider::new(config))),
        "groq" => Some(Arc::new(vibe_ai::providers::groq::GroqProvider::new(config))),
        "openrouter" => Some(Arc::new(vibe_ai::providers::openrouter::OpenRouterProvider::new(config))),
        "azure_openai" | "azure" => Some(Arc::new(vibe_ai::providers::azure_openai::AzureOpenAIProvider::new(config))),
        "mistral" => Some(Arc::new(vibe_ai::providers::mistral::MistralProvider::new(config))),
        "cerebras" => Some(Arc::new(vibe_ai::providers::cerebras::CerebrasProvider::new(config))),
        "deepseek" => Some(Arc::new(vibe_ai::providers::deepseek::DeepSeekProvider::new(config))),
        "zhipu" | "glm" => Some(Arc::new(vibe_ai::providers::zhipu::ZhipuProvider::new(config))),
        "vercel_ai" | "vercel" => Some(Arc::new(vibe_ai::providers::vercel_ai::VercelAIProvider::new(config))),
        "minimax" => Some(Arc::new(vibe_ai::providers::minimax::MiniMaxProvider::new(config))),
        "perplexity" => Some(Arc::new(vibe_ai::providers::perplexity::PerplexityProvider::new(config))),
        "together" => Some(Arc::new(vibe_ai::providers::together::TogetherProvider::new(config))),
        "fireworks" => Some(Arc::new(vibe_ai::providers::fireworks::FireworksProvider::new(config))),
        "sambanova" => Some(Arc::new(vibe_ai::providers::sambanova::SambaNovaProvider::new(config))),
        "ollama" => Some(Arc::new(vibe_ai::providers::ollama::OllamaProvider::new(config))),
        _ => None,
    }
}

/// Try to resolve an API key from the standard environment variable for the given provider.
fn resolve_env_key(name: &str) -> Option<String> {
    let var = match name {
        "anthropic" | "claude" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "grok" => "GROK_API_KEY",
        "groq" => "GROQ_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "azure_openai" | "azure" => "AZURE_OPENAI_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "cerebras" => "CEREBRAS_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "zhipu" | "glm" => "ZHIPU_API_KEY",
        "vercel_ai" | "vercel" => "VERCEL_AI_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "fireworks" => "FIREWORKS_API_KEY",
        "sambanova" => "SAMBANOVA_API_KEY",
        _ => return None,
    };
    std::env::var(var).ok().filter(|v| !v.is_empty())
}

/// Collect every configured provider from the config.
fn configured_providers(cfg: &Config) -> Vec<(String, Arc<dyn vibe_ai::provider::AIProvider>)> {
    let all_names = [
        "anthropic", "openai", "gemini", "grok", "groq", "openrouter",
        "azure_openai", "mistral", "cerebras", "deepseek", "zhipu",
        "vercel_ai", "minimax", "perplexity", "together", "fireworks",
        "sambanova", "ollama",
    ];
    let mut providers = Vec::new();
    for name in &all_names {
        if let Some(pc) = cfg.get_provider_config(name) {
            if pc.enabled {
                if let Some(p) = build_provider(name, pc) {
                    providers.push((name.to_string(), p));
                }
            }
        } else if resolve_env_key(name).is_some() {
            // Provider not in config but env var is set — still check it
            let pc = CfgProviderConfig {
                enabled: true,
                api_key: resolve_env_key(name),
                ..Default::default()
            };
            if let Some(p) = build_provider(name, &pc) {
                providers.push((name.to_string(), p));
            }
        }
    }
    providers
}

/// Check all providers and return health results.
pub async fn check_all_providers(cfg: &Config) -> Vec<ProviderHealthResult> {
    let providers = configured_providers(cfg);
    let mut results = Vec::with_capacity(providers.len());

    for (name, provider) in providers {
        let start = std::time::Instant::now();
        let available = provider.is_available().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        results.push(ProviderHealthResult {
            provider: name,
            available,
            error: if available { None } else { Some("not available".to_string()) },
            latency_ms,
        });
    }
    results
}

/// Background loop that periodically checks provider health.
async fn monitor_loop(interval: Duration, initial_delay: Duration, mut stop_rx: watch::Receiver<bool>) {
    // Wait for initial delay (let REPL boot up)
    tokio::select! {
        _ = tokio::time::sleep(initial_delay) => {},
        _ = stop_rx.changed() => return,
    }

    let mut prev_status: HashMap<String, bool> = HashMap::new();
    let mut is_first = true;

    loop {
        // Load config fresh each cycle (user may edit config.toml between checks)
        let cfg = Config::load().unwrap_or_default();
        let results = check_all_providers(&cfg).await;

        if is_first {
            // First run: report any currently-failing keys
            let failing: Vec<&ProviderHealthResult> = results.iter().filter(|r| !r.available).collect();
            if !failing.is_empty() {
                let names: Vec<&str> = failing.iter().map(|r| provider_label(&r.provider)).collect();
                eprintln!(
                    "\x1b[33m[vibecli] API key health check: {} provider(s) unavailable: {}\x1b[0m",
                    failing.len(),
                    names.join(", ")
                );
            }
            is_first = false;
        } else {
            // Subsequent runs: only report changes
            for r in &results {
                let was_ok = prev_status.get(&r.provider).copied();
                match (was_ok, r.available) {
                    (Some(true), false) => {
                        eprintln!(
                            "\x1b[31m[vibecli] {} API key is no longer working{}\x1b[0m",
                            provider_label(&r.provider),
                            r.error.as_deref().map(|e| format!(" ({})", e)).unwrap_or_default()
                        );
                    }
                    (Some(false), true) => {
                        eprintln!(
                            "\x1b[32m[vibecli] {} API key recovered ({}ms)\x1b[0m",
                            provider_label(&r.provider),
                            r.latency_ms
                        );
                    }
                    _ => {}
                }
            }
        }

        // Update previous status
        for r in &results {
            prev_status.insert(r.provider.clone(), r.available);
        }

        // Wait for next cycle or stop signal
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = stop_rx.changed() => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_label_known() {
        assert_eq!(provider_label("anthropic"), "Anthropic (Claude)");
        assert_eq!(provider_label("openai"), "OpenAI");
        assert_eq!(provider_label("ollama"), "Ollama");
        assert_eq!(provider_label("groq"), "Groq");
        assert_eq!(provider_label("azure_openai"), "Azure OpenAI");
        assert_eq!(provider_label("mistral"), "Mistral AI");
        assert_eq!(provider_label("cerebras"), "Cerebras");
        assert_eq!(provider_label("deepseek"), "DeepSeek");
        assert_eq!(provider_label("zhipu"), "Zhipu (GLM)");
        assert_eq!(provider_label("vercel_ai"), "Vercel AI");
        assert_eq!(provider_label("minimax"), "MiniMax");
        assert_eq!(provider_label("perplexity"), "Perplexity");
        assert_eq!(provider_label("together"), "Together AI");
        assert_eq!(provider_label("fireworks"), "Fireworks AI");
        assert_eq!(provider_label("sambanova"), "SambaNova");
    }

    #[test]
    fn test_provider_label_unknown() {
        assert_eq!(provider_label("custom_thing"), "custom_thing");
    }

    #[test]
    fn test_resolve_env_key_mappings() {
        // Verify the function returns None for providers with no env var set.
        // (We don't set env vars in unit tests to avoid side-effects.)
        assert!(resolve_env_key("unknown_provider").is_none());
    }

    #[test]
    fn test_configured_providers_default_config() {
        // Default config has no providers enabled (no keys set).
        let cfg = Config::default();
        let providers = configured_providers(&cfg);
        // Without env vars, should be empty.
        // (In CI, env vars might be set, so we just check it doesn't panic.)
        let _ = providers;
    }

    #[test]
    fn test_health_result_fields() {
        let r = ProviderHealthResult {
            provider: "openai".to_string(),
            available: false,
            error: Some("401 Unauthorized".to_string()),
            latency_ms: 150,
        };
        assert_eq!(r.provider, "openai");
        assert!(!r.available);
        assert_eq!(r.error.as_deref(), Some("401 Unauthorized"));
        assert_eq!(r.latency_ms, 150);
    }

    #[test]
    fn test_monitor_start_stop() {
        // Verify that creating and immediately dropping a monitor doesn't panic.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let monitor = ApiKeyMonitor::start(Duration::from_secs(3600), Duration::from_secs(3600));
            monitor.stop();
        });
    }

    #[tokio::test]
    async fn test_check_all_providers_no_crash() {
        let cfg = Config::default();
        let results = check_all_providers(&cfg).await;
        // Should return results (possibly empty) without panicking
        for r in &results {
            assert!(!r.provider.is_empty());
        }
    }

    #[test]
    fn test_build_provider_known() {
        let pc = CfgProviderConfig {
            enabled: true,
            api_key: Some("test-key".to_string()),
            ..Default::default()
        };
        // Known providers should return Some
        for name in &["anthropic", "openai", "gemini", "grok", "groq", "ollama",
                       "mistral", "cerebras", "deepseek", "zhipu", "minimax",
                       "perplexity", "together", "fireworks", "sambanova"] {
            assert!(build_provider(name, &pc).is_some(), "Failed to build provider: {}", name);
        }
    }

    #[test]
    fn test_build_provider_unknown() {
        let pc = CfgProviderConfig { enabled: true, ..Default::default() };
        assert!(build_provider("nonexistent_provider", &pc).is_none());
    }
}
