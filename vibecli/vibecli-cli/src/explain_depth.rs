#![allow(dead_code)]
//! Code explanation depth levels — surface / deep / expert explanations.
//! FIT-GAP v11 Phase 48 — closes gap vs Claude Code 1.x, Cody 6.0.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Explanation depth level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DepthLevel {
    /// One-sentence summary for non-technical readers.
    Surface,
    /// Paragraph-level explanation with key concepts.
    Overview,
    /// Full breakdown including data flow and edge cases.
    Deep,
    /// Expert-level: internals, complexity, tradeoffs.
    Expert,
}

impl DepthLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "surface" | "1" => Some(Self::Surface),
            "overview" | "2" => Some(Self::Overview),
            "deep" | "3" => Some(Self::Deep),
            "expert" | "4" => Some(Self::Expert),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Surface => "surface",
            Self::Overview => "overview",
            Self::Deep => "deep",
            Self::Expert => "expert",
        }
    }
    pub fn all() -> &'static [DepthLevel] {
        &[Self::Surface, Self::Overview, Self::Deep, Self::Expert]
    }
}

/// Target audience for the explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    Novice,
    Developer,
    SeniorEngineer,
    Architect,
}

impl Audience {
    pub fn suggested_depth(&self) -> DepthLevel {
        match self {
            Self::Novice => DepthLevel::Surface,
            Self::Developer => DepthLevel::Overview,
            Self::SeniorEngineer => DepthLevel::Deep,
            Self::Architect => DepthLevel::Expert,
        }
    }
}

/// An explanation request for a code snippet.
#[derive(Debug, Clone)]
pub struct ExplainRequest {
    pub code: String,
    pub language: String,
    pub depth: DepthLevel,
    pub audience: Audience,
    pub focus: Option<String>,
}

impl ExplainRequest {
    pub fn new(code: impl Into<String>, language: impl Into<String>, depth: DepthLevel) -> Self {
        Self {
            code: code.into(),
            language: language.into(),
            depth,
            audience: Audience::Developer,
            focus: None,
        }
    }
    pub fn with_audience(mut self, audience: Audience) -> Self {
        self.audience = audience;
        self
    }
    pub fn with_focus(mut self, focus: impl Into<String>) -> Self {
        self.focus = Some(focus.into());
        self
    }
}

/// Generated explanation at a given depth.
#[derive(Debug, Clone)]
pub struct Explanation {
    pub depth: DepthLevel,
    pub title: String,
    pub body: String,
    pub complexity_hint: Option<String>,
    pub follow_up_questions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Explainer
// ---------------------------------------------------------------------------

/// Generates structured explanation prompts for LLM consumption.
pub struct CodeExplainer;

impl CodeExplainer {
    /// Build a system prompt for a given depth and audience.
    pub fn system_prompt(depth: DepthLevel, audience: Audience) -> String {
        let style = match audience {
            Audience::Novice => "Use plain English. Avoid jargon. Assume no prior programming knowledge.",
            Audience::Developer => "Assume familiarity with programming concepts. Use technical terms where helpful.",
            Audience::SeniorEngineer => "Be precise and complete. Include edge cases, complexity, and tradeoffs.",
            Audience::Architect => "Include system-level concerns, scalability, security, and design patterns.",
        };
        let depth_instr = match depth {
            DepthLevel::Surface => "Provide a single-sentence summary of what this code does.",
            DepthLevel::Overview => "Explain the purpose, key components, and high-level data flow in 2-4 sentences.",
            DepthLevel::Deep => "Break down the logic step by step, explain inputs/outputs, error handling, and important edge cases.",
            DepthLevel::Expert => "Provide expert analysis: algorithmic complexity, memory model, potential concurrency issues, security concerns, and architectural implications.",
        };
        format!("{}\n\n{}", style, depth_instr)
    }

    /// Build a user prompt given the request.
    pub fn user_prompt(req: &ExplainRequest) -> String {
        let focus_str = req.focus.as_deref()
            .map(|f| format!(" Focus particularly on: {}.", f))
            .unwrap_or_default();
        format!(
            "Explain the following {} code at {} depth.{}\n\n```{}\n{}\n```",
            req.language, req.depth.as_str(), focus_str, req.language, req.code
        )
    }

