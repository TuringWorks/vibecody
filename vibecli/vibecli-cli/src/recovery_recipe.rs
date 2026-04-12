//! Recovery recipes for failed or stalled agent actions.
//!
//! Claw-code parity Wave 2: maintains a library of recovery strategies indexed
//! by error pattern, enabling automatic retry with corrective context.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Error Pattern ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorPattern {
    CompilationError,
    TestFailure,
    PermissionDenied,
    FileNotFound,
    NetworkTimeout,
    RateLimited,
    ToolTimeout,
    ParseError,
    MergeConflict,
    Custom(String),
}

impl ErrorPattern {
    /// Detect pattern from an error message string.
    pub fn from_message(msg: &str) -> Self {
        let lower = msg.to_lowercase();
        if lower.contains("permission denied") || lower.contains("access denied") { return Self::PermissionDenied; }
        if lower.contains("no such file") || lower.contains("not found") { return Self::FileNotFound; }
        if lower.contains("timeout") || lower.contains("timed out") { return Self::NetworkTimeout; }
        if lower.contains("rate limit") || lower.contains("429") { return Self::RateLimited; }
        if lower.contains("conflict") { return Self::MergeConflict; }
        if lower.contains("error[e") || lower.contains("compil") { return Self::CompilationError; }
        if lower.contains("test") && (lower.contains("failed") || lower.contains("panic")) { return Self::TestFailure; }
        if lower.contains("parse") || lower.contains("syntax") { return Self::ParseError; }
        Self::Custom(msg.chars().take(60).collect())
    }
}

