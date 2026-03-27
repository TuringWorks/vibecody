#![allow(dead_code)]

use serde::{Serialize, Deserialize};

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
pub struct RoutingRule {
    pub keywords: Vec<String>,
    pub category: String,
    pub provider: String,
    pub model: String,
    pub priority: u32,
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
    pub routing_rules: Vec<RoutingRule>,
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

pub struct SmartRouter;

impl SmartRouter {
    /// Default routing rules mapping task categories to providers.
    pub fn default_rules() -> Vec<RoutingRule> {
        vec![
            RoutingRule { keywords: vec!["implement".into(), "function".into(), "code".into(), "debug".into(), "fix".into(), "bug".into(), "refactor".into(), "class".into(), "struct".into(), "async".into(), "test".into(), "compile".into(), "error".into(), "rust".into(), "python".into(), "javascript".into(), "typescript".into()], category: "code".into(), provider: "claude".into(), model: "claude-3.5-sonnet".into(), priority: 10 },
            RoutingRule { keywords: vec!["calculate".into(), "equation".into(), "prove".into(), "solve".into(), "integral".into(), "derivative".into(), "matrix".into(), "algebra".into(), "theorem".into(), "probability".into(), "statistics".into()], category: "math".into(), provider: "openai".into(), model: "gpt-4o".into(), priority: 10 },
            RoutingRule { keywords: vec!["write a story".into(), "poem".into(), "creative".into(), "brainstorm".into(), "imagine".into(), "story".into(), "narrative".into(), "fiction".into(), "design".into()], category: "creative".into(), provider: "gemini".into(), model: "gemini-2.0-flash".into(), priority: 10 },
            RoutingRule { keywords: vec!["analyze".into(), "compare".into(), "evaluate".into(), "review".into(), "assess".into(), "critique".into(), "research".into(), "explain".into()], category: "analysis".into(), provider: "claude".into(), model: "claude-3.5-sonnet".into(), priority: 8 },
            RoutingRule { keywords: vec!["what is".into(), "define".into(), "who is".into(), "when did".into(), "where is".into(), "list".into(), "name".into()], category: "factual".into(), provider: "groq".into(), model: "llama-3.3-70b-versatile".into(), priority: 5 },
        ]
    }

    /// Route a query to the best provider based on keyword matching.
    pub fn route(query: &str, rules: &[RoutingRule]) -> RoutingDecision {
        let lower = query.to_lowercase();
        let mut best_score = 0u32;
        let mut best_rule: Option<&RoutingRule> = None;
        let mut matched_keywords: Vec<String> = Vec::new();

        for rule in rules {
            let mut score = 0u32;
            let mut matches: Vec<String> = Vec::new();
            for keyword in &rule.keywords {
                if lower.contains(&keyword.to_lowercase()) {
                    score += rule.priority;
                    matches.push(keyword.clone());
                }
            }
            if score > best_score {
                best_score = score;
                best_rule = Some(rule);
                matched_keywords = matches;
            }
        }

        if let Some(rule) = best_rule {
            RoutingDecision {
                provider: rule.provider.clone(),
                model: rule.model.clone(),
                category: rule.category.clone(),
                reason: format!("Matched {} keywords [{}] in category '{}'", matched_keywords.len(), matched_keywords.join(", "), rule.category),
                confidence: (best_score as f64 / 30.0).min(1.0),
            }
        } else {
            // Default fallback
            RoutingDecision {
                provider: "ollama".into(),
                model: "llama3.2".into(),
                category: "general".into(),
                reason: "No specific category matched — using general-purpose model".into(),
                confidence: 0.3,
            }
        }
    }
}

// ── Prompt Builders for Each Mode ──

pub struct SuperBrainPrompts;

impl SuperBrainPrompts {
    /// Build the chain relay prompt for model N, including all previous model outputs.
    pub fn chain_relay_prompt(query: &str, previous: &[ModelContribution], step: usize, total: usize) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let role_label = match step {
            0 => "Initial Analyst",
            s if s == total - 1 => "Final Synthesizer",
            _ => "Critical Reviewer",
        };

        let mut system = format!("You are step {} of {} in a chain-of-thought relay. Your role: {}.", step + 1, total, role_label);
        if step > 0 {
            system.push_str("\nBuild upon the previous analysis. Add depth, correct errors, and refine the reasoning.");
        }

        let mut user_content = format!("Original query: {}\n", query);
        for (i, prev) in previous.iter().enumerate() {
            user_content.push_str(&format!("\n--- Step {} ({} / {}) ---\n{}\n", i + 1, prev.provider, prev.model, prev.content));
        }
        if step > 0 {
            user_content.push_str(&format!("\nAs the {}, provide your refined analysis:", role_label));
        }

        vec![
            Message { role: MessageRole::System, content: system },
            Message { role: MessageRole::User, content: user_content },
        ]
    }

    /// Build the judge prompt for Best-of-N mode.
    pub fn best_of_n_judge_prompt(query: &str, responses: &[ModelContribution]) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!("You are a judge evaluating multiple AI responses to the following query:\n\n\"{}\"\n\nHere are the responses:\n", query);
        for (i, resp) in responses.iter().enumerate() {
            content.push_str(&format!("\n--- Response {} ({}/{}) ---\n{}\n", i + 1, resp.provider, resp.model, resp.content));
        }
        content.push_str("\nEvaluate each response for accuracy, completeness, clarity, and helpfulness. Then:\n1. Rank all responses from best to worst\n2. Explain why the best response is superior\n3. Provide the best response (or an improved version combining the best elements)\n\nFormat your answer as:\nWINNER: [number]\nREASON: [explanation]\nBEST RESPONSE:\n[the winning or improved response]");

