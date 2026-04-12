#![allow(dead_code)]
//! On-device model registry, hardware capability, and local-only policy guard.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── HardwareBackend ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardwareBackend {
    Metal,
    Cuda { compute_capability: String },
    Rocm,
    CpuAvx2,
    CpuFallback,
}

// ─── QuantizationType ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantizationType {
    F32,
    F16,
    Q8_0,
    Q4KM,
    Q4_0,
    Q2K,
}

// ─── ModelFormat ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelFormat {
    Gguf,
    SafeTensors,
    Ggml,
}

// ─── ModelCard ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCard {
    pub model_id: String,
    pub name: String,
    pub format: ModelFormat,
    pub quant: QuantizationType,
    pub size_mb: u64,
    pub context_length: u32,
    pub parameter_count_b: f32,
    pub sha256: String,
}

// ─── ModelRegistry ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ModelRegistry {
    cards: HashMap<String, ModelCard>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    /// Registers a model card. Returns `Err` if the `model_id` already exists.
    pub fn register(&mut self, card: ModelCard) -> Result<(), String> {
        if self.cards.contains_key(&card.model_id) {
            return Err(format!("model '{}' is already registered", card.model_id));
        }
        self.cards.insert(card.model_id.clone(), card);
        Ok(())
    }

    pub fn get(&self, model_id: &str) -> Option<&ModelCard> {
        self.cards.get(model_id)
    }

    pub fn list_by_quant(&self, quant: &QuantizationType) -> Vec<&ModelCard> {
        let mut result: Vec<&ModelCard> = self
            .cards
            .values()
            .filter(|c| &c.quant == quant)
            .collect();
        result.sort_by(|a, b| a.model_id.cmp(&b.model_id));
        result
    }

    pub fn total_size_mb(&self) -> u64 {
        self.cards.values().map(|c| c.size_mb).sum()
    }

    pub fn model_count(&self) -> usize {
        self.cards.len()
    }
}

// ─── HardwareCapability ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapability {
    pub backend: HardwareBackend,
    pub vram_mb: Option<u64>,
    /// Estimated tokens per second.
    pub estimated_tps: f32,
}

impl HardwareCapability {
    /// Returns true for GPU-class backends (Metal, Cuda, Rocm).
    pub fn is_gpu(&self) -> bool {
        matches!(
            &self.backend,
            HardwareBackend::Metal | HardwareBackend::Cuda { .. } | HardwareBackend::Rocm
        )
    }

    pub fn backend_name(&self) -> &str {
        match &self.backend {
            HardwareBackend::Metal => "Metal",
            HardwareBackend::Cuda { .. } => "CUDA",
            HardwareBackend::Rocm => "ROCm",
            HardwareBackend::CpuAvx2 => "CPU (AVX2)",
            HardwareBackend::CpuFallback => "CPU (fallback)",
        }
    }
}

// ─── BenchmarkResult ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub model_id: String,
    pub backend: HardwareBackend,
    pub median_tps: f32,
    pub first_token_ms: u64,
    pub memory_mb: u64,
    pub run_count: u32,
}

impl BenchmarkResult {
    pub fn is_faster_than(&self, other: &BenchmarkResult) -> bool {
        self.median_tps > other.median_tps
    }
}

// ─── LocalOnlyConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalOnlyConfig {
    pub enabled: bool,
    pub allow_localhost: bool,
    pub blocked_providers: Vec<String>,
}

impl LocalOnlyConfig {
    /// Strict mode: all remote providers blocked.
    pub fn new_enforcing() -> Self {
        Self {
            enabled: true,
            allow_localhost: true,
            blocked_providers: vec![
                "openai".to_string(),
                "anthropic".to_string(),
                "gemini".to_string(),
                "groq".to_string(),
                "grok".to_string(),
                "openrouter".to_string(),
                "azure".to_string(),
                "bedrock".to_string(),
                "copilot".to_string(),
                "mistral".to_string(),
                "cerebras".to_string(),
                "deepseek".to_string(),
                "zhipu".to_string(),
                "vercel".to_string(),
            ],
        }
    }

