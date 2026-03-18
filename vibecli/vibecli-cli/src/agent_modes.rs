use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The three agent modes matching Amp's Smart/Rush/Deep paradigm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentMode {
    /// Balanced speed and quality using the best available model (Claude Opus).
    #[default]
    Smart,
    /// Optimized for speed using the fastest model (Haiku). Best for simple tasks.
    Rush,
    /// Maximum capability with extended thinking. Best for complex multi-file tasks.
    Deep,
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Smart => write!(f, "Smart"),
            AgentMode::Rush => write!(f, "Rush"),
            AgentMode::Deep => write!(f, "Deep"),
        }
    }
}


/// Task complexity level used for automatic mode selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskComplexity {
    /// Simple tasks: typo fixes, renames, formatting.
    Simple,
    /// Moderate tasks: single-file edits, small features.
    Moderate,
    /// Complex tasks: multi-file refactors, architecture, debugging.
    Complex,
}

/// Configuration for a specific agent mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Model identifier (e.g., "claude-opus-4-20250514", "claude-3-5-haiku-20241022").
    pub model_id: String,
    /// Maximum output tokens for the model.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// Maximum agent loop turns before stopping.
    pub max_turns: u32,
    /// Token budget for extended thinking (0 = disabled).
    pub thinking_budget: u32,
    /// Human-readable description of this mode.
    pub description: String,
}

impl ModeConfig {
    /// Default configuration for Smart mode.
    pub fn default_smart() -> Self {
        Self {
            model_id: "claude-opus-4-20250514".to_string(),
            max_tokens: 16384,
            temperature: 0.7,
            max_turns: 20,
            thinking_budget: 0,
            description: "Balanced speed and quality using Claude Opus".to_string(),
        }
    }

    /// Default configuration for Rush mode.
    pub fn default_rush() -> Self {
        Self {
            model_id: "claude-3-5-haiku-20241022".to_string(),
            max_tokens: 4096,
            temperature: 0.3,
            max_turns: 8,
            thinking_budget: 0,
            description: "Fast responses using Claude Haiku for simple tasks".to_string(),
        }
    }

    /// Default configuration for Deep mode.
    pub fn default_deep() -> Self {
        Self {
            model_id: "claude-opus-4-20250514".to_string(),
            max_tokens: 32768,
            temperature: 0.5,
            max_turns: 50,
            thinking_budget: 10000,
            description: "Extended thinking with Claude Opus for complex tasks".to_string(),
        }
    }
}

/// Tracks usage statistics for a mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeUsageStats {
    /// Number of times this mode has been invoked.
    pub invocation_count: u64,
    /// Total tokens consumed across all invocations.
    pub total_tokens: u64,
}

impl ModeUsageStats {
    pub fn new() -> Self {
        Self {
            invocation_count: 0,
            total_tokens: 0,
        }
    }

    /// Record an invocation with the given token count.
    pub fn record(&mut self, tokens: u64) {
        self.invocation_count += 1;
        self.total_tokens += tokens;
    }

    /// Average tokens per invocation, or 0 if no invocations.
    pub fn avg_tokens(&self) -> u64 {
        if self.invocation_count == 0 {
            0
        } else {
            self.total_tokens / self.invocation_count
        }
    }
}

impl Default for ModeUsageStats {
    fn default() -> Self {
        Self::new()
    }
}

/// A user profile with a preferred mode and optional custom configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeProfile {
    /// Profile name.
    pub name: String,
    /// Preferred default mode for this profile.
    pub preferred_mode: AgentMode,
    /// Custom mode configurations that override defaults.
    pub custom_configs: HashMap<AgentMode, ModeConfig>,
}

impl ModeProfile {
    pub fn new(name: impl Into<String>, preferred_mode: AgentMode) -> Self {
        Self {
            name: name.into(),
            preferred_mode,
            custom_configs: HashMap::new(),
        }
    }

    /// Set a custom config for a specific mode.
    pub fn set_config(&mut self, mode: AgentMode, config: ModeConfig) {
        self.custom_configs.insert(mode, config);
    }
}