// ─── Recovery Step ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Retry the same action immediately.
    Retry,
    /// Retry after waiting this many milliseconds.
    RetryAfterMs(u64),
    /// Inject additional context/instruction into the agent prompt.
    InjectContext(String),
    /// Run a corrective shell command before retrying.
    RunCommand(String),
    /// Escalate to human review.
    Escalate,
    /// Abort the current task cleanly.
    Abort { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    pub action: RecoveryAction,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRecipe {
    pub pattern: ErrorPattern,
    pub name: String,
    pub max_attempts: u32,
    pub steps: Vec<RecoveryStep>,
}

impl RecoveryRecipe {
    pub fn step_at(&self, attempt: u32) -> Option<&RecoveryStep> {
        self.steps.get((attempt as usize).min(self.steps.len().saturating_sub(1)))
    }

    pub fn is_exhausted(&self, attempt: u32) -> bool { attempt >= self.max_attempts }
}

// ─── Built-in Recipes ────────────────────────────────────────────────────────

pub fn builtin_recipes() -> Vec<RecoveryRecipe> {
    vec![
        RecoveryRecipe {
            pattern: ErrorPattern::RateLimited,
            name: "rate-limit-backoff".into(),
            max_attempts: 3,
            steps: vec![
                RecoveryStep { action: RecoveryAction::RetryAfterMs(5_000), description: "wait 5s".into() },
                RecoveryStep { action: RecoveryAction::RetryAfterMs(30_000), description: "wait 30s".into() },
                RecoveryStep { action: RecoveryAction::Escalate, description: "persistent rate limit".into() },
            ],
        },
        RecoveryRecipe {
            pattern: ErrorPattern::NetworkTimeout,
            name: "network-retry".into(),
            max_attempts: 3,
            steps: vec![
                RecoveryStep { action: RecoveryAction::Retry, description: "immediate retry".into() },
                RecoveryStep { action: RecoveryAction::RetryAfterMs(10_000), description: "retry after 10s".into() },
                RecoveryStep { action: RecoveryAction::Abort { reason: "network unavailable".into() }, description: "give up".into() },
            ],
        },
        RecoveryRecipe {
            pattern: ErrorPattern::CompilationError,
            name: "compile-fix".into(),
            max_attempts: 2,
            steps: vec![
                RecoveryStep {
                    action: RecoveryAction::InjectContext("Review the compilation error above and fix it. Common causes: missing imports, type mismatches, borrow errors.".into()),
                    description: "inject fix context".into(),
                },
                RecoveryStep { action: RecoveryAction::Escalate, description: "unable to fix compilation".into() },
            ],
        },
        RecoveryRecipe {
            pattern: ErrorPattern::TestFailure,
            name: "test-fix".into(),
            max_attempts: 2,
            steps: vec![
                RecoveryStep {
                    action: RecoveryAction::InjectContext("Tests are failing. Read the failing test output carefully and fix the implementation (not the tests).".into()),
                    description: "inject test context".into(),
                },
                RecoveryStep { action: RecoveryAction::Escalate, description: "test fix exhausted".into() },
            ],
        },
        RecoveryRecipe {
            pattern: ErrorPattern::MergeConflict,
            name: "merge-resolve".into(),
            max_attempts: 2,
            steps: vec![
                RecoveryStep {
                    action: RecoveryAction::InjectContext("Merge conflicts detected. Resolve them preferring the incoming changes for code, and both for prose/docs.".into()),
                    description: "inject merge context".into(),
                },
                RecoveryStep { action: RecoveryAction::Escalate, description: "complex conflict".into() },
            ],
        },
        RecoveryRecipe {
            pattern: ErrorPattern::PermissionDenied,
            name: "permission-fix".into(),
            max_attempts: 1,
            steps: vec![
                RecoveryStep { action: RecoveryAction::Escalate, description: "permission error requires human".into() },
            ],
        },
    ]
}

// ─── Recipe Engine ────────────────────────────────────────────────────────────

pub struct RecoveryEngine {
    recipes: HashMap<String, RecoveryRecipe>,
    /// pattern key → attempt count
    attempt_counts: HashMap<String, u32>,
}

impl RecoveryEngine {
    pub fn new(recipes: Vec<RecoveryRecipe>) -> Self {
        let map = recipes.into_iter().map(|r| (r.name.clone(), r)).collect();
        Self { recipes: map, attempt_counts: HashMap::new() }
    }

    /// Look up a recipe by error pattern.
    pub fn find_recipe(&self, pattern: &ErrorPattern) -> Option<&RecoveryRecipe> {
        self.recipes.values().find(|r| &r.pattern == pattern)
    }

    /// Recommend a recovery action for the given error message.
    pub fn recommend(&mut self, error_msg: &str) -> Option<RecoveryStep> {
        let pattern = ErrorPattern::from_message(error_msg);
        let recipe = self.find_recipe(&pattern)?;
        let name = recipe.name.clone();
        let attempt = *self.attempt_counts.get(&name).unwrap_or(&0);
        if recipe.is_exhausted(attempt) { return None; }
        let step = recipe.step_at(attempt).cloned();
        *self.attempt_counts.entry(name).or_insert(0) += 1;
        step
    }

    /// Reset attempt counter for a recipe (after successful recovery).
    pub fn reset(&mut self, recipe_name: &str) { self.attempt_counts.remove(recipe_name); }

    pub fn attempt_count(&self, recipe_name: &str) -> u32 {
        *self.attempt_counts.get(recipe_name).unwrap_or(&0)
    }
}

impl Default for RecoveryEngine {
    fn default() -> Self { Self::new(builtin_recipes()) }
}

// ── Prescriptive FailureScenario Registry ─────────────────────────────────────
//
// Maps each `FailureScenario` to an ordered `Vec<RecoveryStep>`.  On failure
// the registry attempts the first automatic step once; if that fails it
// escalates rather than retrying infinitely.

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// The seven high-level agent failure scenarios this registry covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureScenario {
    ProviderTimeout,
    ToolPermissionDenied,
    SessionCorrupted,
    CompactionFailed,
    SubagentCrash,
    WorkspaceConflict,
    MCPServerDown,
}

impl std::fmt::Display for FailureScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderTimeout      => write!(f, "provider_timeout"),
            Self::ToolPermissionDenied => write!(f, "tool_permission_denied"),
            Self::SessionCorrupted     => write!(f, "session_corrupted"),
            Self::CompactionFailed     => write!(f, "compaction_failed"),
            Self::SubagentCrash        => write!(f, "subagent_crash"),
            Self::WorkspaceConflict    => write!(f, "workspace_conflict"),
            Self::MCPServerDown        => write!(f, "mcp_server_down"),
        }
    }
}

/// Outcome reported after executing auto-recovery for a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Resolved,
    Escalated,
    Failed,
}

impl std::fmt::Display for RecoveryOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved  => write!(f, "resolved"),
            Self::Escalated => write!(f, "escalated"),
            Self::Failed    => write!(f, "failed"),
        }
    }
}