        vec![
            Message { role: MessageRole::System, content: "You are an impartial judge evaluating AI responses. Be objective and thorough.".into() },
            Message { role: MessageRole::User, content },
        ]
    }

    /// Build consensus synthesis prompt.
    pub fn consensus_prompt(query: &str, responses: &[ModelContribution]) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!("Multiple AI models were asked: \"{}\"\n\nTheir responses:\n", query);
        for (i, resp) in responses.iter().enumerate() {
            content.push_str(&format!("\n--- Model {} ({}/{}) ---\n{}\n", i + 1, resp.provider, resp.model, resp.content));
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
    pub fn specialist_merge_prompt(query: &str, subtask_results: &[(String, ModelContribution)]) -> Vec<vibe_ai::provider::Message> {
        use vibe_ai::provider::{Message, MessageRole};

        let mut content = format!("Original query: {}\n\nSubtask results:\n", query);
        for (subtask, result) in subtask_results {
            content.push_str(&format!("\n--- Subtask: {} ---\n[Handled by {}/{}]\n{}\n", subtask, result.provider, result.model, result.content));
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
    response.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() { return None; }
            // Strip leading number + dot/paren
            let content = trimmed
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == '-')
                .trim();
            if content.is_empty() { None } else { Some(content.to_string()) }
        })
        .take(5) // max 5 subtasks
        .collect()
}

/// Available SuperBrain modes with descriptions.
pub fn available_modes() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Smart Router", "Routes query to the best model based on task type"),
        ("Consensus", "Sends to all models, synthesizes the majority view"),
        ("Chain Relay", "Sequential refinement: each model builds on the previous"),
        ("Best-of-N", "All models respond, a judge picks the best"),
        ("Specialist", "Decomposes into subtasks, assigns to different models"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_code_query() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("Implement a binary search function in Rust", &rules);
        assert_eq!(decision.category, "code");
        assert!(decision.confidence > 0.0);
    }

    #[test]
    fn test_route_math_query() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("Solve this integral: ∫x²dx", &rules);
        assert_eq!(decision.category, "math");
    }

    #[test]
    fn test_route_creative_query() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("Write a story about a robot learning to paint", &rules);
        assert_eq!(decision.category, "creative");
    }

    #[test]
    fn test_route_factual_query() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("What is the capital of France?", &rules);
        assert_eq!(decision.category, "factual");
    }

    #[test]
    fn test_route_no_match_fallback() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("xyz 123 qqq", &rules);
        assert_eq!(decision.category, "general");
        assert!(decision.confidence < 0.5);
    }

    #[test]
    fn test_route_multi_keyword() {
        let rules = SmartRouter::default_rules();
        let decision = SmartRouter::route("Debug this async function and fix the compile error", &rules);
        assert_eq!(decision.category, "code");
        assert!(decision.confidence > 0.5);
    }

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
            provider: "claude".into(), model: "sonnet".into(),
            role: "primary".into(), content: "AI is...".into(),
            duration_ms: 100, tokens: Some(10),
        }];
        let msgs = SuperBrainPrompts::chain_relay_prompt("What is AI?", &prev, 1, 3);
        assert!(msgs[1].content.contains("AI is..."));
        assert!(msgs[0].content.contains("Critical Reviewer"));
    }

    #[test]
    fn test_chain_relay_prompt_final() {
        let prev = vec![
            ModelContribution { provider: "a".into(), model: "m1".into(), role: "p".into(), content: "Step 1".into(), duration_ms: 100, tokens: None },
            ModelContribution { provider: "b".into(), model: "m2".into(), role: "p".into(), content: "Step 2".into(), duration_ms: 100, tokens: None },
        ];
        let msgs = SuperBrainPrompts::chain_relay_prompt("Q", &prev, 2, 3);
        assert!(msgs[0].content.contains("Final Synthesizer"));
    }

    #[test]
    fn test_best_of_n_judge_prompt() {
        let responses = vec![
            ModelContribution { provider: "a".into(), model: "m1".into(), role: "p".into(), content: "Answer A".into(), duration_ms: 100, tokens: None },
            ModelContribution { provider: "b".into(), model: "m2".into(), role: "p".into(), content: "Answer B".into(), duration_ms: 200, tokens: None },
        ];
        let msgs = SuperBrainPrompts::best_of_n_judge_prompt("Q?", &responses);
        assert!(msgs[1].content.contains("Answer A"));
        assert!(msgs[1].content.contains("Answer B"));
        assert!(msgs[1].content.contains("WINNER"));
    }

    #[test]
    fn test_consensus_prompt() {
        let responses = vec![
            ModelContribution { provider: "a".into(), model: "m1".into(), role: "p".into(), content: "Yes".into(), duration_ms: 100, tokens: None },
        ];
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
        let results = vec![
            ("Research".into(), ModelContribution { provider: "a".into(), model: "m".into(), role: "s".into(), content: "Found...".into(), duration_ms: 100, tokens: None }),
        ];
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
    fn test_default_rules_coverage() {
        let rules = SmartRouter::default_rules();
        assert!(rules.len() >= 5);
        let categories: Vec<&str> = rules.iter().map(|r| r.category.as_str()).collect();
        assert!(categories.contains(&"code"));
        assert!(categories.contains(&"math"));
        assert!(categories.contains(&"creative"));
        assert!(categories.contains(&"factual"));
    }

    #[test]
    fn test_superbrain_mode_display() {
        assert_eq!(SuperBrainMode::SmartRouter.to_string(), "Smart Router");
        assert_eq!(SuperBrainMode::BestOfN.to_string(), "Best-of-N");
        assert_eq!(SuperBrainMode::ChainRelay.to_string(), "Chain Relay");
    }
}