/// Complexity signals extracted from a task description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexitySignals {
    /// Estimated token count of the input.
    pub token_count: usize,
    /// Number of files mentioned or involved.
    pub file_count: usize,
    /// Whether the task involves a question (vs. a command).
    pub is_question: bool,
    /// Raw task text for keyword analysis.
    pub task_text: String,
}

/// Keywords that signal high complexity (Deep mode).
const DEEP_KEYWORDS: &[&str] = &[
    "refactor",
    "architect",
    "debug complex",
    "redesign",
    "migrate",
    "optimize performance",
    "implement feature",
    "multi-file",
    "cross-module",
    "system design",
    "rewrite",
    "overhaul",
];

/// Keywords that signal low complexity (Rush mode).
const RUSH_KEYWORDS: &[&str] = &[
    "fix typo",
    "rename",
    "format",
    "add comment",
    "remove unused",
    "update version",
    "simple",
    "quick",
    "trivial",
    "lint",
    "spelling",
    "whitespace",
];

/// Selects the appropriate mode based on task complexity signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeSelector;

impl ModeSelector {
    /// Estimate task complexity from signals.
    pub fn estimate_complexity(signals: &ComplexitySignals) -> TaskComplexity {
        let text_lower = signals.task_text.to_lowercase();

        // Keyword analysis takes priority
        if Self::matches_keywords(&text_lower, DEEP_KEYWORDS) {
            return TaskComplexity::Complex;
        }
        if Self::matches_keywords(&text_lower, RUSH_KEYWORDS) {
            return TaskComplexity::Simple;
        }

        // Heuristic scoring based on quantitative signals
        let mut score: i32 = 0;

        // Token count contribution
        match signals.token_count {
            0..=50 => score -= 1,
            51..=200 => {}
            201..=500 => score += 1,
            _ => score += 2,
        }

        // File count contribution
        match signals.file_count {
            0..=1 => score -= 1,
            2..=3 => {}
            _ => score += 2,
        }

        // Questions are typically moderate
        if signals.is_question {
            score -= 1;
        }

        match score {
            i32::MIN..=-1 => TaskComplexity::Simple,
            0..=1 => TaskComplexity::Moderate,
            _ => TaskComplexity::Complex,
        }
    }

    /// Map complexity to the recommended mode.
    pub fn mode_for_complexity(complexity: TaskComplexity) -> AgentMode {
        match complexity {
            TaskComplexity::Simple => AgentMode::Rush,
            TaskComplexity::Moderate => AgentMode::Smart,
            TaskComplexity::Complex => AgentMode::Deep,
        }
    }

    /// Auto-select mode from signals.
    pub fn auto_select(signals: &ComplexitySignals) -> AgentMode {
        let complexity = Self::estimate_complexity(signals);
        Self::mode_for_complexity(complexity)
    }

    fn matches_keywords(text: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|kw| text.contains(kw))
    }
}

/// Central router that manages modes, configs, profiles, and usage stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeRouter {
    /// Registered mode configurations.
    configs: HashMap<AgentMode, ModeConfig>,
    /// Usage statistics per mode.
    stats: HashMap<AgentMode, ModeUsageStats>,
    /// Optional manual override.
    mode_override: Option<AgentMode>,
    /// Active profile (if any).
    active_profile: Option<ModeProfile>,
}

impl ModeRouter {
    /// Create a new router with default configs for all three modes.
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        configs.insert(AgentMode::Smart, ModeConfig::default_smart());
        configs.insert(AgentMode::Rush, ModeConfig::default_rush());
        configs.insert(AgentMode::Deep, ModeConfig::default_deep());

        let mut stats = HashMap::new();
        stats.insert(AgentMode::Smart, ModeUsageStats::new());
        stats.insert(AgentMode::Rush, ModeUsageStats::new());
        stats.insert(AgentMode::Deep, ModeUsageStats::new());

