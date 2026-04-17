//! 6-level thinking abstraction with per-level token budgets.
//! Pi-mono gap bridge: Phase B5.
//!
//! Supports `--model sonnet:high` shorthand and works across all providers
//! that support extended reasoning (Claude, o1/o3, Gemini thinking).

use std::collections::HashMap;

// ── ThinkingLevel ─────────────────────────────────────────────────────────────

/// The six thinking intensity levels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ThinkingLevel {
    #[default]
    Off,     // no thinking tokens
    Minimal, // ~200 tokens
    Low,     // ~1_000 tokens
    Medium,  // ~5_000 tokens
    High,    // ~10_000 tokens
    XHigh,   // ~32_000 tokens
}

impl ThinkingLevel {
    /// Per-level token budget.
    pub fn token_budget(&self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Minimal => 200,
            Self::Low => 1_000,
            Self::Medium => 5_000,
            Self::High => 10_000,
            Self::XHigh => 32_000,
        }
    }

    /// Canonical string identifier.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
        }
    }

    /// Parse from string — case-insensitive, also accepts "x-high" and "extra-high".
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "off" | "none" | "disabled" => Some(Self::Off),
            "minimal" | "min" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" | "x-high" | "extra-high" | "xl" => Some(Self::XHigh),
            _ => None,
        }
    }

    /// Returns `true` for every level except `Off`.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Step up one level; returns `None` from `XHigh`.
    pub fn next_level(&self) -> Option<Self> {
        match self {
            Self::Off => Some(Self::Minimal),
            Self::Minimal => Some(Self::Low),
            Self::Low => Some(Self::Medium),
            Self::Medium => Some(Self::High),
            Self::High => Some(Self::XHigh),
            Self::XHigh => None,
        }
    }

    /// Step down one level; returns `None` from `Off`.
    pub fn prev_level(&self) -> Option<Self> {
        match self {
            Self::Off => None,
            Self::Minimal => Some(Self::Off),
            Self::Low => Some(Self::Minimal),
            Self::Medium => Some(Self::Low),
            Self::High => Some(Self::Medium),
            Self::XHigh => Some(Self::High),
        }
    }

    /// Auto-select a sensible level based on the task type.
    pub fn default_for_task(task: &TaskHint) -> Self {
        match task {
            TaskHint::SimpleEdit => Self::Minimal,
            TaskHint::CodeGeneration => Self::Low,
            TaskHint::Debugging => Self::Medium,
            TaskHint::Architecture => Self::High,
            TaskHint::ComplexReasoning => Self::XHigh,
            TaskHint::Unknown => Self::Low,
        }
    }
}

// ── TaskHint ──────────────────────────────────────────────────────────────────

/// Hint about the task being performed, for auto-level selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskHint {
    SimpleEdit,       // → Minimal
    CodeGeneration,   // → Low
    Debugging,        // → Medium
    Architecture,     // → High
    ComplexReasoning, // → XHigh
    Unknown,          // → Low
}

// ── ThinkingConfig ────────────────────────────────────────────────────────────

/// Resolved thinking configuration to pass to a provider.
#[derive(Debug, Clone)]
pub struct ThinkingConfig {
    pub level: ThinkingLevel,
    pub token_budget: u32,
    pub enabled: bool,
    /// Provider-specific parameter name (e.g. `"reasoning_effort"` for OpenAI).
    pub provider_param: Option<String>,
}

impl ThinkingConfig {
    /// Build a config from a level using that level's default budget.
    pub fn for_level(level: ThinkingLevel) -> Self {
        let token_budget = level.token_budget();
        let enabled = level.is_enabled();
        Self {
            level,
            token_budget,
            enabled,
            provider_param: None,
        }
    }

    /// Build a fully disabled config.
    pub fn disabled() -> Self {
        Self {
            level: ThinkingLevel::Off,
            token_budget: 0,
            enabled: false,
            provider_param: None,
        }
    }

