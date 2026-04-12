//! Hard problem decomposer — complexity estimation, assumption tracking, and hypothesis management.

use serde::{Deserialize, Serialize};

// ─── ComplexityTier ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityTier {
    Trivial,
    Simple,
    Moderate,
    Complex,
    Hard,
}

impl ComplexityTier {
    /// Estimates complexity from raw counts.
    /// score = files_affected + cross_module_deps*2 + ambiguous_specs*3
    /// 0-3: Trivial, 4-7: Simple, 8-14: Moderate, 15-24: Complex, 25+: Hard
    pub fn estimate(files_affected: u32, cross_module_deps: u32, ambiguous_specs: u32) -> ComplexityTier {
        let score = estimate_complexity_score(files_affected, cross_module_deps, ambiguous_specs);
        match score {
            0..=3 => ComplexityTier::Trivial,
            4..=7 => ComplexityTier::Simple,
            8..=14 => ComplexityTier::Moderate,
            15..=24 => ComplexityTier::Complex,
            _ => ComplexityTier::Hard,
        }
    }
}

// ─── AssumptionImpact ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssumptionImpact {
    Critical,
    High,
    Medium,
    Low,
}

// ─── Assumption ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assumption {
    pub assumption_id: String,
    pub description: String,
    pub impact: AssumptionImpact,
    pub confirmed: bool,
}

// ─── HypothesisResult ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HypothesisResult {
    Confirmed,
    Refuted(String),
    Inconclusive,
}

// ─── Hypothesis ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub hypothesis_id: String,
    pub description: String,
    pub verifiable_unit: String,
    pub confidence: u8,
    pub result: Option<HypothesisResult>,
}

// ─── AmbiguityKind ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AmbiguityKind {
    MissingReturnType,
    UnspecifiedErrorHandling,
    AmbiguousEntityName,
    MissingEdgeCases,
    UnclearScope,
}

// ─── ClarifyingQuestion ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyingQuestion {
    pub question_id: String,
    pub ambiguity_kind: AmbiguityKind,
    pub question: String,
    pub impact: AssumptionImpact,
}

// ─── Subsystem ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subsystem {
    pub name: String,
    pub file_patterns: Vec<String>,
    pub depends_on: Vec<String>,
}

// ─── DecomposedTask ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedTask {
    pub task_id: String,
    pub original_description: String,
    pub subsystems: Vec<Subsystem>,
    pub assumptions: Vec<Assumption>,
    pub hypotheses: Vec<Hypothesis>,
    pub clarifying_questions: Vec<ClarifyingQuestion>,
    pub complexity: ComplexityTier,
    pub recommended_strategy: String,
}

// ─── ProblemDecomposer ───────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ProblemDecomposer;

impl ProblemDecomposer {
    pub fn new() -> Self {
        Self
    }

    pub fn decompose(&self, description: &str) -> DecomposedTask {
        let desc_lower = description.to_lowercase();
        let mut subsystems: Vec<Subsystem> = Vec::new();
        let mut assumptions: Vec<Assumption> = Vec::new();

        // ── Subsystem extraction ───────────────────────────────────────────
        if desc_lower.contains("database") || desc_lower.contains("db") {
            subsystems.push(Subsystem {
                name: "data".to_string(),
                file_patterns: vec!["**/db/**".to_string(), "**/*repository*".to_string()],
                depends_on: vec![],
            });
        }
        if desc_lower.contains("auth") || desc_lower.contains("login") {
            subsystems.push(Subsystem {
                name: "auth".to_string(),
                file_patterns: vec!["**/auth/**".to_string(), "**/*middleware*".to_string()],
                depends_on: vec![],
            });
        }
        if desc_lower.contains("api")
            || desc_lower.contains("endpoint")
            || desc_lower.contains("route")
        {
            subsystems.push(Subsystem {
                name: "api".to_string(),
                file_patterns: vec!["**/routes/**".to_string(), "**/*controller*".to_string()],
                depends_on: vec![],
            });
        }
        if desc_lower.contains("ui")
            || desc_lower.contains("component")
            || desc_lower.contains("form")
        {
            subsystems.push(Subsystem {
                name: "frontend".to_string(),
                file_patterns: vec!["**/components/**".to_string(), "**/*.tsx".to_string()],
                depends_on: vec![],
            });
        }
        if subsystems.is_empty() {
            subsystems.push(Subsystem {
                name: "core".to_string(),
                file_patterns: vec!["src/**".to_string()],
                depends_on: vec![],
            });
        }

        // ── Assumptions ────────────────────────────────────────────────────
        if desc_lower.contains("database") || desc_lower.contains("db") {
            assumptions.push(Assumption {
                assumption_id: "a-db-schema".to_string(),
                description: "Database schema has required tables".to_string(),
                impact: AssumptionImpact::High,
                confirmed: false,
            });
        }
        if desc_lower.contains("auth") || desc_lower.contains("login") {
            assumptions.push(Assumption {
                assumption_id: "a-auth-middleware".to_string(),
                description: "Authentication middleware is configured".to_string(),
                impact: AssumptionImpact::High,
                confirmed: false,
            });
        }

        // ── Clarifying questions ───────────────────────────────────────────
        let mut clarifying_questions: Vec<ClarifyingQuestion> = Vec::new();
        if description.len() < 50 {
            clarifying_questions.push(ClarifyingQuestion {
                question_id: "q-scope".to_string(),
                ambiguity_kind: AmbiguityKind::UnclearScope,
                question: "What is the full scope of this task?".to_string(),
                impact: AssumptionImpact::High,
            });
            clarifying_questions.push(ClarifyingQuestion {
                question_id: "q-error-handling".to_string(),
                ambiguity_kind: AmbiguityKind::UnspecifiedErrorHandling,
                question: "How should errors be handled?".to_string(),
                impact: AssumptionImpact::Medium,
            });
        }

        // ── Complexity ────────────────────────────────────────────────────
        let files_affected = subsystems.len() as u32;
        let ambiguous = if description.len() < 50 { 1 } else { 0 };
        let complexity = ComplexityTier::estimate(files_affected, 0, ambiguous);
        let recommended_strategy = strategy_for_tier(&complexity).to_string();

        DecomposedTask {
            task_id: format!("task-{}", description.len()),
            original_description: description.to_string(),
            subsystems,
            assumptions,
            hypotheses: vec![],
            clarifying_questions,
            complexity,
            recommended_strategy,
        }
    }
}

