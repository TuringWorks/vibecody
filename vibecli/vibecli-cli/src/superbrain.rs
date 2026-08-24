#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuperBrainMode {
    SmartRouter,
    Consensus,
    ChainRelay,
    BestOfN,
    Specialist,
}

impl std::fmt::Display for SuperBrainMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SmartRouter => write!(f, "Smart Router"),
            Self::Consensus => write!(f, "Consensus"),
            Self::ChainRelay => write!(f, "Chain Relay"),
            Self::BestOfN => write!(f, "Best-of-N"),
            Self::Specialist => write!(f, "Specialist"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContribution {
    pub provider: String,
    pub model: String,
    pub role: String,
    pub content: String,
    pub duration_ms: u64,
    pub tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperBrainResult {
    pub mode: String,
    pub final_response: String,
    pub model_responses: Vec<ModelContribution>,
    pub routing_reason: Option<String>,
    pub total_duration_ms: u64,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperBrainConfig {
    pub providers: Vec<ProviderEntry>,
    pub judge: Option<ProviderEntry>,
    /// Category routing rules. Vendor selection lives in
    /// `[superbrain.routes]`, not here.
    pub routing_rules: Vec<CategoryRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub provider: String,
    pub model: String,
}

// ── Smart Router ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub provider: String,
    pub model: String,
    pub category: String,
    pub reason: String,
    pub confidence: f64,
}


// ── Category routing — which *kind* of task, not which vendor ────────────────

/// A keyword rule that names a task category and nothing else.
///
/// The rule this replaced carried a provider and a model, which baked a
/// vendor choice into the classifier: "this looks like code" and "code means
/// Claude" are different claims, and only the first is something keyword
/// matching can support. A category rule makes the second claim somebody
/// else's job — see [`resolve_route`], which answers it from configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRule {
    pub keywords: Vec<String>,
    pub category: String,
    pub priority: u32,
}

/// What the classifier decided, with no provider attached.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Classification {
    pub category: String,
    pub matched: Vec<String>,
    pub confidence: f64,
    pub reason: String,
}

/// Where a category should be sent. `model: None` means "whatever that
/// provider's configured default is" — absent, not guessed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteTarget {
    pub provider: String,
    pub model: Option<String>,
}

/// The task categories the router knows.
///
/// Keywords and a category. No provider, no model, no vendor of any kind —
/// see [`resolve_route`], which answers "which model" from configuration.
pub fn category_rules() -> Vec<CategoryRule> {
    vec![
        CategoryRule {
            keywords: vec!["implement", "function", "code", "debug", "fix", "bug", "refactor", "class", "struct", "async", "test", "compile", "error", "rust", "python", "javascript", "typescript"].into_iter().map(String::from).collect(),
            category: "code".into(),
            priority: 10,
        },
        CategoryRule {
            keywords: vec!["calculate", "equation", "prove", "solve", "integral", "derivative", "matrix", "algebra", "theorem", "probability", "statistics"].into_iter().map(String::from).collect(),
            category: "math".into(),
            priority: 10,
        },
        CategoryRule {
            keywords: vec!["write a story", "poem", "creative", "brainstorm", "imagine", "story", "narrative", "fiction", "design"].into_iter().map(String::from).collect(),
            category: "creative".into(),
            priority: 10,
        },
        CategoryRule {
            keywords: vec!["analyze", "compare", "evaluate", "review", "assess", "critique", "research", "explain"].into_iter().map(String::from).collect(),
            category: "analysis".into(),
            priority: 8,
        },
        CategoryRule {
            keywords: vec!["what is", "define", "who is", "when did", "where is", "list", "name"].into_iter().map(String::from).collect(),
            category: "factual".into(),
            priority: 5,
        },
    ]
}