/// A single step in a scenario recovery recipe.
/// `automatic = true` means the engine may run it without human input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRecoveryStep {
    pub description: String,
    pub automatic: bool,
    pub action: String,
}

impl ScenarioRecoveryStep {
    pub fn auto(description: &str, action: &str) -> Self {
        Self { description: description.to_string(), automatic: true, action: action.to_string() }
    }
    pub fn manual(description: &str, action: &str) -> Self {
        Self { description: description.to_string(), automatic: false, action: action.to_string() }
    }
}

/// The ordered recipe for a single `FailureScenario`.
#[derive(Debug, Clone)]
pub struct ScenarioRecipe {
    pub scenario: FailureScenario,
    pub steps: Vec<ScenarioRecoveryStep>,
    pub max_auto_attempts: u32,
}

/// Audit record emitted each time auto-recovery runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEvent {
    pub scenario_name: String,
    pub outcome: String,
    pub steps_attempted: usize,
    pub timestamp: u64,
}

/// Central registry: maps every `FailureScenario` to its `ScenarioRecipe`.
pub struct RecoveryRegistry {
    recipes: HashMap<FailureScenario, ScenarioRecipe>,
    events: Vec<RecoveryEvent>,
}

impl RecoveryRegistry {
    /// Build the registry with all 7 built-in scenario recipes.
    pub fn new() -> Self {
        let mut recipes = HashMap::new();

        let all: Vec<(FailureScenario, Vec<ScenarioRecoveryStep>)> = vec![
            (FailureScenario::ProviderTimeout, vec![
                ScenarioRecoveryStep::auto("Retry with exponential backoff", "retry_backoff"),
                ScenarioRecoveryStep::auto("Switch to fallback provider", "switch_provider"),
                ScenarioRecoveryStep::manual("Check provider status page", "manual_check"),
            ]),
            (FailureScenario::ToolPermissionDenied, vec![
                ScenarioRecoveryStep::auto("Retry with reduced permissions scope", "reduce_scope"),
                ScenarioRecoveryStep::manual("Request user permission elevation", "request_elevation"),
            ]),
            (FailureScenario::SessionCorrupted, vec![
                ScenarioRecoveryStep::auto("Restore from last valid checkpoint", "restore_checkpoint"),
                ScenarioRecoveryStep::auto("Rebuild session from trace files", "rebuild_from_trace"),
                ScenarioRecoveryStep::manual("Start fresh session", "fresh_session"),
            ]),
            (FailureScenario::CompactionFailed, vec![
                ScenarioRecoveryStep::auto("Retry compaction with smaller window", "retry_smaller_window"),
                ScenarioRecoveryStep::auto("Skip compaction, continue with full context", "skip_compaction"),
                ScenarioRecoveryStep::manual("Manually summarize and restart", "manual_restart"),
            ]),
            (FailureScenario::SubagentCrash, vec![
                ScenarioRecoveryStep::auto("Respawn subagent with same task", "respawn"),
                ScenarioRecoveryStep::auto("Escalate task to parent agent", "escalate_to_parent"),
                ScenarioRecoveryStep::manual("Mark task as failed and continue", "mark_failed"),
            ]),
            (FailureScenario::WorkspaceConflict, vec![
                ScenarioRecoveryStep::auto("Auto-rebase on base branch", "auto_rebase"),
                ScenarioRecoveryStep::auto("Create new branch to avoid conflict", "new_branch"),
                ScenarioRecoveryStep::manual("Manually resolve conflicts", "manual_resolve"),
            ]),
            (FailureScenario::MCPServerDown, vec![
                ScenarioRecoveryStep::auto("Reconnect after backoff", "reconnect"),
                ScenarioRecoveryStep::auto("Use cached tool results if available", "use_cache"),
                ScenarioRecoveryStep::manual("Disable MCP server and continue without", "disable_mcp"),
            ]),
        ];

        for (scenario, steps) in all {
            recipes.insert(scenario.clone(), ScenarioRecipe {
                scenario,
                steps,
                max_auto_attempts: 1,
            });
        }

        Self { recipes, events: Vec::new() }
    }

    pub fn get_recipe(&self, scenario: &FailureScenario) -> Option<&ScenarioRecipe> {
        self.recipes.get(scenario)
    }