        Self {
            configs,
            stats,
            mode_override: None,
            active_profile: None,
        }
    }

    /// Register or update a mode configuration.
    pub fn register_mode(&mut self, mode: AgentMode, config: ModeConfig) {
        self.configs.insert(mode, config);
        self.stats.entry(mode).or_default();
    }

    /// Set a manual mode override. Pass `None` to clear.
    pub fn set_override(&mut self, mode: Option<AgentMode>) {
        self.mode_override = mode;
    }

    /// Get the current override, if any.
    pub fn get_override(&self) -> Option<AgentMode> {
        self.mode_override
    }

    /// Set the active profile.
    pub fn set_profile(&mut self, profile: ModeProfile) {
        // Apply profile's custom configs
        for (mode, config) in &profile.custom_configs {
            self.configs.insert(*mode, config.clone());
        }
        self.active_profile = Some(profile);
    }

    /// Get the active profile.
    pub fn get_profile(&self) -> Option<&ModeProfile> {
        self.active_profile.as_ref()
    }

    /// Select the mode for a task. Manual override takes priority, then profile
    /// preference for moderate complexity, then auto-selection.
    pub fn select_mode(&self, signals: &ComplexitySignals) -> AgentMode {
        // Manual override always wins
        if let Some(mode) = self.mode_override {
            return mode;
        }

        // Auto-select based on complexity
        let auto = ModeSelector::auto_select(signals);

        // If a profile is active and auto selected Smart (moderate), prefer the
        // profile's preferred mode instead.
        if let Some(profile) = &self.active_profile {
            if auto == AgentMode::Smart {
                return profile.preferred_mode;
            }
        }

        auto
    }

    /// Get the config for a mode.
    pub fn get_config(&self, mode: AgentMode) -> Option<&ModeConfig> {
        self.configs.get(&mode)
    }

    /// Record a mode invocation with token usage.
    pub fn record_usage(&mut self, mode: AgentMode, tokens: u64) {
        self.stats
            .entry(mode)
            .or_default()
            .record(tokens);
    }

    /// Get usage stats for a mode.
    pub fn get_stats(&self, mode: AgentMode) -> Option<&ModeUsageStats> {
        self.stats.get(&mode)
    }

    /// Get all usage stats.
    pub fn all_stats(&self) -> &HashMap<AgentMode, ModeUsageStats> {
        &self.stats
    }

    /// List all registered modes.
    pub fn registered_modes(&self) -> Vec<AgentMode> {
        self.configs.keys().copied().collect()
    }
}