/// Classify a prompt into a task category by keyword weight.
///
/// Returns `None` when nothing matched. That is deliberately not a "general"
/// category with a low confidence attached: an unmatched prompt is one the
/// classifier has no opinion about, and reporting 0.3 confidence in a guess
/// invites the caller to treat the guess as a weak signal rather than as no
/// signal at all.
pub fn classify(query: &str, rules: &[CategoryRule]) -> Option<Classification> {
    let lower = query.to_lowercase();
    let scored = rules.iter().filter_map(|rule| {
        let matched: Vec<String> = rule
            .keywords
            .iter()
            .filter(|k| lower.contains(&k.to_lowercase()))
            .cloned()
            .collect();
        match matched.is_empty() {
            true => None,
            false => Some((matched.len() as u32 * rule.priority, rule, matched)),
        }
    });

    let (score, rule, matched) = scored.max_by_key(|(score, _, _)| *score)?;
    Some(Classification {
        category: rule.category.clone(),
        confidence: (f64::from(score) / 30.0).min(1.0),
        reason: format!(
            "matched {} keyword{} [{}] → category '{}'",
            matched.len(),
            if matched.len() == 1 { "" } else { "s" },
            matched.join(", "),
            rule.category
        ),
        matched,
    })
}

/// Why a route ended up where it did — shown to the user so a surprising
/// choice can be traced to the setting that caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSource {
    /// A `[superbrain.routes.<category>]` entry matched the classification.
    ConfiguredCategory,
    /// The prompt classified, but that category has no configured route.
    SessionDefaultUnconfiguredCategory,
    /// Nothing classified; the session's provider handles it.
    SessionDefaultUnclassified,
}

/// The route a prompt resolves to, and the reason for it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRoute {
    pub target: RouteTarget,
    pub category: Option<String>,
    pub source: RouteSource,
    pub reason: String,
}

/// Resolve a prompt to a provider and model.
///
/// The vendor comes from `routes` — the user's `[superbrain.routes]` config —
/// or from `session`, the provider and model the REPL is already using. It
/// never comes from a table compiled into this binary. That is the whole
/// point: a router that always picks one vendor is not routing, and a default
/// that names a vendor is a recommendation the user did not ask for.
pub fn resolve_route(
    query: &str,
    rules: &[CategoryRule],
    routes: &[(String, RouteTarget)],
    session: &RouteTarget,
) -> ResolvedRoute {
    let lookup = |category: &str| {
        routes
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(category))
            .map(|(_, t)| t.clone())
    };

    match classify(query, rules) {
        Some(class) => match lookup(&class.category) {
            Some(target) => ResolvedRoute {
                reason: format!(
                    "{} → {}{}",
                    class.reason,
                    target.provider,
                    target
                        .model
                        .as_deref()
                        .map(|m| format!("/{m}"))
                        .unwrap_or_default()
                ),
                target,
                category: Some(class.category),
                source: RouteSource::ConfiguredCategory,
            },
            None => ResolvedRoute {
                reason: format!(
                    "{}, which has no [superbrain.routes.{}] entry — using the session provider",
                    class.reason, class.category
                ),
                target: session.clone(),
                category: Some(class.category),
                source: RouteSource::SessionDefaultUnconfiguredCategory,
            },
        },
        None => ResolvedRoute {
            target: session.clone(),
            category: None,
            source: RouteSource::SessionDefaultUnclassified,
            reason: "no category keywords matched — using the session provider".into(),
        },
    }
}

// ── Prompt Builders for Each Mode ──

pub struct SuperBrainPrompts;

impl SuperBrainPrompts {
    /// Build the chain relay prompt for model N, including all previous model outputs.
    pub fn chain_relay_prompt(
        query: &str,
        previous: &[ModelContribution],
        step: usize,
        total: usize,
    ) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let role_label = match step {
            0 => "Initial Analyst",
            s if s == total - 1 => "Final Synthesizer",
            _ => "Critical Reviewer",
        };

        let mut system = format!(
            "You are step {} of {} in a chain-of-thought relay. Your role: {}.",
            step + 1,
            total,
            role_label
        );
        if step > 0 {
            system.push_str("\nBuild upon the previous analysis. Add depth, correct errors, and refine the reasoning.");
        }

        let mut user_content = format!("Original query: {}\n", query);
        for (i, prev) in previous.iter().enumerate() {
            user_content.push_str(&format!(
                "\n--- Step {} ({} / {}) ---\n{}\n",
                i + 1,
                prev.provider,
                prev.model,
                prev.content
            ));
        }
        if step > 0 {
            user_content.push_str(&format!(
                "\nAs the {}, provide your refined analysis:",
                role_label
            ));
        }