    /// Anthropic / Claude extended thinking.
    ///
    /// Uses the `interleaved-thinking-2025-05-14` beta and the `budget_tokens`
    /// parameter under the `thinking` object.
    pub fn for_anthropic(level: &ThinkingLevel) -> Self {
        let token_budget = level.token_budget();
        let enabled = level.is_enabled();
        Self {
            level: level.clone(),
            token_budget,
            enabled,
            provider_param: if enabled {
                Some("budget_tokens".to_string())
            } else {
                None
            },
        }
    }

    /// OpenAI o1 / o3 reasoning effort.
    ///
    /// Maps levels to `"low"`, `"medium"`, or `"high"` via `reasoning_effort`.
    pub fn for_openai(level: &ThinkingLevel) -> Self {
        let token_budget = level.token_budget();
        let enabled = level.is_enabled();
        let effort = match level {
            ThinkingLevel::Off => None,
            ThinkingLevel::Minimal | ThinkingLevel::Low => Some("reasoning_effort:low".to_string()),
            ThinkingLevel::Medium => Some("reasoning_effort:medium".to_string()),
            ThinkingLevel::High | ThinkingLevel::XHigh => Some("reasoning_effort:high".to_string()),
        };
        Self {
            level: level.clone(),
            token_budget,
            enabled,
            provider_param: effort,
        }
    }

    /// Gemini thinking budget.
    ///
    /// Maps to `thinkingConfig.thinkingBudget` in the Gemini API.
    pub fn for_gemini(level: &ThinkingLevel) -> Self {
        let token_budget = level.token_budget();
        let enabled = level.is_enabled();
        Self {
            level: level.clone(),
            token_budget,
            enabled,
            provider_param: if enabled {
                Some(format!("thinkingConfig.thinkingBudget:{token_budget}"))
            } else {
                None
            },
        }
    }
}

// ── ModelWithLevel ────────────────────────────────────────────────────────────

/// Parse model:level shorthand like `"sonnet:high"` or `"gpt-4o:medium"`.
#[derive(Debug, Clone)]
pub struct ModelWithLevel {
    pub model_name: String,
    pub level: ThinkingLevel,
}

impl ModelWithLevel {
    /// Parse `"model:level"`.
    ///
    /// If the level suffix is absent or unrecognised the level defaults to
    /// `ThinkingLevel::Off`.
    pub fn parse(s: &str) -> Self {
        match s.rsplit_once(':') {
            Some((model, level_str)) => {
                let level = ThinkingLevel::from_str(level_str).unwrap_or(ThinkingLevel::Off);
                Self {
                    model_name: model.to_string(),
                    level,
                }
            }
            None => Self {
                model_name: s.to_string(),
                level: ThinkingLevel::Off,
            },
        }
    }

    /// Serialise back to `"model:level"`.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.model_name, self.level.as_str())
    }
}

// ── ThinkingBudgetOverride ────────────────────────────────────────────────────

/// Allow per-session custom token budgets that override the per-level defaults.
#[derive(Debug, Clone, Default)]
pub struct ThinkingBudgetOverride {
    overrides: HashMap<String, u32>,
}

impl ThinkingBudgetOverride {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    /// Store a custom budget for the given level.
    pub fn set(&mut self, level: ThinkingLevel, tokens: u32) {
        self.overrides.insert(level.as_str().to_string(), tokens);
    }

