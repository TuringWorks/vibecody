#![allow(dead_code)]
//! Model provider marketplace for VibeCody.
//!
//! Browse, compare, and configure AI models with guided selection,
//! benchmarks, and pricing. Goes beyond BYOK to help users pick
//! the best model for their task and budget.
//!
//! REPL commands: `/marketplace browse|compare|recommend|estimate|rate`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    CodeGeneration,
    Chat,
    Vision,
    ToolUse,
    Streaming,
    SystemPrompt,
    MultiModal,
    FunctionCalling,
    JsonMode,
    ReasoningMode,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodeGeneration => write!(f, "code-generation"),
            Self::Chat => write!(f, "chat"),
            Self::Vision => write!(f, "vision"),
            Self::ToolUse => write!(f, "tool-use"),
            Self::Streaming => write!(f, "streaming"),
            Self::SystemPrompt => write!(f, "system-prompt"),
            Self::MultiModal => write!(f, "multi-modal"),
            Self::FunctionCalling => write!(f, "function-calling"),
            Self::JsonMode => write!(f, "json-mode"),
            Self::ReasoningMode => write!(f, "reasoning-mode"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarketplaceError {
    ModelNotFound,
    DuplicateModel,
    FilterError,
    InvalidRating,
}

impl std::fmt::Display for MarketplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelNotFound => write!(f, "model not found"),
            Self::DuplicateModel => write!(f, "duplicate model"),
            Self::FilterError => write!(f, "filter error"),
            Self::InvalidRating => write!(f, "invalid rating (must be 0.0-5.0)"),
        }
    }
}

// === Data Types ===