impl Default for ModeRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AgentMode basics ---

    #[test]
    fn test_agent_mode_default_is_smart() {
        assert_eq!(AgentMode::default(), AgentMode::Smart);
    }

    #[test]
    fn test_agent_mode_display() {
        assert_eq!(format!("{}", AgentMode::Smart), "Smart");
        assert_eq!(format!("{}", AgentMode::Rush), "Rush");
        assert_eq!(format!("{}", AgentMode::Deep), "Deep");
    }

    #[test]
    fn test_agent_mode_equality() {
        assert_eq!(AgentMode::Smart, AgentMode::Smart);
        assert_ne!(AgentMode::Rush, AgentMode::Deep);
    }

    // --- ModeConfig defaults ---

    #[test]
    fn test_default_smart_config() {
        let cfg = ModeConfig::default_smart();
        assert!(cfg.model_id.contains("opus"));
        assert_eq!(cfg.max_tokens, 16384);
        assert_eq!(cfg.thinking_budget, 0);
        assert!(cfg.max_turns > 0);
    }

    #[test]
    fn test_default_rush_config() {
        let cfg = ModeConfig::default_rush();
        assert!(cfg.model_id.contains("haiku"));
        assert_eq!(cfg.max_tokens, 4096);
        assert!(cfg.temperature < 0.5);
        assert!(cfg.max_turns < 10);
    }

    #[test]
    fn test_default_deep_config() {
        let cfg = ModeConfig::default_deep();
        assert!(cfg.model_id.contains("opus"));
        assert_eq!(cfg.max_tokens, 32768);
        assert!(cfg.thinking_budget > 0);
        assert!(cfg.max_turns >= 50);
    }

    // --- Complexity estimation ---

    #[test]
    fn test_deep_keyword_triggers_complex() {
        let signals = ComplexitySignals {
            token_count: 10,
            file_count: 1,
            is_question: false,
            task_text: "refactor the authentication module".to_string(),
        };
        assert_eq!(
            ModeSelector::estimate_complexity(&signals),
            TaskComplexity::Complex
        );
    }

    #[test]
    fn test_rush_keyword_triggers_simple() {
        let signals = ComplexitySignals {
            token_count: 500,
            file_count: 5,
            is_question: false,
            task_text: "fix typo in readme".to_string(),
        };
        assert_eq!(
            ModeSelector::estimate_complexity(&signals),
            TaskComplexity::Simple
        );
    }

    #[test]
    fn test_high_token_and_file_count_is_complex() {
        let signals = ComplexitySignals {
            token_count: 1000,
            file_count: 10,
            is_question: false,
            task_text: "update the codebase".to_string(),
        };
        assert_eq!(
            ModeSelector::estimate_complexity(&signals),
            TaskComplexity::Complex
        );
    }

    #[test]
    fn test_low_signals_is_simple() {
        let signals = ComplexitySignals {
            token_count: 20,
            file_count: 0,
            is_question: true,
            task_text: "what does this do".to_string(),
        };
        assert_eq!(
            ModeSelector::estimate_complexity(&signals),
            TaskComplexity::Simple
        );
    }

    #[test]
    fn test_moderate_signals() {
        let signals = ComplexitySignals {
            token_count: 100,
            file_count: 2,
            is_question: false,
            task_text: "add error handling to the parser".to_string(),
        };
        assert_eq!(
            ModeSelector::estimate_complexity(&signals),
            TaskComplexity::Moderate
        );
    }

    // --- Mode selection ---

    #[test]
    fn test_auto_select_rush_for_simple() {
        let signals = ComplexitySignals {
            token_count: 10,
            file_count: 1,
            is_question: true,
            task_text: "rename variable x to count".to_string(),
        };
        assert_eq!(ModeSelector::auto_select(&signals), AgentMode::Rush);
    }

    #[test]
    fn test_auto_select_deep_for_complex() {
        let signals = ComplexitySignals {
            token_count: 500,
            file_count: 8,
            is_question: false,
            task_text: "architect a new microservice".to_string(),
        };
        assert_eq!(ModeSelector::auto_select(&signals), AgentMode::Deep);
    }

    #[test]
    fn test_auto_select_smart_for_moderate() {
        let signals = ComplexitySignals {
            token_count: 100,
            file_count: 2,
            is_question: false,
            task_text: "add error handling to the parser".to_string(),
        };
        assert_eq!(ModeSelector::auto_select(&signals), AgentMode::Smart);
    }

    // --- ModeRouter ---

    #[test]
    fn test_router_has_all_modes() {
        let router = ModeRouter::new();
        let modes = router.registered_modes();
        assert!(modes.contains(&AgentMode::Smart));
        assert!(modes.contains(&AgentMode::Rush));
        assert!(modes.contains(&AgentMode::Deep));
    }

    #[test]
    fn test_router_get_config() {
        let router = ModeRouter::new();
        let cfg = router.get_config(AgentMode::Rush).expect("Rush config");
        assert!(cfg.model_id.contains("haiku"));
    }

    #[test]
    fn test_router_manual_override() {
        let mut router = ModeRouter::new();
        router.set_override(Some(AgentMode::Deep));
        let signals = ComplexitySignals {
            token_count: 5,
            file_count: 0,
            is_question: true,
            task_text: "fix typo".to_string(),
        };
        // Override forces Deep even for simple task
        assert_eq!(router.select_mode(&signals), AgentMode::Deep);
    }

    #[test]
    fn test_router_clear_override() {
        let mut router = ModeRouter::new();
        router.set_override(Some(AgentMode::Deep));
        router.set_override(None);
        assert!(router.get_override().is_none());
    }

    #[test]
    fn test_router_register_custom_mode() {
        let mut router = ModeRouter::new();
        let custom = ModeConfig {
            model_id: "custom-model".to_string(),
            max_tokens: 8192,
            temperature: 0.9,
            max_turns: 15,
            thinking_budget: 5000,
            description: "Custom Smart".to_string(),
        };
        router.register_mode(AgentMode::Smart, custom);
        let cfg = router.get_config(AgentMode::Smart).expect("config");
        assert_eq!(cfg.model_id, "custom-model");
    }

    // --- Usage stats ---

    #[test]
    fn test_usage_stats_recording() {
        let mut router = ModeRouter::new();
        router.record_usage(AgentMode::Smart, 100);
        router.record_usage(AgentMode::Smart, 200);
        let stats = router.get_stats(AgentMode::Smart).expect("stats");
        assert_eq!(stats.invocation_count, 2);
        assert_eq!(stats.total_tokens, 300);
        assert_eq!(stats.avg_tokens(), 150);
    }

    #[test]
    fn test_usage_stats_zero_avg() {
        let stats = ModeUsageStats::new();
        assert_eq!(stats.avg_tokens(), 0);
    }

    #[test]
    fn test_all_stats() {
        let mut router = ModeRouter::new();
        router.record_usage(AgentMode::Rush, 50);
        let all = router.all_stats();
        assert_eq!(all.get(&AgentMode::Rush).expect("rush").invocation_count, 1);
    }

    // --- Profiles ---

    #[test]
    fn test_profile_preferred_mode() {
        let profile = ModeProfile::new("speed-freak", AgentMode::Rush);
        assert_eq!(profile.preferred_mode, AgentMode::Rush);
        assert_eq!(profile.name, "speed-freak");
    }

    #[test]
    fn test_profile_custom_config() {
        let mut profile = ModeProfile::new("custom", AgentMode::Smart);
        let cfg = ModeConfig {
            model_id: "my-model".to_string(),
            max_tokens: 2048,
            temperature: 0.1,
            max_turns: 5,
            thinking_budget: 0,
            description: "Minimal".to_string(),
        };
        profile.set_config(AgentMode::Rush, cfg);
        assert!(profile.custom_configs.contains_key(&AgentMode::Rush));
    }

    #[test]
    fn test_profile_influences_moderate_selection() {
        let mut router = ModeRouter::new();
        let profile = ModeProfile::new("deep-thinker", AgentMode::Deep);
        router.set_profile(profile);

        // Moderate task would normally be Smart, but profile prefers Deep
        let signals = ComplexitySignals {
            token_count: 100,
            file_count: 2,
            is_question: false,
            task_text: "add error handling to the parser".to_string(),
        };
        assert_eq!(router.select_mode(&signals), AgentMode::Deep);
    }

    #[test]
    fn test_profile_does_not_override_explicit_complexity() {
        let mut router = ModeRouter::new();
        let profile = ModeProfile::new("speed", AgentMode::Rush);
        router.set_profile(profile);

        // Complex task should still go Deep regardless of profile
        let signals = ComplexitySignals {
            token_count: 500,
            file_count: 8,
            is_question: false,
            task_text: "architect a new microservice".to_string(),
        };
        assert_eq!(router.select_mode(&signals), AgentMode::Deep);
    }

    #[test]
    fn test_profile_applies_custom_configs() {
        let mut router = ModeRouter::new();
        let mut profile = ModeProfile::new("custom", AgentMode::Smart);
        profile.set_config(
            AgentMode::Rush,
            ModeConfig {
                model_id: "profile-haiku".to_string(),
                max_tokens: 1024,
                temperature: 0.2,
                max_turns: 3,
                thinking_budget: 0,
                description: "Profile Rush".to_string(),
            },
        );
        router.set_profile(profile);
        let cfg = router.get_config(AgentMode::Rush).expect("config");
        assert_eq!(cfg.model_id, "profile-haiku");
    }

    // --- Serialization round-trip ---

    #[test]
    fn test_mode_serialization_roundtrip() {
        let mode = AgentMode::Deep;
        let json = serde_json::to_string(&mode).expect("serialize");
        let deserialized: AgentMode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let cfg = ModeConfig::default_smart();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let deserialized: ModeConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.model_id, cfg.model_id);
        assert_eq!(deserialized.max_tokens, cfg.max_tokens);
    }
}