    /// Return the override if one exists, otherwise fall back to the level default.
    pub fn resolve(&self, level: &ThinkingLevel) -> u32 {
        self.overrides
            .get(level.as_str())
            .copied()
            .unwrap_or_else(|| level.token_budget())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- token_budget ---

    #[test]
    fn token_budget_off() {
        assert_eq!(ThinkingLevel::Off.token_budget(), 0);
    }

    #[test]
    fn token_budget_minimal() {
        assert_eq!(ThinkingLevel::Minimal.token_budget(), 200);
    }

    #[test]
    fn token_budget_low() {
        assert_eq!(ThinkingLevel::Low.token_budget(), 1_000);
    }

    #[test]
    fn token_budget_medium() {
        assert_eq!(ThinkingLevel::Medium.token_budget(), 5_000);
    }

    #[test]
    fn token_budget_high() {
        assert_eq!(ThinkingLevel::High.token_budget(), 10_000);
    }

    #[test]
    fn token_budget_xhigh() {
        assert_eq!(ThinkingLevel::XHigh.token_budget(), 32_000);
    }

    // --- from_str roundtrip ---

    #[test]
    fn from_str_roundtrip_all_levels() {
        for level in [
            ThinkingLevel::Off,
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::XHigh,
        ] {
            let parsed = ThinkingLevel::from_str(level.as_str()).unwrap();
            assert_eq!(parsed, level, "roundtrip failed for {:?}", level);
        }
    }

    #[test]
    fn from_str_aliases() {
        assert_eq!(ThinkingLevel::from_str("none"), Some(ThinkingLevel::Off));
        assert_eq!(ThinkingLevel::from_str("med"), Some(ThinkingLevel::Medium));
        assert_eq!(
            ThinkingLevel::from_str("x-high"),
            Some(ThinkingLevel::XHigh)
        );
        assert_eq!(ThinkingLevel::from_str("xl"), Some(ThinkingLevel::XHigh));
    }

    #[test]
    fn from_str_unknown_returns_none() {
        assert_eq!(ThinkingLevel::from_str("banana"), None);
    }

    // --- next_level / prev_level chain ---

    #[test]
    fn next_level_chain() {
        let chain = vec![
            ThinkingLevel::Off,
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::XHigh,
        ];
        for window in chain.windows(2) {
            assert_eq!(window[0].next_level(), Some(window[1].clone()));
        }
        assert_eq!(ThinkingLevel::XHigh.next_level(), None);
    }

    #[test]
    fn prev_level_chain() {
        assert_eq!(ThinkingLevel::Off.prev_level(), None);
        assert_eq!(
            ThinkingLevel::Minimal.prev_level(),
            Some(ThinkingLevel::Off)
        );
        assert_eq!(
            ThinkingLevel::Low.prev_level(),
            Some(ThinkingLevel::Minimal)
        );
        assert_eq!(ThinkingLevel::Medium.prev_level(), Some(ThinkingLevel::Low));
        assert_eq!(
            ThinkingLevel::High.prev_level(),
            Some(ThinkingLevel::Medium)
        );
        assert_eq!(ThinkingLevel::XHigh.prev_level(), Some(ThinkingLevel::High));
    }

    // --- is_enabled ---

    #[test]
    fn is_enabled_off_is_false() {
        assert!(!ThinkingLevel::Off.is_enabled());
    }

    #[test]
    fn is_enabled_all_others_are_true() {
        for level in [
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::XHigh,
        ] {
            assert!(level.is_enabled(), "{:?} should be enabled", level);
        }
    }

    // --- ModelWithLevel parse ---

    #[test]
    fn model_with_level_parse_with_level() {
        let mwl = ModelWithLevel::parse("sonnet:high");
        assert_eq!(mwl.model_name, "sonnet");
        assert_eq!(mwl.level, ThinkingLevel::High);
    }

    #[test]
    fn model_with_level_parse_without_level() {
        let mwl = ModelWithLevel::parse("gpt-4o");
        assert_eq!(mwl.model_name, "gpt-4o");
        assert_eq!(mwl.level, ThinkingLevel::Off);
    }

    #[test]
    fn model_with_level_parse_unrecognised_level() {
        let mwl = ModelWithLevel::parse("claude:turbo");
        assert_eq!(mwl.model_name, "claude");
        assert_eq!(mwl.level, ThinkingLevel::Off);
    }

    #[test]
    fn model_with_level_to_string() {
        let mwl = ModelWithLevel {
            model_name: "opus".to_string(),
            level: ThinkingLevel::XHigh,
        };
        assert_eq!(mwl.to_string(), "opus:xhigh");
    }

    // --- ThinkingConfig provider variants ---

    #[test]
    fn thinking_config_disabled() {
        let cfg = ThinkingConfig::disabled();
        assert!(!cfg.enabled);
        assert_eq!(cfg.token_budget, 0);
        assert!(cfg.provider_param.is_none());
    }

    #[test]
    fn thinking_config_for_anthropic_enabled() {
        let cfg = ThinkingConfig::for_anthropic(&ThinkingLevel::High);
        assert!(cfg.enabled);
        assert_eq!(cfg.token_budget, 10_000);
        assert_eq!(cfg.provider_param.as_deref(), Some("budget_tokens"));
    }

    #[test]
    fn thinking_config_for_anthropic_off() {
        let cfg = ThinkingConfig::for_anthropic(&ThinkingLevel::Off);
        assert!(!cfg.enabled);
        assert!(cfg.provider_param.is_none());
    }

    #[test]
    fn thinking_config_for_openai_low() {
        let cfg = ThinkingConfig::for_openai(&ThinkingLevel::Low);
        assert!(cfg.enabled);
        assert_eq!(cfg.provider_param.as_deref(), Some("reasoning_effort:low"));
    }

    #[test]
    fn thinking_config_for_openai_medium() {
        let cfg = ThinkingConfig::for_openai(&ThinkingLevel::Medium);
        assert_eq!(
            cfg.provider_param.as_deref(),
            Some("reasoning_effort:medium")
        );
    }

    #[test]
    fn thinking_config_for_openai_xhigh() {
        let cfg = ThinkingConfig::for_openai(&ThinkingLevel::XHigh);
        assert_eq!(cfg.provider_param.as_deref(), Some("reasoning_effort:high"));
    }

    #[test]
    fn thinking_config_for_gemini_medium() {
        let cfg = ThinkingConfig::for_gemini(&ThinkingLevel::Medium);
        assert!(cfg.enabled);
        assert_eq!(
            cfg.provider_param.as_deref(),
            Some("thinkingConfig.thinkingBudget:5000")
        );
    }

    #[test]
    fn thinking_config_for_gemini_off() {
        let cfg = ThinkingConfig::for_gemini(&ThinkingLevel::Off);
        assert!(!cfg.enabled);
        assert!(cfg.provider_param.is_none());
    }

    // --- default_for_task ---

    #[test]
    fn default_for_task_mapping() {
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::SimpleEdit),
            ThinkingLevel::Minimal
        );
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::CodeGeneration),
            ThinkingLevel::Low
        );
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::Debugging),
            ThinkingLevel::Medium
        );
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::Architecture),
            ThinkingLevel::High
        );
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::ComplexReasoning),
            ThinkingLevel::XHigh
        );
        assert_eq!(
            ThinkingLevel::default_for_task(&TaskHint::Unknown),
            ThinkingLevel::Low
        );
    }

    // --- ThinkingBudgetOverride ---

    #[test]
    fn budget_override_uses_default_when_not_set() {
        let ovr = ThinkingBudgetOverride::new();
        assert_eq!(ovr.resolve(&ThinkingLevel::Medium), 5_000);
    }

    #[test]
    fn budget_override_uses_custom_when_set() {
        let mut ovr = ThinkingBudgetOverride::new();
        ovr.set(ThinkingLevel::Medium, 3_500);
        assert_eq!(ovr.resolve(&ThinkingLevel::Medium), 3_500);
    }

    #[test]
    fn budget_override_does_not_affect_other_levels() {
        let mut ovr = ThinkingBudgetOverride::new();
        ovr.set(ThinkingLevel::Medium, 3_500);
        assert_eq!(ovr.resolve(&ThinkingLevel::High), 10_000);
        assert_eq!(ovr.resolve(&ThinkingLevel::Low), 1_000);
    }
}