    /// Permissive mode: nothing is blocked.
    pub fn new_permissive() -> Self {
        Self {
            enabled: false,
            allow_localhost: true,
            blocked_providers: vec![],
        }
    }
}

// ─── LocalOnlyGuard ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LocalOnlyGuard {
    config: LocalOnlyConfig,
}

impl LocalOnlyGuard {
    pub fn new(config: LocalOnlyConfig) -> Self {
        Self { config }
    }

    /// Checks whether a request to `provider` at `host` is permitted.
    pub fn is_allowed(&self, provider: &str, host: &str) -> bool {
        if !self.config.enabled {
            return true;
        }
        let is_local = host == "localhost" || host == "127.0.0.1";
        if is_local {
            return self.config.allow_localhost;
        }
        // Remote host — check provider blocklist
        let provider_lower = provider.to_lowercase();
        !self
            .config
            .blocked_providers
            .iter()
            .any(|b| b.to_lowercase() == provider_lower)
    }

    pub fn config(&self) -> &LocalOnlyConfig {
        &self.config
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_card(id: &str, quant: QuantizationType, size_mb: u64) -> ModelCard {
        ModelCard {
            model_id: id.to_string(),
            name: format!("Model {}", id),
            format: ModelFormat::Gguf,
            quant,
            size_mb,
            context_length: 4096,
            parameter_count_b: 7.0,
            sha256: format!("sha256_{}", id),
        }
    }

    // ── ModelCard ─────────────────────────────────────────────────────────

    #[test]
    fn test_model_card_fields() {
        let card = sample_card("llama-7b", QuantizationType::Q4_0, 4096);
        assert_eq!(card.model_id, "llama-7b");
        assert_eq!(card.size_mb, 4096);
        assert_eq!(card.context_length, 4096);
    }

    // ── ModelRegistry ─────────────────────────────────────────────────────

    #[test]
    fn test_registry_new_empty() {
        let r = ModelRegistry::new();
        assert_eq!(r.model_count(), 0);
        assert_eq!(r.total_size_mb(), 0);
    }

    #[test]
    fn test_registry_register_success() {
        let mut r = ModelRegistry::new();
        let result = r.register(sample_card("m1", QuantizationType::F32, 1000));
        assert!(result.is_ok());
        assert_eq!(r.model_count(), 1);
    }

    #[test]
    fn test_registry_register_duplicate_fails() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("m1", QuantizationType::F32, 1000)).unwrap();
        let result = r.register(sample_card("m1", QuantizationType::F16, 500));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_get_existing() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("m2", QuantizationType::Q8_0, 2000)).unwrap();
        let card = r.get("m2");
        assert!(card.is_some());
        assert_eq!(card.unwrap().model_id, "m2");
    }

    #[test]
    fn test_registry_get_missing() {
        let r = ModelRegistry::new();
        assert!(r.get("nope").is_none());
    }

    #[test]
    fn test_registry_list_by_quant_match() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("a", QuantizationType::Q4KM, 2000)).unwrap();
        r.register(sample_card("b", QuantizationType::Q4KM, 1500)).unwrap();
        r.register(sample_card("c", QuantizationType::F32, 8000)).unwrap();
        let q4km = r.list_by_quant(&QuantizationType::Q4KM);
        assert_eq!(q4km.len(), 2);
    }

    #[test]
    fn test_registry_list_by_quant_no_match() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("a", QuantizationType::F32, 8000)).unwrap();
        let result = r.list_by_quant(&QuantizationType::Q2K);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_registry_total_size_mb() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("a", QuantizationType::F32, 1000)).unwrap();
        r.register(sample_card("b", QuantizationType::F16, 2000)).unwrap();
        assert_eq!(r.total_size_mb(), 3000);
    }

    #[test]
    fn test_registry_model_count_multiple() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("a", QuantizationType::F32, 1000)).unwrap();
        r.register(sample_card("b", QuantizationType::F16, 2000)).unwrap();
        r.register(sample_card("c", QuantizationType::Q4_0, 500)).unwrap();
        assert_eq!(r.model_count(), 3);
    }

    #[test]
    fn test_registry_list_by_quant_returns_correct_cards() {
        let mut r = ModelRegistry::new();
        r.register(sample_card("x", QuantizationType::Q8_0, 3000)).unwrap();
        r.register(sample_card("y", QuantizationType::Q8_0, 3100)).unwrap();
        let cards = r.list_by_quant(&QuantizationType::Q8_0);
        let ids: Vec<&str> = cards.iter().map(|c| c.model_id.as_str()).collect();
        assert!(ids.contains(&"x"));
        assert!(ids.contains(&"y"));
    }

    // ── HardwareCapability ────────────────────────────────────────────────

    #[test]
    fn test_is_gpu_metal() {
        let h = HardwareCapability { backend: HardwareBackend::Metal, vram_mb: Some(16384), estimated_tps: 50.0 };
        assert!(h.is_gpu());
    }

    #[test]
    fn test_is_gpu_cuda() {
        let h = HardwareCapability {
            backend: HardwareBackend::Cuda { compute_capability: "8.6".into() },
            vram_mb: Some(24576),
            estimated_tps: 80.0,
        };
        assert!(h.is_gpu());
    }

    #[test]
    fn test_is_gpu_rocm() {
        let h = HardwareCapability { backend: HardwareBackend::Rocm, vram_mb: Some(8192), estimated_tps: 40.0 };
        assert!(h.is_gpu());
    }

    #[test]
    fn test_is_gpu_cpu_avx2_false() {
        let h = HardwareCapability { backend: HardwareBackend::CpuAvx2, vram_mb: None, estimated_tps: 10.0 };
        assert!(!h.is_gpu());
    }

    #[test]
    fn test_is_gpu_cpu_fallback_false() {
        let h = HardwareCapability { backend: HardwareBackend::CpuFallback, vram_mb: None, estimated_tps: 3.0 };
        assert!(!h.is_gpu());
    }

    #[test]
    fn test_backend_name_metal() {
        let h = HardwareCapability { backend: HardwareBackend::Metal, vram_mb: None, estimated_tps: 50.0 };
        assert_eq!(h.backend_name(), "Metal");
    }

    #[test]
    fn test_backend_name_cuda() {
        let h = HardwareCapability {
            backend: HardwareBackend::Cuda { compute_capability: "7.5".into() },
            vram_mb: None,
            estimated_tps: 60.0,
        };
        assert_eq!(h.backend_name(), "CUDA");
    }

    #[test]
    fn test_backend_name_rocm() {
        let h = HardwareCapability { backend: HardwareBackend::Rocm, vram_mb: None, estimated_tps: 40.0 };
        assert_eq!(h.backend_name(), "ROCm");
    }

    #[test]
    fn test_backend_name_cpu_avx2() {
        let h = HardwareCapability { backend: HardwareBackend::CpuAvx2, vram_mb: None, estimated_tps: 10.0 };
        assert_eq!(h.backend_name(), "CPU (AVX2)");
    }

    #[test]
    fn test_backend_name_cpu_fallback() {
        let h = HardwareCapability { backend: HardwareBackend::CpuFallback, vram_mb: None, estimated_tps: 3.0 };
        assert_eq!(h.backend_name(), "CPU (fallback)");
    }

    // ── BenchmarkResult ───────────────────────────────────────────────────

    #[test]
    fn test_is_faster_than_true() {
        let a = BenchmarkResult { model_id: "a".into(), backend: HardwareBackend::Metal, median_tps: 80.0, first_token_ms: 100, memory_mb: 4096, run_count: 5 };
        let b = BenchmarkResult { model_id: "b".into(), backend: HardwareBackend::CpuAvx2, median_tps: 10.0, first_token_ms: 500, memory_mb: 2048, run_count: 5 };
        assert!(a.is_faster_than(&b));
    }

    #[test]
    fn test_is_faster_than_false() {
        let a = BenchmarkResult { model_id: "a".into(), backend: HardwareBackend::CpuFallback, median_tps: 5.0, first_token_ms: 800, memory_mb: 2048, run_count: 3 };
        let b = BenchmarkResult { model_id: "b".into(), backend: HardwareBackend::Metal, median_tps: 70.0, first_token_ms: 120, memory_mb: 8192, run_count: 3 };
        assert!(!a.is_faster_than(&b));
    }

    #[test]
    fn test_is_faster_than_equal() {
        let a = BenchmarkResult { model_id: "a".into(), backend: HardwareBackend::Metal, median_tps: 50.0, first_token_ms: 150, memory_mb: 4096, run_count: 5 };
        let b = BenchmarkResult { model_id: "b".into(), backend: HardwareBackend::Metal, median_tps: 50.0, first_token_ms: 150, memory_mb: 4096, run_count: 5 };
        assert!(!a.is_faster_than(&b));
    }

    // ── LocalOnlyGuard ────────────────────────────────────────────────────

    #[test]
    fn test_guard_permissive_allows_all() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_permissive());
        assert!(g.is_allowed("openai", "api.openai.com"));
        assert!(g.is_allowed("anthropic", "api.anthropic.com"));
    }

    #[test]
    fn test_guard_enforcing_blocks_openai() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(!g.is_allowed("openai", "api.openai.com"));
    }

    #[test]
    fn test_guard_enforcing_blocks_anthropic() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(!g.is_allowed("anthropic", "api.anthropic.com"));
    }

    #[test]
    fn test_guard_enforcing_allows_localhost() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(g.is_allowed("ollama", "localhost"));
    }

    #[test]
    fn test_guard_enforcing_allows_127_0_0_1() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(g.is_allowed("ollama", "127.0.0.1"));
    }

    #[test]
    fn test_guard_blocks_localhost_when_not_allowed() {
        let config = LocalOnlyConfig {
            enabled: true,
            allow_localhost: false,
            blocked_providers: vec![],
        };
        let g = LocalOnlyGuard::new(config);
        assert!(!g.is_allowed("ollama", "localhost"));
    }

    #[test]
    fn test_guard_allows_unknown_provider_on_remote() {
        let config = LocalOnlyConfig {
            enabled: true,
            allow_localhost: true,
            blocked_providers: vec!["openai".into()],
        };
        let g = LocalOnlyGuard::new(config);
        assert!(g.is_allowed("unknown-provider", "some.remote.host"));
    }

    #[test]
    fn test_guard_blocks_case_insensitive() {
        let config = LocalOnlyConfig {
            enabled: true,
            allow_localhost: true,
            blocked_providers: vec!["OpenAI".into()],
        };
        let g = LocalOnlyGuard::new(config);
        assert!(!g.is_allowed("openai", "api.openai.com"));
    }

    #[test]
    fn test_guard_config_accessor() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_permissive());
        assert!(!g.config().enabled);
    }

    #[test]
    fn test_guard_enforcing_config_enabled() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(g.config().enabled);
    }

    #[test]
    fn test_guard_permissive_config_not_enabled() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_permissive());
        assert!(!g.config().enabled);
    }

    #[test]
    fn test_guard_enforcing_blocks_groq() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(!g.is_allowed("groq", "api.groq.com"));
    }

    #[test]
    fn test_guard_enforcing_blocks_deepseek() {
        let g = LocalOnlyGuard::new(LocalOnlyConfig::new_enforcing());
        assert!(!g.is_allowed("deepseek", "api.deepseek.com"));
    }
}