    /// Generate a placeholder explanation (for offline / testing use).
    pub fn mock_explain(req: &ExplainRequest) -> Explanation {
        let code_lines = req.code.lines().count();
        let title = format!("{} code ({} lines)", req.language, code_lines);
        let body = match req.depth {
            DepthLevel::Surface => format!("This {} snippet performs a computation.", req.language),
            DepthLevel::Overview => format!(
                "This {} code has {} lines. It defines logic that operates on input data and produces output.",
                req.language, code_lines
            ),
            DepthLevel::Deep => format!(
                "The {} code ({} lines) contains detailed logic. Key data flows from inputs through transformations to outputs. Edge cases should be considered.",
                req.language, code_lines
            ),
            DepthLevel::Expert => format!(
                "Expert analysis of {} ({} lines): algorithmic complexity is O(n) assuming linear scan. Memory usage is bounded. Consider concurrency implications if shared state is involved.",
                req.language, code_lines
            ),
        };

        let follow_ups = match req.depth {
            DepthLevel::Surface => vec!["What does this code do in more detail?".to_string()],
            DepthLevel::Overview => vec![
                "What are the edge cases?".to_string(),
                "How does the data flow through?".to_string(),
            ],
            DepthLevel::Deep => vec![
                "What is the time complexity?".to_string(),
                "Are there any security concerns?".to_string(),
            ],
            DepthLevel::Expert => vec![
                "How would this scale to 1M users?".to_string(),
                "What design patterns apply here?".to_string(),
                "What are the tradeoffs vs alternatives?".to_string(),
            ],
        };

        let complexity_hint = if req.depth >= DepthLevel::Deep {
            Some("O(n) — linear with input size (estimated)".to_string())
        } else {
            None
        };

        Explanation { depth: req.depth, title, body, complexity_hint, follow_up_questions: follow_ups }
    }

    /// Suggest a deeper depth level (returns None if already Expert).
    pub fn suggest_deeper(current: DepthLevel) -> Option<DepthLevel> {
        match current {
            DepthLevel::Surface => Some(DepthLevel::Overview),
            DepthLevel::Overview => Some(DepthLevel::Deep),
            DepthLevel::Deep => Some(DepthLevel::Expert),
            DepthLevel::Expert => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn req(depth: DepthLevel) -> ExplainRequest {
        ExplainRequest::new("fn add(a: i32, b: i32) -> i32 { a + b }", "rust", depth)
    }

    #[test]
    fn test_depth_from_str() {
        assert_eq!(DepthLevel::from_str("surface"), Some(DepthLevel::Surface));
        assert_eq!(DepthLevel::from_str("1"), Some(DepthLevel::Surface));
        assert_eq!(DepthLevel::from_str("expert"), Some(DepthLevel::Expert));
        assert_eq!(DepthLevel::from_str("unknown"), None);
    }

    #[test]
    fn test_depth_ordering() {
        assert!(DepthLevel::Surface < DepthLevel::Expert);
        assert!(DepthLevel::Deep > DepthLevel::Overview);
    }

    #[test]
    fn test_all_depths() {
        assert_eq!(DepthLevel::all().len(), 4);
    }

    #[test]
    fn test_audience_suggested_depth() {
        assert_eq!(Audience::Novice.suggested_depth(), DepthLevel::Surface);
        assert_eq!(Audience::Architect.suggested_depth(), DepthLevel::Expert);
    }

    #[test]
    fn test_system_prompt_changes_by_depth() {
        let s = CodeExplainer::system_prompt(DepthLevel::Surface, Audience::Developer);
        let d = CodeExplainer::system_prompt(DepthLevel::Deep, Audience::Developer);
        assert_ne!(s, d);
    }

    #[test]
    fn test_user_prompt_contains_language() {
        let r = req(DepthLevel::Overview);
        let p = CodeExplainer::user_prompt(&r);
        assert!(p.contains("rust"));
    }

    #[test]
    fn test_user_prompt_with_focus() {
        let r = req(DepthLevel::Deep).with_focus("error handling");
        let p = CodeExplainer::user_prompt(&r);
        assert!(p.contains("error handling"));
    }

    #[test]
    fn test_mock_surface_explanation() {
        let e = CodeExplainer::mock_explain(&req(DepthLevel::Surface));
        assert!(e.body.contains("rust"));
        assert_eq!(e.follow_up_questions.len(), 1);
    }

    #[test]
    fn test_mock_expert_has_complexity() {
        let e = CodeExplainer::mock_explain(&req(DepthLevel::Expert));
        assert!(e.complexity_hint.is_some());
        assert!(!e.follow_up_questions.is_empty());
    }

    #[test]
    fn test_suggest_deeper() {
        assert_eq!(CodeExplainer::suggest_deeper(DepthLevel::Surface), Some(DepthLevel::Overview));
        assert_eq!(CodeExplainer::suggest_deeper(DepthLevel::Expert), None);
    }

    #[test]
    fn test_depth_as_str() {
        assert_eq!(DepthLevel::Deep.as_str(), "deep");
    }

    #[test]
    fn test_explanation_depth_matches_request() {
        let e = CodeExplainer::mock_explain(&req(DepthLevel::Overview));
        assert_eq!(e.depth, DepthLevel::Overview);
    }

    #[test]
    fn test_explanation_title_contains_language() {
        let e = CodeExplainer::mock_explain(&req(DepthLevel::Surface));
        assert!(e.title.contains("rust"));
    }
}