#[derive(Debug, Clone)]
pub struct MarketplaceConfig {
    pub cache_dir: String,
    pub auto_update: bool,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            cache_dir: ".vibecody/models".to_string(),
            auto_update: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub free_tier: bool,
    pub currency: String,
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input_per_million: 0.0,
            output_per_million: 0.0,
            free_tier: false,
            currency: "USD".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModelBenchmarks {
    pub swe_bench_score: Option<f32>,
    pub humaneval_score: Option<f32>,
    pub mbpp_score: Option<f32>,
    pub arena_elo: Option<u32>,
    pub mmlu_score: Option<f32>,
    pub speed_tokens_per_sec: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_family: String,
    pub capabilities: Vec<Capability>,
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub pricing: ModelPricing,
    pub benchmarks: ModelBenchmarks,
    pub release_date: String,
    pub description: String,
    pub supported_features: Vec<String>,
    pub community_rating: Option<f32>,
    pub rating_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelComparison {
    pub models: Vec<String>,
    pub comparison_table: Vec<ComparisonRow>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonRow {
    pub attribute: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    pub model_id: String,
    pub daily_tokens_input: u64,
    pub daily_tokens_output: u64,
    pub daily_cost: f64,
    pub monthly_cost: f64,
    pub yearly_cost: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ModelFilter {
    pub provider: Option<String>,
    pub capability: Option<Capability>,
    pub max_price_input: Option<f64>,
    pub min_context_window: Option<usize>,
    pub min_swe_bench: Option<f32>,
    pub free_only: bool,
}

#[derive(Debug, Clone)]
pub struct ModelRecommendation {
    pub model_id: String,
    pub reason: String,
    pub score: f32,
}

// === ModelMarketplace ===

pub struct ModelMarketplace {
    config: MarketplaceConfig,
    models: HashMap<String, ModelEntry>,
}

impl ModelMarketplace {
    /// Create a new marketplace with pre-loaded default models.
    pub fn new(config: MarketplaceConfig) -> Self {
        let mut mp = Self {
            config,
            models: HashMap::new(),
        };
        mp.load_default_models();
        mp
    }

    /// Populate the registry with 20+ known models and realistic pricing/benchmarks.
    pub fn load_default_models(&mut self) {
        let defaults = vec![
            // Anthropic models
            ModelEntry {
                id: "claude-opus-4.6".into(),
                name: "Claude Opus 4.6".into(),
                provider: "anthropic".into(),
                model_family: "claude-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::MultiModal, Capability::FunctionCalling, Capability::JsonMode,
                    Capability::ReasoningMode,
                ],
                context_window: 1_000_000,
                max_output_tokens: 32_000,
                pricing: ModelPricing { input_per_million: 15.0, output_per_million: 75.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(72.0), humaneval_score: Some(96.4), mbpp_score: Some(91.0), arena_elo: Some(1380), mmlu_score: Some(92.0), speed_tokens_per_sec: Some(60.0) },
                release_date: "2026-02-01".into(),
                description: "Most capable Anthropic model for complex reasoning and code".into(),
                supported_features: vec!["extended-thinking".into(), "tool-use".into(), "vision".into()],
                community_rating: Some(4.8),
                rating_count: 1250,
            },
            ModelEntry {
                id: "claude-sonnet-4.5".into(),
                name: "Claude Sonnet 4.5".into(),
                provider: "anthropic".into(),
                model_family: "claude-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::FunctionCalling, Capability::JsonMode,
                ],
                context_window: 200_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 3.0, output_per_million: 15.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(62.0), humaneval_score: Some(93.7), mbpp_score: Some(88.0), arena_elo: Some(1340), mmlu_score: Some(89.5), speed_tokens_per_sec: Some(110.0) },
                release_date: "2026-01-15".into(),
                description: "Best balance of speed and intelligence for daily coding".into(),
                supported_features: vec!["tool-use".into(), "vision".into()],
                community_rating: Some(4.6),
                rating_count: 2100,
            },
            ModelEntry {
                id: "claude-haiku-4.5".into(),
                name: "Claude Haiku 4.5".into(),
                provider: "anthropic".into(),
                model_family: "claude-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt, Capability::FunctionCalling, Capability::JsonMode,
                ],
                context_window: 200_000,
                max_output_tokens: 8_000,
                pricing: ModelPricing { input_per_million: 0.25, output_per_million: 1.25, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(41.0), humaneval_score: Some(85.0), mbpp_score: Some(80.0), arena_elo: Some(1250), mmlu_score: Some(82.0), speed_tokens_per_sec: Some(200.0) },
                release_date: "2025-11-01".into(),
                description: "Fast and affordable for high-volume tasks".into(),
                supported_features: vec!["tool-use".into()],
                community_rating: Some(4.2),
                rating_count: 3200,
            },
            // OpenAI models
            ModelEntry {
                id: "gpt-5.4".into(),
                name: "GPT-5.4".into(),
                provider: "openai".into(),
                model_family: "gpt-5".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::MultiModal, Capability::FunctionCalling, Capability::JsonMode,
                    Capability::ReasoningMode,
                ],
                context_window: 256_000,
                max_output_tokens: 32_000,
                pricing: ModelPricing { input_per_million: 12.0, output_per_million: 60.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(68.0), humaneval_score: Some(95.0), mbpp_score: Some(90.0), arena_elo: Some(1370), mmlu_score: Some(91.0), speed_tokens_per_sec: Some(80.0) },
                release_date: "2026-01-20".into(),
                description: "OpenAI flagship model with strong reasoning".into(),
                supported_features: vec!["reasoning".into(), "vision".into(), "tools".into()],
                community_rating: Some(4.7),
                rating_count: 1800,
            },
            ModelEntry {
                id: "gpt-5.3".into(),
                name: "GPT-5.3".into(),
                provider: "openai".into(),
                model_family: "gpt-5".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::FunctionCalling, Capability::JsonMode,
                ],
                context_window: 128_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 5.0, output_per_million: 25.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(55.0), humaneval_score: Some(91.0), mbpp_score: Some(86.0), arena_elo: Some(1320), mmlu_score: Some(88.0), speed_tokens_per_sec: Some(120.0) },
                release_date: "2025-12-01".into(),
                description: "Strong general-purpose model at moderate cost".into(),
                supported_features: vec!["vision".into(), "tools".into()],
                community_rating: Some(4.4),
                rating_count: 2500,
            },
            ModelEntry {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                provider: "openai".into(),
                model_family: "gpt-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::MultiModal, Capability::FunctionCalling, Capability::JsonMode,
                ],
                context_window: 128_000,
                max_output_tokens: 16_384,
                pricing: ModelPricing { input_per_million: 2.50, output_per_million: 10.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(48.0), humaneval_score: Some(90.2), mbpp_score: Some(85.0), arena_elo: Some(1290), mmlu_score: Some(87.2), speed_tokens_per_sec: Some(150.0) },
                release_date: "2024-05-13".into(),
                description: "Fast multimodal model with broad capabilities".into(),
                supported_features: vec!["vision".into(), "audio".into(), "tools".into()],
                community_rating: Some(4.3),
                rating_count: 5000,
            },
            ModelEntry {
                id: "codex-2".into(),
                name: "Codex 2".into(),
                provider: "openai".into(),
                model_family: "codex".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::ToolUse,
                    Capability::Streaming, Capability::SystemPrompt, Capability::FunctionCalling,
                ],
                context_window: 192_000,
                max_output_tokens: 32_000,
                pricing: ModelPricing { input_per_million: 6.0, output_per_million: 30.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(70.0), humaneval_score: Some(97.0), mbpp_score: Some(92.0), arena_elo: Some(1360), mmlu_score: None, speed_tokens_per_sec: Some(90.0) },
                release_date: "2026-02-10".into(),
                description: "Specialized code generation model from OpenAI".into(),
                supported_features: vec!["code-execution".into(), "tools".into()],
                community_rating: Some(4.7),
                rating_count: 900,
            },
            // Google models
            ModelEntry {
                id: "gemini-2.5-pro".into(),
                name: "Gemini 2.5 Pro".into(),
                provider: "google".into(),
                model_family: "gemini-2".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::MultiModal, Capability::FunctionCalling, Capability::JsonMode,
                    Capability::ReasoningMode,
                ],
                context_window: 2_000_000,
                max_output_tokens: 65_536,
                pricing: ModelPricing { input_per_million: 7.0, output_per_million: 21.0, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(63.8), humaneval_score: Some(94.0), mbpp_score: Some(89.0), arena_elo: Some(1350), mmlu_score: Some(90.5), speed_tokens_per_sec: Some(95.0) },
                release_date: "2025-12-15".into(),
                description: "Google flagship with 2M context window and strong reasoning".into(),
                supported_features: vec!["grounding".into(), "code-execution".into(), "vision".into()],
                community_rating: Some(4.5),
                rating_count: 1600,
            },
            ModelEntry {
                id: "gemini-2.5-flash".into(),
                name: "Gemini 2.5 Flash".into(),
                provider: "google".into(),
                model_family: "gemini-2".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::Streaming, Capability::SystemPrompt, Capability::FunctionCalling,
                    Capability::JsonMode,
                ],
                context_window: 1_000_000,
                max_output_tokens: 32_768,
                pricing: ModelPricing { input_per_million: 0.15, output_per_million: 0.60, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(42.0), humaneval_score: Some(86.0), mbpp_score: Some(81.0), arena_elo: Some(1260), mmlu_score: Some(83.0), speed_tokens_per_sec: Some(250.0) },
                release_date: "2025-11-20".into(),
                description: "Ultra-fast and cheap with large context".into(),
                supported_features: vec!["grounding".into(), "vision".into()],
                community_rating: Some(4.1),
                rating_count: 2800,
            },
            // Meta models
            ModelEntry {
                id: "llama-4-405b".into(),
                name: "Llama 4 405B".into(),
                provider: "meta".into(),
                model_family: "llama-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::ToolUse,
                    Capability::Streaming, Capability::SystemPrompt, Capability::FunctionCalling,
                ],
                context_window: 128_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 0.0, output_per_million: 0.0, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(50.0), humaneval_score: Some(89.0), mbpp_score: Some(84.0), arena_elo: Some(1300), mmlu_score: Some(86.0), speed_tokens_per_sec: Some(40.0) },
                release_date: "2025-10-01".into(),
                description: "Open-weight flagship model, self-hostable".into(),
                supported_features: vec!["self-hosted".into(), "fine-tunable".into()],
                community_rating: Some(4.4),
                rating_count: 4000,
            },
            ModelEntry {
                id: "llama-4-70b".into(),
                name: "Llama 4 70B".into(),
                provider: "meta".into(),
                model_family: "llama-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt, Capability::FunctionCalling,
                ],
                context_window: 128_000,
                max_output_tokens: 8_000,
                pricing: ModelPricing { input_per_million: 0.0, output_per_million: 0.0, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(38.0), humaneval_score: Some(82.0), mbpp_score: Some(78.0), arena_elo: Some(1240), mmlu_score: Some(80.0), speed_tokens_per_sec: Some(90.0) },
                release_date: "2025-10-01".into(),
                description: "Strong open model for local or cloud deployment".into(),
                supported_features: vec!["self-hosted".into(), "fine-tunable".into()],
                community_rating: Some(4.2),
                rating_count: 3500,
            },
            // Mistral models
            ModelEntry {
                id: "mistral-large-3".into(),
                name: "Mistral Large 3".into(),
                provider: "mistral".into(),
                model_family: "mistral-large".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::ToolUse,
                    Capability::Streaming, Capability::SystemPrompt, Capability::FunctionCalling,
                    Capability::JsonMode,
                ],
                context_window: 128_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 2.0, output_per_million: 6.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(45.0), humaneval_score: Some(88.0), mbpp_score: Some(83.0), arena_elo: Some(1280), mmlu_score: Some(84.5), speed_tokens_per_sec: Some(130.0) },
                release_date: "2025-11-10".into(),
                description: "European frontier model with strong multilingual support".into(),
                supported_features: vec!["tool-use".into(), "json-mode".into()],
                community_rating: Some(4.2),
                rating_count: 1400,
            },
            ModelEntry {
                id: "codestral-2".into(),
                name: "Codestral 2".into(),
                provider: "mistral".into(),
                model_family: "codestral".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt, Capability::FunctionCalling,
                ],
                context_window: 64_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 0.30, output_per_million: 0.90, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(40.0), humaneval_score: Some(90.0), mbpp_score: Some(85.0), arena_elo: Some(1270), mmlu_score: None, speed_tokens_per_sec: Some(180.0) },
                release_date: "2025-09-15".into(),
                description: "Dedicated code model with fill-in-the-middle support".into(),
                supported_features: vec!["fim".into(), "code-completion".into()],
                community_rating: Some(4.3),
                rating_count: 1100,
            },
            // DeepSeek models
            ModelEntry {
                id: "deepseek-v3".into(),
                name: "DeepSeek V3".into(),
                provider: "deepseek".into(),
                model_family: "deepseek-v3".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::ToolUse,
                    Capability::Streaming, Capability::SystemPrompt, Capability::FunctionCalling,
                    Capability::JsonMode, Capability::ReasoningMode,
                ],
                context_window: 128_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 0.27, output_per_million: 1.10, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(55.0), humaneval_score: Some(91.5), mbpp_score: Some(87.0), arena_elo: Some(1330), mmlu_score: Some(87.5), speed_tokens_per_sec: Some(100.0) },
                release_date: "2025-12-20".into(),
                description: "Cost-effective MoE model with strong coding abilities".into(),
                supported_features: vec!["reasoning".into(), "tool-use".into()],
                community_rating: Some(4.5),
                rating_count: 2200,
            },
            ModelEntry {
                id: "deepseek-r2".into(),
                name: "DeepSeek R2".into(),
                provider: "deepseek".into(),
                model_family: "deepseek-r".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt, Capability::ReasoningMode,
                ],
                context_window: 128_000,
                max_output_tokens: 32_000,
                pricing: ModelPricing { input_per_million: 0.55, output_per_million: 2.19, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(58.0), humaneval_score: Some(92.0), mbpp_score: Some(88.0), arena_elo: Some(1345), mmlu_score: Some(88.5), speed_tokens_per_sec: Some(70.0) },
                release_date: "2026-01-10".into(),
                description: "Reasoning-focused model with chain-of-thought".into(),
                supported_features: vec!["reasoning".into(), "math".into()],
                community_rating: Some(4.6),
                rating_count: 1700,
            },
            // xAI models
            ModelEntry {
                id: "grok-3".into(),
                name: "Grok 3".into(),
                provider: "xai".into(),
                model_family: "grok".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Vision,
                    Capability::ToolUse, Capability::Streaming, Capability::SystemPrompt,
                    Capability::FunctionCalling, Capability::JsonMode,
                ],
                context_window: 131_072,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 3.0, output_per_million: 15.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(52.0), humaneval_score: Some(90.0), mbpp_score: Some(85.0), arena_elo: Some(1310), mmlu_score: Some(87.0), speed_tokens_per_sec: Some(100.0) },
                release_date: "2025-12-05".into(),
                description: "xAI model with real-time knowledge and humor".into(),
                supported_features: vec!["real-time".into(), "vision".into()],
                community_rating: Some(4.3),
                rating_count: 1300,
            },
            // Groq-hosted models
            ModelEntry {
                id: "groq-llama-4-70b".into(),
                name: "Llama 4 70B (Groq)".into(),
                provider: "groq".into(),
                model_family: "llama-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt, Capability::FunctionCalling,
                ],
                context_window: 128_000,
                max_output_tokens: 8_000,
                pricing: ModelPricing { input_per_million: 0.59, output_per_million: 0.79, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(38.0), humaneval_score: Some(82.0), mbpp_score: Some(78.0), arena_elo: Some(1240), mmlu_score: Some(80.0), speed_tokens_per_sec: Some(800.0) },
                release_date: "2025-10-01".into(),
                description: "Ultra-fast Llama inference on Groq LPU hardware".into(),
                supported_features: vec!["fast-inference".into()],
                community_rating: Some(4.4),
                rating_count: 2000,
            },
            // Cerebras-hosted
            ModelEntry {
                id: "cerebras-llama-4-70b".into(),
                name: "Llama 4 70B (Cerebras)".into(),
                provider: "cerebras".into(),
                model_family: "llama-4".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt,
                ],
                context_window: 128_000,
                max_output_tokens: 8_000,
                pricing: ModelPricing { input_per_million: 0.60, output_per_million: 0.60, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(38.0), humaneval_score: Some(82.0), mbpp_score: Some(78.0), arena_elo: Some(1240), mmlu_score: Some(80.0), speed_tokens_per_sec: Some(900.0) },
                release_date: "2025-10-01".into(),
                description: "Fastest inference via Cerebras wafer-scale chips".into(),
                supported_features: vec!["fast-inference".into()],
                community_rating: Some(4.3),
                rating_count: 800,
            },
            // OpenRouter aggregated
            ModelEntry {
                id: "openrouter-auto".into(),
                name: "OpenRouter Auto".into(),
                provider: "openrouter".into(),
                model_family: "auto".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt,
                ],
                context_window: 128_000,
                max_output_tokens: 16_000,
                pricing: ModelPricing { input_per_million: 1.0, output_per_million: 3.0, free_tier: false, currency: "USD".into() },
                benchmarks: ModelBenchmarks::default(),
                release_date: "2025-06-01".into(),
                description: "Auto-routes to best model for each request".into(),
                supported_features: vec!["auto-routing".into(), "fallback".into()],
                community_rating: Some(4.0),
                rating_count: 1500,
            },
            // Ollama local
            ModelEntry {
                id: "ollama-qwen3-32b".into(),
                name: "Qwen 3 32B (Ollama)".into(),
                provider: "ollama".into(),
                model_family: "qwen-3".into(),
                capabilities: vec![
                    Capability::CodeGeneration, Capability::Chat, Capability::Streaming,
                    Capability::SystemPrompt,
                ],
                context_window: 32_768,
                max_output_tokens: 8_000,
                pricing: ModelPricing { input_per_million: 0.0, output_per_million: 0.0, free_tier: true, currency: "USD".into() },
                benchmarks: ModelBenchmarks { swe_bench_score: Some(30.0), humaneval_score: Some(78.0), mbpp_score: Some(74.0), arena_elo: Some(1200), mmlu_score: Some(76.0), speed_tokens_per_sec: Some(25.0) },
                release_date: "2025-09-01".into(),
                description: "Strong local model for privacy-first workflows".into(),
                supported_features: vec!["self-hosted".into(), "offline".into()],
                community_rating: Some(4.1),
                rating_count: 2400,
            },
        ];

        for entry in defaults {
            self.models.insert(entry.id.clone(), entry);
        }
    }

    /// Add a custom model entry to the registry.
    pub fn add_model(&mut self, entry: ModelEntry) -> Result<(), MarketplaceError> {
        if self.models.contains_key(&entry.id) {
            return Err(MarketplaceError::DuplicateModel);
        }
        self.models.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// Look up a model by ID.
    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.get(id)
    }

    /// Return all models sorted by name.
    pub fn list_models(&self) -> Vec<&ModelEntry> {
        let mut entries: Vec<&ModelEntry> = self.models.values().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    /// Search models by name, provider, or description (case-insensitive).
    pub fn search_models(&self, query: &str) -> Vec<&ModelEntry> {
        let q = query.to_lowercase();
        let mut results: Vec<&ModelEntry> = self.models.values().filter(|m| {
            m.name.to_lowercase().contains(&q)
                || m.provider.to_lowercase().contains(&q)
                || m.description.to_lowercase().contains(&q)
                || m.id.to_lowercase().contains(&q)
        }).collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Filter models by multiple criteria.
    pub fn filter_models(&self, filter: &ModelFilter) -> Vec<&ModelEntry> {
        let mut results: Vec<&ModelEntry> = self.models.values().filter(|m| {
            if let Some(ref p) = filter.provider {
                if m.provider.to_lowercase() != p.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref cap) = filter.capability {
                if !m.capabilities.contains(cap) {
                    return false;
                }
            }
            if let Some(max_price) = filter.max_price_input {
                if m.pricing.input_per_million > max_price {
                    return false;
                }
            }
            if let Some(min_ctx) = filter.min_context_window {
                if m.context_window < min_ctx {
                    return false;
                }
            }
            if let Some(min_swe) = filter.min_swe_bench {
                match m.benchmarks.swe_bench_score {
                    Some(s) if s >= min_swe => {},
                    _ => return false,
                }
            }
            if filter.free_only && !m.pricing.free_tier {
                return false;
            }
            true
        }).collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Compare two or more models side-by-side.
    pub fn compare_models(&self, ids: &[&str]) -> Result<ModelComparison, MarketplaceError> {
        let mut entries = Vec::new();
        for id in ids {
            match self.models.get(*id) {
                Some(m) => entries.push(m),
                None => return Err(MarketplaceError::ModelNotFound),
            }
        }

        let mut table = Vec::new();

        table.push(ComparisonRow {
            attribute: "Provider".into(),
            values: entries.iter().map(|m| m.provider.clone()).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Context Window".into(),
            values: entries.iter().map(|m| format!("{}", m.context_window)).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Max Output Tokens".into(),
            values: entries.iter().map(|m| format!("{}", m.max_output_tokens)).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Input $/1M tokens".into(),
            values: entries.iter().map(|m| format!("{:.2}", m.pricing.input_per_million)).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Output $/1M tokens".into(),
            values: entries.iter().map(|m| format!("{:.2}", m.pricing.output_per_million)).collect(),
        });
        table.push(ComparisonRow {
            attribute: "SWE-bench".into(),
            values: entries.iter().map(|m| match m.benchmarks.swe_bench_score {
                Some(s) => format!("{:.1}", s),
                None => "N/A".into(),
            }).collect(),
        });
        table.push(ComparisonRow {
            attribute: "HumanEval".into(),
            values: entries.iter().map(|m| match m.benchmarks.humaneval_score {
                Some(s) => format!("{:.1}", s),
                None => "N/A".into(),
            }).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Arena ELO".into(),
            values: entries.iter().map(|m| match m.benchmarks.arena_elo {
                Some(e) => format!("{}", e),
                None => "N/A".into(),
            }).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Speed (tok/s)".into(),
            values: entries.iter().map(|m| match m.benchmarks.speed_tokens_per_sec {
                Some(s) => format!("{:.0}", s),
                None => "N/A".into(),
            }).collect(),
        });
        table.push(ComparisonRow {
            attribute: "Community Rating".into(),
            values: entries.iter().map(|m| match m.community_rating {
                Some(r) => format!("{:.1}/5.0", r),
                None => "N/A".into(),
            }).collect(),
        });

        // Pick recommendation: highest SWE-bench score among compared
        let best = entries.iter().max_by(|a, b| {
            let sa = a.benchmarks.swe_bench_score.unwrap_or(0.0);
            let sb = b.benchmarks.swe_bench_score.unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let recommendation = match best {
            Some(m) => format!("{} has the highest SWE-bench score among compared models", m.name),
            None => "No clear recommendation".into(),
        };

        Ok(ModelComparison {
            models: ids.iter().map(|s| s.to_string()).collect(),
            comparison_table: table,
            recommendation,
        })
    }

    /// Estimate daily/monthly/yearly cost for a model given token usage.
    pub fn estimate_cost(
        &self,
        model_id: &str,
        daily_input: u64,
        daily_output: u64,
    ) -> Result<CostEstimate, MarketplaceError> {
        let model = self.models.get(model_id).ok_or(MarketplaceError::ModelNotFound)?;
        let daily_cost = (daily_input as f64 / 1_000_000.0) * model.pricing.input_per_million
            + (daily_output as f64 / 1_000_000.0) * model.pricing.output_per_million;
        Ok(CostEstimate {
            model_id: model_id.to_string(),
            daily_tokens_input: daily_input,
            daily_tokens_output: daily_output,
            daily_cost,
            monthly_cost: daily_cost * 30.0,
            yearly_cost: daily_cost * 365.0,
        })
    }

    /// Rate a model (0.0-5.0). Updates the running average.
    pub fn rate_model(&mut self, model_id: &str, rating: f32) -> Result<(), MarketplaceError> {
        if !(0.0..=5.0).contains(&rating) {
            return Err(MarketplaceError::InvalidRating);
        }
        let model = self.models.get_mut(model_id).ok_or(MarketplaceError::ModelNotFound)?;
        let old_rating = model.community_rating.unwrap_or(0.0);
        let old_count = model.rating_count;
        let new_count = old_count + 1;
        let new_rating = (old_rating * old_count as f32 + rating) / new_count as f32;
        model.community_rating = Some(new_rating);
        model.rating_count = new_count;
        Ok(())
    }

    /// Return the top-rated models, sorted by community rating descending.
    pub fn get_top_rated(&self, limit: usize) -> Vec<&ModelEntry> {
        let mut entries: Vec<&ModelEntry> = self.models.values()
            .filter(|m| m.community_rating.is_some())
            .collect();
        entries.sort_by(|a, b| {
            let ra = a.community_rating.unwrap_or(0.0);
            let rb = b.community_rating.unwrap_or(0.0);
            rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        entries
    }

    /// Recommend models for a task type: "code", "chat", "vision", "reasoning", "fast", "cheap".
    pub fn recommend_for_task(&self, task: &str) -> Vec<ModelRecommendation> {
        let task_lower = task.to_lowercase();
        let mut scored: Vec<ModelRecommendation> = self.models.values().filter_map(|m| {
            let (score, reason) = match task_lower.as_str() {
                "code" | "coding" | "code-generation" => {
                    if !m.capabilities.contains(&Capability::CodeGeneration) {
                        return None;
                    }
                    let mut s: f32 = 0.0;
                    if let Some(swe) = m.benchmarks.swe_bench_score { s += swe / 100.0 * 50.0; }
                    if let Some(he) = m.benchmarks.humaneval_score { s += he / 100.0 * 30.0; }
                    if let Some(r) = m.community_rating { s += r * 4.0; }
                    (s, "Strong code generation benchmarks".to_string())
                }
                "chat" | "conversation" => {
                    if !m.capabilities.contains(&Capability::Chat) {
                        return None;
                    }
                    let mut s: f32 = 0.0;
                    if let Some(elo) = m.benchmarks.arena_elo { s += (elo as f32 - 1000.0) / 400.0 * 50.0; }
                    if let Some(r) = m.community_rating { s += r * 10.0; }
                    (s, "High chat quality and community rating".to_string())
                }
                "vision" | "image" | "multimodal" => {
                    if !m.capabilities.contains(&Capability::Vision) {
                        return None;
                    }
                    let mut s: f32 = 50.0;
                    if let Some(r) = m.community_rating { s += r * 10.0; }
                    if m.capabilities.contains(&Capability::MultiModal) { s += 20.0; }
                    (s, "Vision and multimodal capabilities".to_string())
                }
                "reasoning" | "thinking" => {
                    if !m.capabilities.contains(&Capability::ReasoningMode) {
                        return None;
                    }
                    let mut s: f32 = 50.0;
                    if let Some(mmlu) = m.benchmarks.mmlu_score { s += mmlu / 100.0 * 30.0; }
                    if let Some(r) = m.community_rating { s += r * 4.0; }
                    (s, "Extended reasoning capabilities".to_string())
                }
                "fast" | "speed" | "latency" => {
                    let speed = m.benchmarks.speed_tokens_per_sec.unwrap_or(0.0);
                    if speed < 1.0 { return None; }
                    let s = speed / 10.0;
                    (s, format!("{:.0} tokens/sec throughput", speed))
                }
                "cheap" | "budget" | "cost-effective" => {
                    if m.pricing.free_tier {
                        let s = 90.0 + m.community_rating.unwrap_or(0.0) * 2.0;
                        return Some(ModelRecommendation {
                            model_id: m.id.clone(),
                            reason: "Free tier available".to_string(),
                            score: s,
                        });
                    }
                    if m.pricing.input_per_million > 5.0 { return None; }
                    let s = (50.0 - m.pricing.input_per_million * 5.0) as f32 + m.community_rating.unwrap_or(0.0) * 5.0;
                    (s, "Low-cost with good quality".to_string())
                }
                _ => {
                    // General: use a blend of all signals
                    let mut s: f32 = 0.0;
                    if let Some(swe) = m.benchmarks.swe_bench_score { s += swe / 100.0 * 20.0; }
                    if let Some(r) = m.community_rating { s += r * 10.0; }
                    (s, "General-purpose model".to_string())
                }
            };
            Some(ModelRecommendation {
                model_id: m.id.clone(),
                reason,
                score,
            })
        }).collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(5);
        scored
    }

    /// Return the cheapest model with a given capability (by input price).
    pub fn get_cheapest(&self, capability: &Capability) -> Option<&ModelEntry> {
        self.models.values()
            .filter(|m| m.capabilities.contains(capability))
            .min_by(|a, b| {
                a.pricing.input_per_million
                    .partial_cmp(&b.pricing.input_per_million)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Return the fastest model with a given capability (by tokens/sec).
    pub fn get_fastest(&self, capability: &Capability) -> Option<&ModelEntry> {
        self.models.values()
            .filter(|m| m.capabilities.contains(capability) && m.benchmarks.speed_tokens_per_sec.is_some())
            .max_by(|a, b| {
                let sa = a.benchmarks.speed_tokens_per_sec.unwrap_or(0.0);
                let sb = b.benchmarks.speed_tokens_per_sec.unwrap_or(0.0);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Generate a TOML config snippet for a model.
    pub fn generate_config_snippet(&self, model_id: &str) -> Result<String, MarketplaceError> {
        let model = self.models.get(model_id).ok_or(MarketplaceError::ModelNotFound)?;
        let env_var = match model.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "google" => "GEMINI_API_KEY",
            "xai" => "GROK_API_KEY",
            "groq" => "GROQ_API_KEY",
            "mistral" => "MISTRAL_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "cerebras" => "CEREBRAS_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "ollama" => "# No API key needed for Ollama (local)",
            _ => "API_KEY",
        };

        Ok(format!(
            r#"# VibeCody config for {name}
# Pricing: ${input}/1M input, ${output}/1M output tokens
# Context window: {ctx} tokens

[provider]
name = "{provider}"
model = "{id}"

[provider.env]
# {env_var}
"#,
            name = model.name,
            input = model.pricing.input_per_million,
            output = model.pricing.output_per_million,
            ctx = model.context_window,
            provider = model.provider,
            id = model.id,
            env_var = env_var,
        ))
    }

    /// Total number of models in the registry.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn marketplace() -> ModelMarketplace {
        ModelMarketplace::new(MarketplaceConfig::default())
    }

    #[test]
    fn test_default_config() {
        let cfg = MarketplaceConfig::default();
        assert_eq!(cfg.cache_dir, ".vibecody/models");
        assert!(cfg.auto_update);
    }

    #[test]
    fn test_new_loads_defaults() {
        let mp = marketplace();
        assert!(mp.model_count() >= 20);
    }

    #[test]
    fn test_get_model_found() {
        let mp = marketplace();
        let m = mp.get_model("claude-opus-4.6").expect("should exist");
        assert_eq!(m.provider, "anthropic");
        assert_eq!(m.context_window, 1_000_000);
    }

    #[test]
    fn test_get_model_not_found() {
        let mp = marketplace();
        assert!(mp.get_model("nonexistent-model").is_none());
    }

    #[test]
    fn test_list_models_sorted() {
        let mp = marketplace();
        let list = mp.list_models();
        for i in 1..list.len() {
            assert!(list[i - 1].name <= list[i].name);
        }
    }

    #[test]
    fn test_add_model_success() {
        let mut mp = marketplace();
        let count_before = mp.model_count();
        let entry = ModelEntry {
            id: "custom-model-1".into(),
            name: "Custom Model".into(),
            provider: "custom".into(),
            model_family: "custom".into(),
            capabilities: vec![Capability::Chat],
            context_window: 4096,
            max_output_tokens: 2048,
            pricing: ModelPricing::default(),
            benchmarks: ModelBenchmarks::default(),
            release_date: "2026-03-01".into(),
            description: "Test model".into(),
            supported_features: vec![],
            community_rating: None,
            rating_count: 0,
        };
        mp.add_model(entry).expect("should succeed");
        assert_eq!(mp.model_count(), count_before + 1);
    }

    #[test]
    fn test_add_model_duplicate() {
        let mut mp = marketplace();
        let entry = ModelEntry {
            id: "claude-opus-4.6".into(),
            name: "Dup".into(),
            provider: "test".into(),
            model_family: "test".into(),
            capabilities: vec![],
            context_window: 0,
            max_output_tokens: 0,
            pricing: ModelPricing::default(),
            benchmarks: ModelBenchmarks::default(),
            release_date: "".into(),
            description: "".into(),
            supported_features: vec![],
            community_rating: None,
            rating_count: 0,
        };
        assert_eq!(mp.add_model(entry), Err(MarketplaceError::DuplicateModel));
    }

    #[test]
    fn test_search_by_provider() {
        let mp = marketplace();
        let results = mp.search_models("anthropic");
        assert!(results.len() >= 3);
        for m in &results {
            assert_eq!(m.provider, "anthropic");
        }
    }

    #[test]
    fn test_search_by_name() {
        let mp = marketplace();
        let results = mp.search_models("GPT");
        assert!(!results.is_empty());
        for m in &results {
            assert!(m.name.to_lowercase().contains("gpt") || m.id.to_lowercase().contains("gpt"));
        }
    }

    #[test]
    fn test_search_case_insensitive() {
        let mp = marketplace();
        let r1 = mp.search_models("claude");
        let r2 = mp.search_models("CLAUDE");
        assert_eq!(r1.len(), r2.len());
    }

    #[test]
    fn test_search_no_results() {
        let mp = marketplace();
        let results = mp.search_models("zzz_nonexistent_zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_by_provider() {
        let mp = marketplace();
        let filter = ModelFilter { provider: Some("google".into()), ..Default::default() };
        let results = mp.filter_models(&filter);
        assert!(results.len() >= 2);
        for m in &results {
            assert_eq!(m.provider, "google");
        }
    }

    #[test]
    fn test_filter_by_capability() {
        let mp = marketplace();
        let filter = ModelFilter { capability: Some(Capability::Vision), ..Default::default() };
        let results = mp.filter_models(&filter);
        assert!(!results.is_empty());
        for m in &results {
            assert!(m.capabilities.contains(&Capability::Vision));
        }
    }

    #[test]
    fn test_filter_by_max_price() {
        let mp = marketplace();
        let filter = ModelFilter { max_price_input: Some(1.0), ..Default::default() };
        let results = mp.filter_models(&filter);
        for m in &results {
            assert!(m.pricing.input_per_million <= 1.0);
        }
    }

    #[test]
    fn test_filter_by_min_context() {
        let mp = marketplace();
        let filter = ModelFilter { min_context_window: Some(500_000), ..Default::default() };
        let results = mp.filter_models(&filter);
        assert!(!results.is_empty());
        for m in &results {
            assert!(m.context_window >= 500_000);
        }
    }

    #[test]
    fn test_filter_free_only() {
        let mp = marketplace();
        let filter = ModelFilter { free_only: true, ..Default::default() };
        let results = mp.filter_models(&filter);
        assert!(!results.is_empty());
        for m in &results {
            assert!(m.pricing.free_tier);
        }
    }

    #[test]
    fn test_filter_by_min_swe_bench() {
        let mp = marketplace();
        let filter = ModelFilter { min_swe_bench: Some(60.0), ..Default::default() };
        let results = mp.filter_models(&filter);
        assert!(!results.is_empty());
        for m in &results {
            assert!(m.benchmarks.swe_bench_score.unwrap() >= 60.0);
        }
    }

    #[test]
    fn test_filter_combined() {
        let mp = marketplace();
        let filter = ModelFilter {
            capability: Some(Capability::CodeGeneration),
            max_price_input: Some(5.0),
            min_swe_bench: Some(40.0),
            ..Default::default()
        };
        let results = mp.filter_models(&filter);
        for m in &results {
            assert!(m.capabilities.contains(&Capability::CodeGeneration));
            assert!(m.pricing.input_per_million <= 5.0);
            assert!(m.benchmarks.swe_bench_score.unwrap() >= 40.0);
        }
    }

    #[test]
    fn test_compare_models_success() {
        let mp = marketplace();
        let cmp = mp.compare_models(&["claude-opus-4.6", "gpt-5.4"]).expect("should work");
        assert_eq!(cmp.models.len(), 2);
        assert!(!cmp.comparison_table.is_empty());
        assert!(!cmp.recommendation.is_empty());
    }

    #[test]
    fn test_compare_models_not_found() {
        let mp = marketplace();
        let result = mp.compare_models(&["claude-opus-4.6", "nonexistent"]);
        assert_eq!(result, Err(MarketplaceError::ModelNotFound));
    }

    #[test]
    fn test_compare_table_rows() {
        let mp = marketplace();
        let cmp = mp.compare_models(&["claude-sonnet-4.5", "gemini-2.5-pro"]).unwrap();
        let attrs: Vec<&str> = cmp.comparison_table.iter().map(|r| r.attribute.as_str()).collect();
        assert!(attrs.contains(&"Provider"));
        assert!(attrs.contains(&"Context Window"));
        assert!(attrs.contains(&"SWE-bench"));
        // Each row should have exactly 2 values
        for row in &cmp.comparison_table {
            assert_eq!(row.values.len(), 2);
        }
    }

    #[test]
    fn test_estimate_cost_basic() {
        let mp = marketplace();
        // Claude Opus: $15/1M input, $75/1M output
        let est = mp.estimate_cost("claude-opus-4.6", 1_000_000, 200_000).unwrap();
        assert_eq!(est.model_id, "claude-opus-4.6");
        assert!((est.daily_cost - 30.0).abs() < 0.01); // 15 + 75*0.2 = 30
        assert!((est.monthly_cost - 900.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_free_model() {
        let mp = marketplace();
        let est = mp.estimate_cost("llama-4-405b", 5_000_000, 1_000_000).unwrap();
        assert!((est.daily_cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_estimate_cost_not_found() {
        let mp = marketplace();
        assert_eq!(mp.estimate_cost("nope", 100, 100), Err(MarketplaceError::ModelNotFound));
    }

    #[test]
    fn test_rate_model_valid() {
        let mut mp = marketplace();
        let old_count = mp.get_model("gpt-4o").unwrap().rating_count;
        mp.rate_model("gpt-4o", 4.0).unwrap();
        let m = mp.get_model("gpt-4o").unwrap();
        assert_eq!(m.rating_count, old_count + 1);
        assert!(m.community_rating.is_some());
    }

    #[test]
    fn test_rate_model_invalid_too_high() {
        let mut mp = marketplace();
        assert_eq!(mp.rate_model("gpt-4o", 5.1), Err(MarketplaceError::InvalidRating));
    }

    #[test]
    fn test_rate_model_invalid_negative() {
        let mut mp = marketplace();
        assert_eq!(mp.rate_model("gpt-4o", -0.1), Err(MarketplaceError::InvalidRating));
    }

    #[test]
    fn test_rate_model_not_found() {
        let mut mp = marketplace();
        assert_eq!(mp.rate_model("nope", 3.0), Err(MarketplaceError::ModelNotFound));
    }

    #[test]
    fn test_get_top_rated() {
        let mp = marketplace();
        let top = mp.get_top_rated(3);
        assert_eq!(top.len(), 3);
        // Should be descending by rating
        for i in 1..top.len() {
            assert!(top[i - 1].community_rating.unwrap() >= top[i].community_rating.unwrap());
        }
    }

    #[test]
    fn test_get_top_rated_limit() {
        let mp = marketplace();
        let top = mp.get_top_rated(1);
        assert_eq!(top.len(), 1);
    }

    #[test]
    fn test_recommend_for_code() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("code");
        assert!(!recs.is_empty());
        assert!(recs.len() <= 5);
        // Should be sorted by score descending
        for i in 1..recs.len() {
            assert!(recs[i - 1].score >= recs[i].score);
        }
    }

    #[test]
    fn test_recommend_for_chat() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("chat");
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_recommend_for_vision() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("vision");
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_recommend_for_fast() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("fast");
        assert!(!recs.is_empty());
        // Top recommendation should be a fast model
        assert!(recs[0].score > 0.0);
    }

    #[test]
    fn test_recommend_for_cheap() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("cheap");
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_recommend_for_reasoning() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("reasoning");
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_recommend_unknown_task() {
        let mp = marketplace();
        let recs = mp.recommend_for_task("unknown_xyz");
        // Should still return general recommendations
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_get_cheapest() {
        let mp = marketplace();
        let cheapest = mp.get_cheapest(&Capability::CodeGeneration).expect("should find one");
        // Free models (price 0.0) should win
        assert_eq!(cheapest.pricing.input_per_million, 0.0);
    }

    #[test]
    fn test_get_fastest() {
        let mp = marketplace();
        let fastest = mp.get_fastest(&Capability::CodeGeneration).expect("should find one");
        assert!(fastest.benchmarks.speed_tokens_per_sec.unwrap() >= 100.0);
    }

    #[test]
    fn test_generate_config_snippet_anthropic() {
        let mp = marketplace();
        let snippet = mp.generate_config_snippet("claude-opus-4.6").unwrap();
        assert!(snippet.contains("anthropic"));
        assert!(snippet.contains("ANTHROPIC_API_KEY"));
        assert!(snippet.contains("claude-opus-4.6"));
    }

    #[test]
    fn test_generate_config_snippet_ollama() {
        let mp = marketplace();
        let snippet = mp.generate_config_snippet("ollama-qwen3-32b").unwrap();
        assert!(snippet.contains("ollama"));
        assert!(snippet.contains("No API key needed"));
    }

    #[test]
    fn test_generate_config_snippet_not_found() {
        let mp = marketplace();
        assert_eq!(mp.generate_config_snippet("nope"), Err(MarketplaceError::ModelNotFound));
    }

    #[test]
    fn test_model_count() {
        let mp = marketplace();
        assert_eq!(mp.model_count(), mp.list_models().len());
    }

    #[test]
    fn test_capability_display() {
        assert_eq!(format!("{}", Capability::CodeGeneration), "code-generation");
        assert_eq!(format!("{}", Capability::JsonMode), "json-mode");
        assert_eq!(format!("{}", Capability::ReasoningMode), "reasoning-mode");
    }

    #[test]
    fn test_error_display() {
        assert_eq!(format!("{}", MarketplaceError::ModelNotFound), "model not found");
        assert_eq!(format!("{}", MarketplaceError::DuplicateModel), "duplicate model");
    }

    #[test]
    fn test_pricing_default() {
        let p = ModelPricing::default();
        assert_eq!(p.input_per_million, 0.0);
        assert_eq!(p.currency, "USD");
        assert!(!p.free_tier);
    }

    #[test]
    fn test_model_entry_capabilities() {
        let mp = marketplace();
        let opus = mp.get_model("claude-opus-4.6").unwrap();
        assert!(opus.capabilities.contains(&Capability::CodeGeneration));
        assert!(opus.capabilities.contains(&Capability::Vision));
        assert!(opus.capabilities.contains(&Capability::ReasoningMode));
    }
}