    /// Execute auto-recovery: attempts the first automatic step once.
    /// Returns `Escalated` when `max_auto_attempts` is reached (always 1 here),
    /// or `Failed` if no recipe exists for the scenario.
    pub fn execute_auto_recovery(&mut self, scenario: &FailureScenario) -> RecoveryOutcome {
        let recipe = match self.recipes.get(scenario) {
            Some(r) => r.clone(),
            None => {
                self.record_event(RecoveryEvent {
                    scenario_name: scenario.to_string(),
                    outcome: RecoveryOutcome::Failed.to_string(),
                    steps_attempted: 0,
                    timestamp: now_millis(),
                });
                return RecoveryOutcome::Failed;
            }
        };

        let auto_count = recipe.steps.iter().filter(|s| s.automatic).count();
        let steps_attempted = auto_count.min(recipe.max_auto_attempts as usize);

        // 1 auto attempt → escalate (real impl would execute the action).
        let outcome = if steps_attempted >= recipe.max_auto_attempts as usize {
            RecoveryOutcome::Escalated
        } else {
            RecoveryOutcome::Resolved
        };

        self.record_event(RecoveryEvent {
            scenario_name: scenario.to_string(),
            outcome: outcome.to_string(),
            steps_attempted,
            timestamp: now_millis(),
        });

        outcome
    }

    pub fn record_event(&mut self, event: RecoveryEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[RecoveryEvent] {
        &self.events
    }

    pub fn recipe_count(&self) -> usize {
        self.recipes.len()
    }
}

impl Default for RecoveryRegistry {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for RecoveryRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryRegistry")
            .field("recipe_count", &self.recipes.len())
            .field("events", &self.events)
            .finish()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_from_rate_limit_msg() {
        assert_eq!(ErrorPattern::from_message("429 rate limit exceeded"), ErrorPattern::RateLimited);
    }

    #[test]
    fn test_pattern_from_timeout_msg() {
        assert_eq!(ErrorPattern::from_message("connection timed out"), ErrorPattern::NetworkTimeout);
    }

    #[test]
    fn test_pattern_from_permission_msg() {
        assert_eq!(ErrorPattern::from_message("Permission denied: /etc/shadow"), ErrorPattern::PermissionDenied);
    }

    #[test]
    fn test_pattern_from_compile_msg() {
        assert_eq!(ErrorPattern::from_message("error[E0308]: mismatched types"), ErrorPattern::CompilationError);
    }

    #[test]
    fn test_pattern_from_test_fail() {
        assert_eq!(ErrorPattern::from_message("test foo::bar ... FAILED\npanicked"), ErrorPattern::TestFailure);
    }

    #[test]
    fn test_pattern_from_merge_conflict() {
        assert_eq!(ErrorPattern::from_message("CONFLICT (content): Merge conflict in file.rs"), ErrorPattern::MergeConflict);
    }

    #[test]
    fn test_pattern_custom_unknown() {
        let p = ErrorPattern::from_message("something totally new");
        assert!(matches!(p, ErrorPattern::Custom(_)));
    }