        vec![
            Message {
                role: MessageRole::System,
                content: system,
            },
            Message {
                role: MessageRole::User,
                content: user_content,
            },
        ]
    }

    /// Build the judge prompt for Best-of-N mode.
    pub fn best_of_n_judge_prompt(
        query: &str,
        responses: &[ModelContribution],
    ) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!("You are a judge evaluating multiple AI responses to the following query:\n\n\"{}\"\n\nHere are the responses:\n", query);
        for (i, resp) in responses.iter().enumerate() {
            content.push_str(&format!(
                "\n--- Response {} ({}/{}) ---\n{}\n",
                i + 1,
                resp.provider,
                resp.model,
                resp.content
            ));
        }
        content.push_str("\nEvaluate each response for accuracy, completeness, clarity, and helpfulness. Then:\n1. Rank all responses from best to worst\n2. Explain why the best response is superior\n3. Provide the best response (or an improved version combining the best elements)\n\nFormat your answer as:\nWINNER: [number]\nREASON: [explanation]\nBEST RESPONSE:\n[the winning or improved response]");

        vec![
            Message {
                role: MessageRole::System,
                content:
                    "You are an impartial judge evaluating AI responses. Be objective and thorough."
                        .into(),
            },
            Message {
                role: MessageRole::User,
                content,
            },
        ]
    }

    /// Build consensus synthesis prompt.
    pub fn consensus_prompt(
        query: &str,
        responses: &[ModelContribution],
    ) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!(
            "Multiple AI models were asked: \"{}\"\n\nTheir responses:\n",
            query
        );
        for (i, resp) in responses.iter().enumerate() {
            content.push_str(&format!(
                "\n--- Model {} ({}/{}) ---\n{}\n",
                i + 1,
                resp.provider,
                resp.model,
                resp.content
            ));
        }
        content.push_str("\nSynthesize these into a single comprehensive response that:\n1. Identifies points of agreement (consensus)\n2. Notes any disagreements\n3. Produces the best possible answer combining all perspectives\n4. Reports the agreement level (e.g., \"4/5 models agree that...\")\n\nProvide the synthesized response:");

        vec![
            Message { role: MessageRole::System, content: "You are synthesizing responses from multiple AI models into one optimal answer.".into() },
            Message { role: MessageRole::User, content },
        ]
    }

    /// Build specialist decomposition prompt.
    pub fn specialist_decompose_prompt(query: &str) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};
        vec![
            Message { role: MessageRole::System, content: "You are a task decomposer. Break complex queries into 2-5 independent subtasks that can be handled by different specialists.".into() },
            Message { role: MessageRole::User, content: format!("Decompose this query into subtasks:\n\n{}\n\nReturn ONLY a numbered list of subtasks, one per line. Example:\n1. Research the background\n2. Analyze the technical approach\n3. Evaluate alternatives", query) },
        ]
    }

    /// Build specialist merge prompt.
    pub fn specialist_merge_prompt(
        query: &str,
        subtask_results: &[(String, ModelContribution)],
    ) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!("Original query: {}\n\nSubtask results:\n", query);
        for (subtask, result) in subtask_results {
            content.push_str(&format!(
                "\n--- Subtask: {} ---\n[Handled by {}/{}]\n{}\n",
                subtask, result.provider, result.model, result.content
            ));
        }
        content.push_str("\nMerge these subtask results into one cohesive, comprehensive response to the original query:");

        vec![
            Message { role: MessageRole::System, content: "You are merging specialist results into a unified response. Ensure coherence and completeness.".into() },
            Message { role: MessageRole::User, content },
        ]
    }
}

/// Parse subtask list from decomposition response.
pub fn parse_subtasks(response: &str) -> Vec<String> {
    response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Strip leading number + dot/paren
            let content = trimmed
                .trim_start_matches(|c: char| {
                    c.is_ascii_digit() || c == '.' || c == ')' || c == '-'
                })
                .trim();
            if content.is_empty() {
                None
            } else {
                Some(content.to_string())
            }
        })
        .take(5) // max 5 subtasks
        .collect()
}