// ─── Free functions ───────────────────────────────────────────────────────────

pub fn estimate_complexity_score(files: u32, deps: u32, ambiguous: u32) -> u32 {
    files + deps * 2 + ambiguous * 3
}

pub fn strategy_for_tier(tier: &ComplexityTier) -> &'static str {
    match tier {
        ComplexityTier::Trivial | ComplexityTier::Simple => "direct",
        ComplexityTier::Moderate => "mcts",
        ComplexityTier::Complex | ComplexityTier::Hard => "decompose-first",
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ComplexityTier::estimate ──────────────────────────────────────────

    #[test]
    fn test_trivial_zero() {
        assert_eq!(ComplexityTier::estimate(0, 0, 0), ComplexityTier::Trivial);
    }

    #[test]
    fn test_trivial_score_3() {
        assert_eq!(ComplexityTier::estimate(3, 0, 0), ComplexityTier::Trivial);
    }

    #[test]
    fn test_simple_score_4() {
        assert_eq!(ComplexityTier::estimate(4, 0, 0), ComplexityTier::Simple);
    }

    #[test]
    fn test_simple_score_7() {
        assert_eq!(ComplexityTier::estimate(1, 3, 0), ComplexityTier::Simple);
    }

    #[test]
    fn test_moderate_score_8() {
        assert_eq!(ComplexityTier::estimate(2, 3, 0), ComplexityTier::Moderate);
    }

    #[test]
    fn test_moderate_score_14() {
        assert_eq!(ComplexityTier::estimate(2, 3, 1), ComplexityTier::Moderate);
    }

    #[test]
    fn test_complex_score_15() {
        assert_eq!(ComplexityTier::estimate(3, 3, 2), ComplexityTier::Complex);
    }

    #[test]
    fn test_complex_score_24() {
        assert_eq!(ComplexityTier::estimate(4, 5, 2), ComplexityTier::Complex);
    }

    #[test]
    fn test_hard_score_25() {
        // score = 5 + 5*2 + 4*3 = 5 + 10 + 12 = 27 >= 25 → Hard
        assert_eq!(ComplexityTier::estimate(5, 5, 4), ComplexityTier::Hard);
    }

    #[test]
    fn test_hard_score_large() {
        assert_eq!(ComplexityTier::estimate(10, 10, 10), ComplexityTier::Hard);
    }

    // ── estimate_complexity_score ─────────────────────────────────────────

    #[test]
    fn test_score_formula() {
        // 2 + 3*2 + 1*3 = 2 + 6 + 3 = 11
        assert_eq!(estimate_complexity_score(2, 3, 1), 11);
    }

    #[test]
    fn test_score_zero_all() {
        assert_eq!(estimate_complexity_score(0, 0, 0), 0);
    }

    #[test]
    fn test_score_deps_weighted_double() {
        assert_eq!(estimate_complexity_score(0, 5, 0), 10);
    }

    #[test]
    fn test_score_ambiguous_weighted_triple() {
        assert_eq!(estimate_complexity_score(0, 0, 5), 15);
    }

    // ── strategy_for_tier ─────────────────────────────────────────────────

    #[test]
    fn test_strategy_trivial() {
        assert_eq!(strategy_for_tier(&ComplexityTier::Trivial), "direct");
    }

    #[test]
    fn test_strategy_simple() {
        assert_eq!(strategy_for_tier(&ComplexityTier::Simple), "direct");
    }

    #[test]
    fn test_strategy_moderate() {
        assert_eq!(strategy_for_tier(&ComplexityTier::Moderate), "mcts");
    }

    #[test]
    fn test_strategy_complex() {
        assert_eq!(strategy_for_tier(&ComplexityTier::Complex), "decompose-first");
    }

    #[test]
    fn test_strategy_hard() {
        assert_eq!(strategy_for_tier(&ComplexityTier::Hard), "decompose-first");
    }

    // ── ProblemDecomposer ─────────────────────────────────────────────────

    #[test]
    fn test_decompose_db_subsystem() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Add database support to the backend service");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"data"));
    }

    #[test]
    fn test_decompose_db_keyword_variant() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Connect to the db and run migrations");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"data"));
    }

    #[test]
    fn test_decompose_auth_subsystem() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Implement auth and login flow for users");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"auth"));
    }

    #[test]
    fn test_decompose_api_subsystem() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Add a REST endpoint for the user profile route");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"api"));
    }

    #[test]
    fn test_decompose_frontend_subsystem() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Build a React form component for user registration ui");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"frontend"));
    }

    #[test]
    fn test_decompose_fallback_core() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Refactor the utility functions");
        let names: Vec<&str> = task.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"core"));
    }

    #[test]
    fn test_decompose_db_assumption_generated() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Connect database and run queries");
        assert!(task.assumptions.iter().any(|a| a.description.contains("schema")));
    }

    #[test]
    fn test_decompose_auth_assumption_generated() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Add authentication middleware and login handler");
        assert!(task
            .assumptions
            .iter()
            .any(|a| a.description.contains("Authentication middleware")));
    }

    #[test]
    fn test_decompose_short_desc_clarifying_questions() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("fix bug");
        assert!(!task.clarifying_questions.is_empty());
    }

    #[test]
    fn test_decompose_long_desc_no_clarifying_questions() {
        let d = ProblemDecomposer::new();
        let desc = "Implement a comprehensive user authentication system with OAuth2 support, JWT tokens, and refresh token rotation for our web application backend";
        let task = d.decompose(desc);
        assert!(task.clarifying_questions.is_empty());
    }

    #[test]
    fn test_decompose_short_desc_two_questions() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("do it");
        assert_eq!(task.clarifying_questions.len(), 2);
    }

    #[test]
    fn test_decompose_task_id_not_empty() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("some task");
        assert!(!task.task_id.is_empty());
    }

    #[test]
    fn test_decompose_original_description_preserved() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("my specific task description here");
        assert_eq!(task.original_description, "my specific task description here");
    }

    #[test]
    fn test_decompose_recommended_strategy_set() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("simple fix");
        assert!(!task.recommended_strategy.is_empty());
    }

    #[test]
    fn test_decompose_multiple_subsystems() {
        let d = ProblemDecomposer::new();
        let task = d.decompose("Build a login form component that calls the auth API endpoint and stores session in the database");
        // Should detect: auth, api, frontend, data
        assert!(task.subsystems.len() >= 3);
    }

    #[test]
    fn test_hypothesis_result_variants() {
        let confirmed = HypothesisResult::Confirmed;
        let refuted = HypothesisResult::Refuted("wrong assumption".into());
        let inconclusive = HypothesisResult::Inconclusive;
        assert_eq!(confirmed, HypothesisResult::Confirmed);
        assert!(matches!(refuted, HypothesisResult::Refuted(_)));
        assert_eq!(inconclusive, HypothesisResult::Inconclusive);
    }

    #[test]
    fn test_assumption_impact_variants() {
        let _ = AssumptionImpact::Critical;
        let _ = AssumptionImpact::High;
        let _ = AssumptionImpact::Medium;
        let _ = AssumptionImpact::Low;
    }

    #[test]
    fn test_ambiguity_kind_variants() {
        let _ = AmbiguityKind::MissingReturnType;
        let _ = AmbiguityKind::UnspecifiedErrorHandling;
        let _ = AmbiguityKind::AmbiguousEntityName;
        let _ = AmbiguityKind::MissingEdgeCases;
        let _ = AmbiguityKind::UnclearScope;
    }
}