    #[test]
    fn test_recommend_rate_limit() {
        let mut engine = RecoveryEngine::default();
        let step = engine.recommend("429 rate limit exceeded");
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, RecoveryAction::RetryAfterMs(5_000)));
    }

    #[test]
    fn test_recommend_escalates_after_max_attempts() {
        let mut engine = RecoveryEngine::default();
        for _ in 0..2 { engine.recommend("429 rate limit exceeded"); }
        let step = engine.recommend("429 rate limit exceeded");
        assert!(matches!(step.unwrap().action, RecoveryAction::Escalate));
    }

    #[test]
    fn test_recommend_none_when_exhausted() {
        let mut engine = RecoveryEngine::default();
        for _ in 0..3 { engine.recommend("429 rate limit exceeded"); }
        assert!(engine.recommend("429 rate limit exceeded").is_none());
    }

    #[test]
    fn test_reset_clears_attempts() {
        let mut engine = RecoveryEngine::default();
        engine.recommend("connection timed out");
        engine.recommend("connection timed out");
        engine.reset("network-retry");
        assert_eq!(engine.attempt_count("network-retry"), 0);
    }

    #[test]
    fn test_compile_error_injects_context() {
        let mut engine = RecoveryEngine::default();
        let step = engine.recommend("error[E0308] mismatched types");
        assert!(matches!(step.unwrap().action, RecoveryAction::InjectContext(_)));
    }

    #[test]
    fn test_permission_denied_escalates() {
        let mut engine = RecoveryEngine::default();
        let step = engine.recommend("permission denied: /etc/shadow");
        assert!(matches!(step.unwrap().action, RecoveryAction::Escalate));
    }

    #[test]
    fn test_recipe_step_at_clamps_to_last() {
        let recipe = &builtin_recipes()[0]; // rate-limit (3 steps)
        let last = recipe.step_at(10).unwrap();
        assert!(matches!(last.action, RecoveryAction::Escalate));
    }

    #[test]
    fn test_recipe_exhausted() {
        let recipe = &builtin_recipes()[0];
        assert!(recipe.is_exhausted(3));
        assert!(!recipe.is_exhausted(2));
    }

    #[test]
    fn test_builtin_recipes_count() {
        assert_eq!(builtin_recipes().len(), 6);
    }

    #[test]
    fn test_find_recipe_by_pattern() {
        let engine = RecoveryEngine::default();
        let recipe = engine.find_recipe(&ErrorPattern::RateLimited);
        assert!(recipe.is_some());
        assert_eq!(recipe.unwrap().name, "rate-limit-backoff");
    }

    #[test]
    fn test_find_recipe_not_found() {
        let engine = RecoveryEngine::default();
        assert!(engine.find_recipe(&ErrorPattern::FileNotFound).is_none());
    }

    // ── RecoveryRegistry (FailureScenario) tests ──────────────────────────────

    #[test]
    fn registry_has_all_7_scenarios() {
        let reg = RecoveryRegistry::new();
        assert_eq!(reg.recipe_count(), 7);
    }

    #[test]
    fn provider_timeout_recipe_has_steps() {
        let reg = RecoveryRegistry::new();
        let recipe = reg.get_recipe(&FailureScenario::ProviderTimeout).unwrap();
        assert!(!recipe.steps.is_empty());
    }

    #[test]
    fn first_step_of_each_recipe_is_automatic() {
        let reg = RecoveryRegistry::new();
        for scenario in &[
            FailureScenario::ProviderTimeout,
            FailureScenario::ToolPermissionDenied,
            FailureScenario::SessionCorrupted,
            FailureScenario::CompactionFailed,
            FailureScenario::SubagentCrash,
            FailureScenario::WorkspaceConflict,
            FailureScenario::MCPServerDown,
        ] {
            let recipe = reg.get_recipe(scenario).unwrap();
            assert!(recipe.steps[0].automatic, "First step of {:?} must be automatic", scenario);
        }
    }

    #[test]
    fn execute_auto_recovery_records_event() {
        let mut reg = RecoveryRegistry::new();
        reg.execute_auto_recovery(&FailureScenario::SubagentCrash);
        assert_eq!(reg.events().len(), 1);
    }

    #[test]
    fn event_scenario_name_matches() {
        let mut reg = RecoveryRegistry::new();
        reg.execute_auto_recovery(&FailureScenario::MCPServerDown);
        assert_eq!(reg.events()[0].scenario_name, "mcp_server_down");
    }

    #[test]
    fn event_timestamp_is_nonzero() {
        let mut reg = RecoveryRegistry::new();
        reg.execute_auto_recovery(&FailureScenario::ProviderTimeout);
        assert!(reg.events()[0].timestamp > 0);
    }

    #[test]
    fn events_are_recorded_in_order() {
        let mut reg = RecoveryRegistry::new();
        reg.execute_auto_recovery(&FailureScenario::ProviderTimeout);
        reg.execute_auto_recovery(&FailureScenario::MCPServerDown);
        assert_eq!(reg.events()[0].scenario_name, "provider_timeout");
        assert_eq!(reg.events()[1].scenario_name, "mcp_server_down");
    }

    #[test]
    fn execute_returns_escalated_after_max_attempts() {
        let mut reg = RecoveryRegistry::new();
        let outcome = reg.execute_auto_recovery(&FailureScenario::ProviderTimeout);
        // max_auto_attempts = 1; after 1 auto step it escalates
        assert_eq!(outcome, RecoveryOutcome::Escalated);
    }
}