/// Available SuperBrain modes with descriptions.
pub fn available_modes() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Smart Router",
            "Routes query to the best model based on task type",
        ),
        (
            "Consensus",
            "Sends to all models, synthesizes the majority view",
        ),
        (
            "Chain Relay",
            "Sequential refinement: each model builds on the previous",
        ),
        ("Best-of-N", "All models respond, a judge picks the best"),
        (
            "Specialist",
            "Decomposes into subtasks, assigns to different models",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;







    #[test]
    fn test_chain_relay_prompt_step0() {
        let msgs = SuperBrainPrompts::chain_relay_prompt("What is AI?", &[], 0, 3);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].content.contains("step 1 of 3"));
        assert!(msgs[0].content.contains("Initial Analyst"));
    }

    #[test]
    fn test_chain_relay_prompt_step1() {
        let prev = vec![ModelContribution {
            provider: "claude".into(),
            model: "sonnet".into(),
            role: "primary".into(),
            content: "AI is...".into(),
            duration_ms: 100,
            tokens: Some(10),
        }];
        let msgs = SuperBrainPrompts::chain_relay_prompt("What is AI?", &prev, 1, 3);
        assert!(msgs[1].content.contains("AI is..."));
        assert!(msgs[0].content.contains("Critical Reviewer"));
    }

    #[test]
    fn test_chain_relay_prompt_final() {
        let prev = vec![
            ModelContribution {
                provider: "a".into(),
                model: "m1".into(),
                role: "p".into(),
                content: "Step 1".into(),
                duration_ms: 100,
                tokens: None,
            },
            ModelContribution {
                provider: "b".into(),
                model: "m2".into(),
                role: "p".into(),
                content: "Step 2".into(),
                duration_ms: 100,
                tokens: None,
            },
        ];
        let msgs = SuperBrainPrompts::chain_relay_prompt("Q", &prev, 2, 3);
        assert!(msgs[0].content.contains("Final Synthesizer"));
    }

    #[test]
    fn test_best_of_n_judge_prompt() {
        let responses = vec![
            ModelContribution {
                provider: "a".into(),
                model: "m1".into(),
                role: "p".into(),
                content: "Answer A".into(),
                duration_ms: 100,
                tokens: None,
            },
            ModelContribution {
                provider: "b".into(),
                model: "m2".into(),
                role: "p".into(),
                content: "Answer B".into(),
                duration_ms: 200,
                tokens: None,
            },
        ];
        let msgs = SuperBrainPrompts::best_of_n_judge_prompt("Q?", &responses);
        assert!(msgs[1].content.contains("Answer A"));
        assert!(msgs[1].content.contains("Answer B"));
        assert!(msgs[1].content.contains("WINNER"));
    }

    #[test]
    fn test_consensus_prompt() {
        let responses = vec![ModelContribution {
            provider: "a".into(),
            model: "m1".into(),
            role: "p".into(),
            content: "Yes".into(),
            duration_ms: 100,
            tokens: None,
        }];
        let msgs = SuperBrainPrompts::consensus_prompt("Is water wet?", &responses);
        assert!(msgs[1].content.contains("Is water wet?"));
        assert!(msgs[1].content.contains("Yes"));
        assert!(msgs[1].content.contains("agreement"));
    }

    #[test]
    fn test_parse_subtasks() {
        let response = "1. Research the topic\n2. Analyze the data\n3. Write conclusions\n";
        let tasks = parse_subtasks(response);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0], "Research the topic");
        assert_eq!(tasks[2], "Write conclusions");
    }

    #[test]
    fn test_parse_subtasks_max_5() {
        let response = "1. A\n2. B\n3. C\n4. D\n5. E\n6. F\n7. G";
        let tasks = parse_subtasks(response);
        assert_eq!(tasks.len(), 5);
    }

    #[test]
    fn test_parse_subtasks_various_formats() {
        let response = "- Research background\n- Analyze code\n- Write tests";
        let tasks = parse_subtasks(response);
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn test_specialist_prompts() {
        let msgs = SuperBrainPrompts::specialist_decompose_prompt("Build a web app");
        assert!(msgs[1].content.contains("Build a web app"));
        assert!(msgs[1].content.contains("subtasks"));
    }

    #[test]
    fn test_specialist_merge_prompt() {
        let results = vec![(
            "Research".into(),
            ModelContribution {
                provider: "a".into(),
                model: "m".into(),
                role: "s".into(),
                content: "Found...".into(),
                duration_ms: 100,
                tokens: None,
            },
        )];
        let msgs = SuperBrainPrompts::specialist_merge_prompt("Build app", &results);
        assert!(msgs[1].content.contains("Research"));
        assert!(msgs[1].content.contains("Found..."));
    }

    #[test]
    fn test_available_modes() {
        let modes = available_modes();
        assert_eq!(modes.len(), 5);
        assert!(modes.iter().any(|(n, _)| *n == "Smart Router"));
        assert!(modes.iter().any(|(n, _)| *n == "Chain Relay"));
    }


    #[test]
    fn test_superbrain_mode_display() {
        assert_eq!(SuperBrainMode::SmartRouter.to_string(), "Smart Router");
        assert_eq!(SuperBrainMode::BestOfN.to_string(), "Best-of-N");
        assert_eq!(SuperBrainMode::ChainRelay.to_string(), "Chain Relay");
    }

    // ── Category routing: classify the task, configure the vendor ───────────

    fn session() -> RouteTarget {
        RouteTarget { provider: "ollama".into(), model: Some("llama3.2".into()) }
    }

    #[test]
    fn a_coding_prompt_classifies_as_code() {
        let class = classify("Write a binary search in Rust", &category_rules())
            .expect("a coding prompt must classify");
        assert_eq!(class.category, "code");
        assert!(class.matched.iter().any(|k| k == "rust"), "{:?}", class.matched);
    }

    /// The point of the whole exercise: the category is keyword-derived, the
    /// vendor is not. Nothing in this crate may answer "code means Claude".
    #[test]
    fn no_category_rule_names_a_vendor() {
        let json = serde_json::to_string(&category_rules()).expect("serialize");
        for vendor in ["claude", "anthropic", "openai", "gpt", "gemini", "groq", "ollama"] {
            assert!(
                !json.to_lowercase().contains(vendor),
                "category rules must not name '{vendor}': {json}"
            );
        }
    }

    #[test]
    fn a_configured_category_wins() {
        let routes = vec![(
            "code".to_string(),
            RouteTarget { provider: "deepseek".into(), model: Some("deepseek-coder".into()) },
        )];
        let r = resolve_route("Write a binary search in Rust", &category_rules(), &routes, &session());
        assert_eq!(r.source, RouteSource::ConfiguredCategory);
        assert_eq!(r.target.provider, "deepseek");
        assert_eq!(r.category.as_deref(), Some("code"));
    }

    /// Zero-config: a classified prompt with no route for its category runs on
    /// whatever the session is already using, and says so. It must never fall
    /// back to a vendor this binary picked.
    #[test]
    fn an_unconfigured_category_uses_the_session_provider() {
        let r = resolve_route("Write a binary search in Rust", &category_rules(), &[], &session());
        assert_eq!(r.source, RouteSource::SessionDefaultUnconfiguredCategory);
        assert_eq!(r.target, session());
        assert!(r.reason.contains("superbrain.routes.code"), "{}", r.reason);
    }

    #[test]
    fn an_unclassified_prompt_uses_the_session_provider_and_no_category() {
        let r = resolve_route("zzzz qqqq", &category_rules(), &[], &session());
        assert_eq!(r.source, RouteSource::SessionDefaultUnclassified);
        assert_eq!(r.category, None);
        assert_eq!(r.target, session());
    }

    /// Unmatched is absent, not a low-confidence guess.
    #[test]
    fn nothing_matched_classifies_as_none() {
        assert!(classify("zzzz qqqq", &category_rules()).is_none());
    }

    #[test]
    fn category_lookup_ignores_case() {
        let routes = vec![(
            "CODE".to_string(),
            RouteTarget { provider: "mistral".into(), model: None },
        )];
        let r = resolve_route("refactor this function", &category_rules(), &routes, &session());
        assert_eq!(r.target.provider, "mistral");
        assert_eq!(r.target.model, None, "an unset model must stay unset, not be invented");
    }

}
