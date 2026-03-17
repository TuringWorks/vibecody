use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConversation {
    pub id: String,
    pub query_history: Vec<SearchQuery>,
    pub results: Vec<SearchResult>,
    pub context_window: SearchContext,
    pub follow_up_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub filters: SearchFilters,
    pub query_type: QueryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub file_type: Option<String>,
    pub path: Option<String>,
    pub date_range: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Natural,
    Regex,
    Semantic,
    FollowUp,
    Refinement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub line_range: (usize, usize),
    pub snippet: String,
    pub relevance_score: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnswer {
    pub summary: String,
    pub evidence: Vec<SearchResult>,
    pub confidence: f64,
    pub follow_ups: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchContext {
    pub topic: Option<String>,
    pub relevant_files: Vec<String>,
    pub accumulated_facts: Vec<String>,
    pub turn_count: usize,
}


// ---------------------------------------------------------------------------
// Answer synthesizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AnswerSynthesizer;

impl AnswerSynthesizer {
    pub fn new() -> Self {
        Self
    }

    /// Combine multiple search results into a coherent answer.
    pub fn synthesize(
        &self,
        query: &str,
        results: &[SearchResult],
        context: &SearchContext,
    ) -> SearchAnswer {
        let facts = Self::extract_facts(results);
        let confidence = Self::compute_confidence(results);
        let summary = Self::build_summary(query, &facts, context);
        let follow_ups = Self::suggest_follow_ups(query, results, &facts);

        SearchAnswer {
            summary,
            evidence: results.to_vec(),
            confidence,
            follow_ups,
        }
    }

    /// Extract key facts from code snippets.
    fn extract_facts(results: &[SearchResult]) -> Vec<String> {
        results
            .iter()
            .filter(|r| r.relevance_score > 0.3)
            .map(|r| {
                format!(
                    "{} ({}:{}-{})",
                    r.explanation, r.file_path, r.line_range.0, r.line_range.1
                )
            })
            .collect()
    }

    /// Confidence based on aggregate relevance of results.
    fn compute_confidence(results: &[SearchResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        let total: f64 = results.iter().map(|r| r.relevance_score).sum();
        let avg = total / results.len() as f64;
        // Boost slightly when there are many corroborating results.
        let breadth_bonus = (results.len() as f64 / 10.0).min(0.15);
        (avg + breadth_bonus).min(1.0)
    }

    /// Build a natural-language summary from extracted facts.
    fn build_summary(query: &str, facts: &[String], context: &SearchContext) -> String {
        if facts.is_empty() {
            return format!("No relevant results found for \"{}\".", query);
        }

        let mut summary = format!(
            "Regarding \"{}\": found {} relevant code locations.",
            query,
            facts.len()
        );

        if let Some(ref topic) = context.topic {
            summary.push_str(&format!(" Context topic: {}.", topic));
        }

        for (i, fact) in facts.iter().enumerate().take(5) {
            summary.push_str(&format!(" [{}] {}", i + 1, fact));
        }

        summary
    }

    /// Suggest follow-up questions based on gaps.
    fn suggest_follow_ups(
        query: &str,
        results: &[SearchResult],
        _facts: &[String],
    ) -> Vec<String> {
        let mut suggestions: Vec<String> = Vec::new();

        // Suggest exploring related files.
        let mut seen_dirs: Vec<String> = Vec::new();
        for r in results {
            if let Some(dir) = r.file_path.rsplit_once('/').map(|(d, _)| d.to_string()) {
                if !seen_dirs.contains(&dir) {
                    suggestions.push(format!("What other modules exist in {}?", dir));
                    seen_dirs.push(dir);
                }
            }
        }

        // Suggest digging deeper.
        if !results.is_empty() {
            suggestions.push(format!(
                "How is the code related to \"{}\" tested?",
                query
            ));
            suggestions.push(format!(
                "What calls or depends on the results for \"{}\"?",
                query
            ));
        }

        suggestions.truncate(5);
        suggestions
    }
}

// ---------------------------------------------------------------------------
// Conversational search engine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConversationalSearchEngine {
    conversation: SearchConversation,
    history: Vec<(SearchQuery, SearchAnswer)>,
    synthesizer: AnswerSynthesizer,
    /// Simple in-memory index: keyword → vec of (file, line, snippet, explanation).
    index: HashMap<String, Vec<(String, usize, String, String)>>,
}

impl ConversationalSearchEngine {
    pub fn new() -> Self {
        Self {
            conversation: SearchConversation {
                id: Self::generate_id(),
                query_history: Vec::new(),
                results: Vec::new(),
                context_window: SearchContext::default(),
                follow_up_suggestions: Vec::new(),
            },
            history: Vec::new(),
            synthesizer: AnswerSynthesizer::new(),
            index: HashMap::new(),
        }
    }

    /// Seed the engine with indexable entries (for testing / integration).
    pub fn add_to_index(
        &mut self,
        keyword: &str,
        file_path: &str,
        line: usize,
        snippet: &str,
        explanation: &str,
    ) {
        self.index
            .entry(keyword.to_lowercase())
            .or_default()
            .push((
                file_path.to_string(),
                line,
                snippet.to_string(),
                explanation.to_string(),
            ));
    }

    /// Primary entry point: ask a free-form question.
    pub fn ask(&mut self, query: &str) -> SearchAnswer {
        let sq = SearchQuery {
            text: query.to_string(),
            filters: SearchFilters::default(),
            query_type: QueryType::Natural,
        };
        self.execute_query(sq)
    }

    /// Follow-up that carries forward the conversation context.
    pub fn follow_up(&mut self, query: &str) -> SearchAnswer {
        let sq = SearchQuery {
            text: query.to_string(),
            filters: SearchFilters::default(),
            query_type: QueryType::FollowUp,
        };
        self.execute_query(sq)
    }

    /// Narrow previous results using additional filters.
    pub fn refine(&mut self, filters: SearchFilters) -> SearchAnswer {
        let last_text = self
            .conversation
            .query_history
            .last()
            .map(|q| q.text.clone())
            .unwrap_or_default();

        let sq = SearchQuery {
            text: last_text,
            filters,
            query_type: QueryType::Refinement,
        };
        self.execute_query(sq)
    }

    /// Return full conversation history.
    pub fn history(&self) -> Vec<(SearchQuery, SearchAnswer)> {
        self.history.clone()
    }

    /// Suggest follow-up questions given the current context.
    pub fn suggest_questions(&self) -> Vec<String> {
        self.conversation.follow_up_suggestions.clone()
    }

    /// Reset conversation state.
    pub fn clear_context(&mut self) {
        self.conversation.context_window = SearchContext::default();
        self.conversation.query_history.clear();
        self.conversation.results.clear();
        self.conversation.follow_up_suggestions.clear();
        // history is intentionally preserved across clears
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn execute_query(&mut self, query: SearchQuery) -> SearchAnswer {
        // Update context topic on first query or when the topic changes.
        if self.conversation.context_window.topic.is_none()
            || query.query_type == QueryType::Natural
        {
            self.conversation.context_window.topic = Some(query.text.clone());
        }
        self.conversation.context_window.turn_count += 1;

        let mut results = self.search_index(&query);

        // For refinement, intersect with previous results.
        if query.query_type == QueryType::Refinement && !self.conversation.results.is_empty() {
            let prev_files: Vec<String> = self
                .conversation
                .results
                .iter()
                .map(|r| r.file_path.clone())
                .collect();
            results.retain(|r| prev_files.contains(&r.file_path));
        }

        // Apply filters.
        results = Self::apply_filters(results, &query.filters);

        // Sort by relevance descending.
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Accumulate relevant files in context.
        for r in &results {
            if !self
                .conversation
                .context_window
                .relevant_files
                .contains(&r.file_path)
            {
                self.conversation
                    .context_window
                    .relevant_files
                    .push(r.file_path.clone());
            }
        }

        let answer =
            self.synthesizer
                .synthesize(&query.text, &results, &self.conversation.context_window);

        self.conversation.results = results;
        self.conversation.follow_up_suggestions = answer.follow_ups.clone();
        self.conversation.query_history.push(query.clone());
        self.history.push((query, answer.clone()));

        answer
    }

    fn search_index(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let text_lower = query.text.to_lowercase();
        let keywords: Vec<&str> = text_lower.split_whitespace().collect();

        let mut scored: HashMap<String, SearchResult> = HashMap::new();

        for kw in &keywords {
            if let Some(entries) = self.index.get(*kw) {
                for (file, line, snippet, explanation) in entries {
                    let key = format!("{}:{}", file, line);
                    let entry = scored.entry(key).or_insert_with(|| SearchResult {
                        file_path: file.clone(),
                        line_range: (*line, line + snippet.lines().count()),
                        snippet: snippet.clone(),
                        relevance_score: 0.0,
                        explanation: explanation.clone(),
                    });
                    // Each matching keyword boosts relevance.
                    entry.relevance_score += 1.0 / keywords.len() as f64;
                }
            }
        }

        // Boost results that appear in context-relevant files.
        for r in scored.values_mut() {
            if self
                .conversation
                .context_window
                .relevant_files
                .contains(&r.file_path)
            {
                r.relevance_score = (r.relevance_score + 0.1).min(1.0);
            }
        }

        scored.into_values().collect()
    }

    fn apply_filters(mut results: Vec<SearchResult>, filters: &SearchFilters) -> Vec<SearchResult> {
        if let Some(ref ft) = filters.file_type {
            results.retain(|r| r.file_path.ends_with(ft));
        }
        if let Some(ref path) = filters.path {
            results.retain(|r| r.file_path.contains(path));
        }
        // date_range filtering would require filesystem metadata; skipped for in-memory index.
        results
    }

    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("conv-{}", ts)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_engine() -> ConversationalSearchEngine {
        let mut engine = ConversationalSearchEngine::new();
        engine.add_to_index(
            "agent",
            "src/agent.rs",
            10,
            "pub struct Agent { ... }",
            "Agent struct definition",
        );
        engine.add_to_index(
            "agent",
            "src/agent.rs",
            50,
            "impl Agent { pub fn run() }",
            "Agent run method",
        );
        engine.add_to_index(
            "provider",
            "src/provider.rs",
            1,
            "pub trait AIProvider { ... }",
            "AIProvider trait",
        );
        engine.add_to_index(
            "provider",
            "src/provider.rs",
            30,
            "fn complete(&self, prompt: &str)",
            "Provider complete method",
        );
        engine.add_to_index(
            "config",
            "src/config.rs",
            5,
            "pub struct Config { api_key: String }",
            "Config struct",
        );
        engine.add_to_index(
            "tool",
            "src/tool_executor.rs",
            20,
            "pub fn execute_tool(name: &str)",
            "Tool executor entry",
        );
        engine.add_to_index(
            "tool",
            "src/tools/file.rs",
            1,
            "pub fn read_file(path: &str)",
            "File read tool",
        );
        engine.add_to_index(
            "search",
            "src/search.rs",
            10,
            "pub fn search_files(query: &str)",
            "File search function",
        );
        engine.add_to_index(
            "test",
            "src/agent.rs",
            200,
            "#[test] fn test_agent()",
            "Agent tests",
        );
        engine.add_to_index(
            "test",
            "src/provider.rs",
            100,
            "#[test] fn test_provider()",
            "Provider tests",
        );
        engine
    }

    #[test]
    fn test_basic_search_single_keyword() {
        let mut engine = seeded_engine();
        let answer = engine.ask("agent");
        assert!(!answer.evidence.is_empty());
        assert!(answer.summary.contains("agent"));
    }

    #[test]
    fn test_basic_search_returns_relevance() {
        let mut engine = seeded_engine();
        let answer = engine.ask("agent");
        for r in &answer.evidence {
            assert!(r.relevance_score > 0.0);
        }
    }

    #[test]
    fn test_basic_search_no_results() {
        let mut engine = seeded_engine();
        let answer = engine.ask("nonexistent_xyz");
        assert!(answer.evidence.is_empty());
        assert!(answer.summary.contains("No relevant results"));
        assert_eq!(answer.confidence, 0.0);
    }

    #[test]
    fn test_multi_keyword_search() {
        let mut engine = seeded_engine();
        let answer = engine.ask("agent test");
        // Should find results for both "agent" and "test"
        assert!(!answer.evidence.is_empty());
    }

    #[test]
    fn test_follow_up_maintains_context() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        assert!(engine.conversation.context_window.topic.is_some());

        let answer = engine.follow_up("test");
        // Context should still reference agent-related files.
        assert!(engine
            .conversation
            .context_window
            .relevant_files
            .contains(&"src/agent.rs".to_string()));
        assert!(!answer.evidence.is_empty());
    }

    #[test]
    fn test_follow_up_boosts_context_files() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        let answer = engine.follow_up("test");
        // Results from agent.rs should get a context boost.
        let agent_results: Vec<_> = answer
            .evidence
            .iter()
            .filter(|r| r.file_path == "src/agent.rs")
            .collect();
        assert!(!agent_results.is_empty());
    }

    #[test]
    fn test_follow_up_increments_turn_count() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.follow_up("test");
        assert_eq!(engine.conversation.context_window.turn_count, 2);
    }

    #[test]
    fn test_refine_with_file_type_filter() {
        let mut engine = seeded_engine();
        engine.ask("tool");
        let filters = SearchFilters {
            file_type: Some(".rs".to_string()),
            path: None,
            date_range: None,
        };
        let answer = engine.refine(filters);
        for r in &answer.evidence {
            assert!(r.file_path.ends_with(".rs"));
        }
    }

    #[test]
    fn test_refine_with_path_filter() {
        let mut engine = seeded_engine();
        engine.ask("tool");
        let filters = SearchFilters {
            file_type: None,
            path: Some("tools/".to_string()),
            date_range: None,
        };
        let answer = engine.refine(filters);
        for r in &answer.evidence {
            assert!(r.file_path.contains("tools/"));
        }
    }

    #[test]
    fn test_refine_narrows_previous_results() {
        let mut engine = seeded_engine();
        engine.ask("test");
        let initial_count = engine.conversation.results.len();
        let filters = SearchFilters {
            file_type: None,
            path: Some("agent".to_string()),
            date_range: None,
        };
        let answer = engine.refine(filters);
        assert!(answer.evidence.len() <= initial_count);
    }

    #[test]
    fn test_answer_synthesis_summary() {
        let synth = AnswerSynthesizer::new();
        let results = vec![SearchResult {
            file_path: "src/main.rs".to_string(),
            line_range: (1, 10),
            snippet: "fn main() {}".to_string(),
            relevance_score: 0.9,
            explanation: "Entry point".to_string(),
        }];
        let ctx = SearchContext::default();
        let answer = synth.synthesize("entry point", &results, &ctx);
        assert!(answer.summary.contains("entry point"));
        assert!(answer.confidence > 0.0);
    }

    #[test]
    fn test_answer_synthesis_confidence_empty() {
        let conf = AnswerSynthesizer::compute_confidence(&[]);
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn test_answer_synthesis_confidence_high() {
        let results = vec![
            SearchResult {
                file_path: "a.rs".to_string(),
                line_range: (1, 2),
                snippet: "x".to_string(),
                relevance_score: 0.95,
                explanation: String::new(),
            },
            SearchResult {
                file_path: "b.rs".to_string(),
                line_range: (1, 2),
                snippet: "y".to_string(),
                relevance_score: 0.85,
                explanation: String::new(),
            },
        ];
        let conf = AnswerSynthesizer::compute_confidence(&results);
        assert!(conf > 0.8);
    }

    #[test]
    fn test_follow_up_suggestions_generated() {
        let mut engine = seeded_engine();
        let answer = engine.ask("agent");
        assert!(!answer.follow_ups.is_empty());
    }

    #[test]
    fn test_suggest_questions_returns_cached() {
        let mut engine = seeded_engine();
        engine.ask("provider");
        let suggestions = engine.suggest_questions();
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_history_tracking() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.ask("provider");
        let hist = engine.history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].0.text, "agent");
        assert_eq!(hist[1].0.text, "provider");
    }

    #[test]
    fn test_history_records_query_type() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.follow_up("test");
        let hist = engine.history();
        assert_eq!(hist[0].0.query_type, QueryType::Natural);
        assert_eq!(hist[1].0.query_type, QueryType::FollowUp);
    }

    #[test]
    fn test_clear_context_resets_state() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.follow_up("test");
        engine.clear_context();

        assert!(engine.conversation.context_window.topic.is_none());
        assert!(engine.conversation.context_window.relevant_files.is_empty());
        assert_eq!(engine.conversation.context_window.turn_count, 0);
        assert!(engine.conversation.query_history.is_empty());
        assert!(engine.conversation.results.is_empty());
    }

    #[test]
    fn test_clear_context_preserves_history() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.clear_context();
        assert!(!engine.history().is_empty());
    }

    #[test]
    fn test_conversation_id_generated() {
        let engine = ConversationalSearchEngine::new();
        assert!(engine.conversation.id.starts_with("conv-"));
    }

    #[test]
    fn test_search_result_sorted_by_relevance() {
        let mut engine = seeded_engine();
        let answer = engine.ask("agent test");
        let scores: Vec<f64> = answer.evidence.iter().map(|r| r.relevance_score).collect();
        for window in scores.windows(2) {
            assert!(window[0] >= window[1]);
        }
    }

    #[test]
    fn test_extract_facts_filters_low_relevance() {
        let results = vec![
            SearchResult {
                file_path: "a.rs".to_string(),
                line_range: (1, 2),
                snippet: "x".to_string(),
                relevance_score: 0.1,
                explanation: "low".to_string(),
            },
            SearchResult {
                file_path: "b.rs".to_string(),
                line_range: (1, 2),
                snippet: "y".to_string(),
                relevance_score: 0.8,
                explanation: "high".to_string(),
            },
        ];
        let facts = AnswerSynthesizer::extract_facts(&results);
        assert_eq!(facts.len(), 1);
        assert!(facts[0].contains("high"));
    }

    #[test]
    fn test_search_filters_default() {
        let f = SearchFilters::default();
        assert!(f.file_type.is_none());
        assert!(f.path.is_none());
        assert!(f.date_range.is_none());
    }

    #[test]
    fn test_accumulated_relevant_files() {
        let mut engine = seeded_engine();
        engine.ask("agent");
        engine.ask("provider");
        let files = &engine.conversation.context_window.relevant_files;
        assert!(files.contains(&"src/agent.rs".to_string()));
        assert!(files.contains(&"src/provider.rs".to_string()));
    }
}
