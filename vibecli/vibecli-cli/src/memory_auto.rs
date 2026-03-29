#![allow(dead_code)]
//! Automated memory extraction from agent sessions.
//!
//! After each session completes, MemoryAutoExtractor uses an LLM to extract
//! 3–5 reusable facts and appends them to `~/.vibecli/memory.md` and
//! `.vibecli/project-memory.md` (per-project).
//!
//! Facts are tagged with confidence scores so low-confidence entries can be
//! discarded in the VibeUI Auto-Facts panel.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole};

// ── MemoryFact ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    /// Unique ID (unix-ms hex).
    pub id: String,
    /// The fact text.
    pub fact: String,
    /// Confidence 0.0–1.0 as estimated by the LLM.
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// Topic tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether the user has pinned this fact (pinned = never auto-purge).
    #[serde(default)]
    pub pinned: bool,
    /// Source session ID.
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_confidence() -> f32 { 0.7 }

impl MemoryFact {
    pub fn new(fact: impl Into<String>, confidence: f32, tags: Vec<String>) -> Self {
        let id = {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("{:x}", ms)
        };
        Self { id, fact: fact.into(), confidence, tags, pinned: false, session_id: None }
    }
}

// ── MemoryAutoExtractor ───────────────────────────────────────────────────────

/// Extracts structured facts from a completed session using an LLM.
pub struct MemoryAutoExtractor {
    pub llm: Arc<dyn LLMProvider>,
}

impl MemoryAutoExtractor {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self { llm }
    }

    /// Extract 3–5 reusable facts from a conversation.
    pub async fn extract(&self, messages: &[Message], session_id: Option<&str>) -> Vec<MemoryFact> {
        let conversation: String = messages.iter()
            .filter(|m| matches!(m.role, MessageRole::User | MessageRole::Assistant))
            .take(40) // cap to avoid token overload
            .map(|m| {
                let end = m.content.char_indices().nth(500).map(|(i,_)| i).unwrap_or(m.content.len());
                format!("[{:?}]: {}", m.role, &m.content[..end])
            })
            .collect::<Vec<_>>()
            .join("\n");

        if conversation.trim().is_empty() {
            return vec![];
        }

        let prompt = format!(
            r#"Analyze this coding session and extract 3-5 reusable facts worth remembering.

RULES:
- Only extract stable, project-specific facts (patterns, conventions, file locations, tools used).
- Do NOT extract task-specific details that won't apply to future sessions.
- Confidence: 0.9=certain, 0.7=likely, 0.5=uncertain.
- Tags: use short lowercase words (e.g. ["rust", "testing", "database"]).

Return ONLY valid JSON array, no markdown, no explanation:
[
  {{"fact": "...", "confidence": 0.8, "tags": ["tag1", "tag2"]}},
  ...
]

Session:
{}
"#,
            conversation
        );

        let extract_msgs = vec![
            Message { role: MessageRole::User, content: prompt },
        ];

        match self.llm.chat(&extract_msgs, None).await {
            Ok(response) => {
                // Extract JSON array from response (strip any surrounding text)
                let json_start = match response.find('[') {
                    Some(i) => i,
                    None => return vec![],
                };
                let json_end = match response.rfind(']') {
                    Some(i) => i + 1,
                    None => return vec![],
                };
                if json_start >= json_end {
                    return vec![];
                }
                let json_str = &response[json_start..json_end];

                #[derive(Deserialize)]
                struct RawFact {
                    fact: String,
                    #[serde(default = "default_confidence")]
                    confidence: f32,
                    #[serde(default)]
                    tags: Vec<String>,
                }

                match serde_json::from_str::<Vec<RawFact>>(json_str) {
                    Ok(raw_facts) => raw_facts.into_iter()
                        .filter(|f| f.confidence >= 0.5)
                        .map(|f| {
                            let mut fact = MemoryFact::new(f.fact, f.confidence, f.tags);
                            fact.session_id = session_id.map(|s| s.to_string());
                            fact
                        })
                        .collect(),
                    Err(_) => vec![],
                }
            }
            Err(_) => vec![],
        }
    }

    /// Deduplicate new facts against existing ones (simple text similarity).
    pub fn deduplicate(existing: &[MemoryFact], new: &[MemoryFact]) -> Vec<MemoryFact> {
        new.iter()
            .filter(|new_fact| {
                let new_lower = new_fact.fact.to_lowercase();
                !existing.iter().any(|ex| {
                    let ex_lower = ex.fact.to_lowercase();
                    // Simple overlap check: if 60%+ of words match, consider duplicate
                    let new_words: std::collections::HashSet<&str> = new_lower.split_whitespace().collect();
                    let ex_words: std::collections::HashSet<&str> = ex_lower.split_whitespace().collect();
                    let overlap = new_words.intersection(&ex_words).count();
                    let min_len = new_words.len().min(ex_words.len());
                    min_len > 0 && overlap as f32 / min_len as f32 >= 0.6
                })
            })
            .cloned()
            .collect()
    }
}

// ── AutoMemoryStore ───────────────────────────────────────────────────────────

/// Persists auto-extracted facts to disk.
pub struct AutoMemoryStore {
    path: PathBuf,
}

impl AutoMemoryStore {
    /// Opens the store at `~/.vibecli/auto-memory.json`.
    pub fn global() -> Option<Self> {
        dirs::home_dir().map(|h| Self { path: h.join(".vibecli").join("auto-memory.json") })
    }

    /// Opens a project-scoped store at `.vibecli/auto-memory.json`.
    pub fn for_project(workspace_root: &Path) -> Self {
        Self { path: workspace_root.join(".vibecli").join("auto-memory.json") }
    }

    pub fn load(&self) -> Vec<MemoryFact> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, facts: &[MemoryFact]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(facts)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    /// Append new (deduplicated) facts.
    pub fn append(&self, new_facts: &[MemoryFact]) -> Result<usize> {
        let existing = self.load();
        let deduplicated = MemoryAutoExtractor::deduplicate(&existing, new_facts);
        let count = deduplicated.len();
        if count > 0 {
            let mut all = existing;
            all.extend(deduplicated);
            self.save(&all)?;
        }
        Ok(count)
    }

    /// Delete a fact by ID.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut facts = self.load();
        let before = facts.len();
        facts.retain(|f| f.id != id);
        if facts.len() < before {
            self.save(&facts)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Pin/unpin a fact.
    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<bool> {
        let mut facts = self.load();
        if let Some(f) = facts.iter_mut().find(|f| f.id == id) {
            f.pinned = pinned;
            self.save(&facts)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Also append facts as markdown lines to a `.md` file for easy reading.
    pub fn append_to_markdown(&self, facts: &[MemoryFact], md_path: &Path) -> Result<()> {
        if facts.is_empty() { return Ok(()); }
        let mut lines = vec![String::new(), "<!-- auto-extracted memories -->".to_string()];
        for f in facts {
            let tags = if f.tags.is_empty() {
                String::new()
            } else {
                format!(" `[{}]`", f.tags.join(", "))
            };
            lines.push(format!("- {}{} *(confidence: {:.0}%)*", f.fact, tags, f.confidence * 100.0));
        }
        lines.push(String::new());
        let existing = std::fs::read_to_string(md_path).unwrap_or_default();
        let combined = existing + &lines.join("\n");
        if let Some(parent) = md_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(md_path, combined)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn deduplication_filters_near_duplicates() {
        let existing = vec![
            MemoryFact::new("Use cargo check before cargo build for faster feedback", 0.9, vec!["rust".to_string()]),
        ];
        let new_facts = vec![
            MemoryFact::new("cargo check before cargo build for faster feedback loop", 0.8, vec!["rust".to_string()]),
            MemoryFact::new("Always run tests with cargo test --workspace", 0.9, vec!["rust".to_string()]),
        ];
        let result = MemoryAutoExtractor::deduplicate(&existing, &new_facts);
        // First fact is near-duplicate, second is unique
        assert_eq!(result.len(), 1);
        assert!(result[0].fact.contains("cargo test"));
    }

    #[test]
    fn auto_memory_store_append_and_delete() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());

        let facts = vec![
            MemoryFact::new("Use bun instead of npm", 0.9, vec!["js".to_string()]),
        ];
        let added = store.append(&facts).unwrap();
        assert_eq!(added, 1);

        let loaded = store.load();
        assert_eq!(loaded.len(), 1);

        // Append duplicate — should not grow
        let added2 = store.append(&facts).unwrap();
        assert_eq!(added2, 0);
        assert_eq!(store.load().len(), 1);

        let deleted = store.delete(&loaded[0].id).unwrap();
        assert!(deleted);
        assert!(store.load().is_empty());
    }

    #[test]
    fn set_pinned() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        let fact = MemoryFact::new("Important project convention", 0.9, vec![]);
        store.append(&[fact]).unwrap();
        let loaded = store.load();
        let id = &loaded[0].id.clone();
        store.set_pinned(id, true).unwrap();
        assert!(store.load()[0].pinned);
    }

    // ── MemoryFact construction ──

    #[test]
    fn memory_fact_new_defaults() {
        let fact = MemoryFact::new("Use cargo check", 0.85, vec!["rust".to_string()]);
        assert_eq!(fact.fact, "Use cargo check");
        assert!((fact.confidence - 0.85).abs() < 0.001);
        assert_eq!(fact.tags, vec!["rust"]);
        assert!(!fact.pinned);
        assert!(fact.session_id.is_none());
        assert!(!fact.id.is_empty());
    }

    // ── MemoryFact serde roundtrip ──

    #[test]
    fn memory_fact_serde_roundtrip() {
        let mut fact = MemoryFact::new("Test fact", 0.9, vec!["test".to_string()]);
        fact.pinned = true;
        fact.session_id = Some("session-123".to_string());
        let json = serde_json::to_string(&fact).unwrap();
        let back: MemoryFact = serde_json::from_str(&json).unwrap();
        assert_eq!(back.fact, "Test fact");
        assert!(back.pinned);
        assert_eq!(back.session_id.as_deref(), Some("session-123"));
        assert_eq!(back.tags, vec!["test"]);
    }

    // ── deduplication with no existing facts ──

    #[test]
    fn deduplication_with_no_existing_keeps_all() {
        let new_facts = vec![
            MemoryFact::new("Fact one", 0.9, vec![]),
            MemoryFact::new("Fact two", 0.8, vec![]),
        ];
        let result = MemoryAutoExtractor::deduplicate(&[], &new_facts);
        assert_eq!(result.len(), 2);
    }

    // ── deduplication with completely different facts ──

    #[test]
    fn deduplication_keeps_non_duplicates() {
        let existing = vec![
            MemoryFact::new("The project uses React with TypeScript", 0.9, vec![]),
        ];
        let new_facts = vec![
            MemoryFact::new("Database migrations are in migrations/ directory", 0.8, vec![]),
        ];
        let result = MemoryAutoExtractor::deduplicate(&existing, &new_facts);
        assert_eq!(result.len(), 1);
    }

    // ── delete nonexistent fact returns false ──

    #[test]
    fn delete_nonexistent_fact_returns_false() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        let fact = MemoryFact::new("Some fact", 0.9, vec![]);
        store.append(&[fact]).unwrap();
        let deleted = store.delete("nonexistent-id").unwrap();
        assert!(!deleted);
        assert_eq!(store.load().len(), 1);
    }

    // ── set_pinned nonexistent returns false ──

    #[test]
    fn set_pinned_nonexistent_returns_false() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        let result = store.set_pinned("no-such-id", true).unwrap();
        assert!(!result);
    }

    // ── empty store loads empty ──

    #[test]
    fn empty_store_loads_empty() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        assert!(store.load().is_empty());
    }

    // ── append_to_markdown ──

    #[test]
    fn append_to_markdown_creates_file() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        let md_path = tmp.path().join("memory.md");
        let facts = vec![
            MemoryFact::new("Uses bun for JS", 0.9, vec!["js".to_string()]),
        ];
        store.append_to_markdown(&facts, &md_path).unwrap();
        let content = std::fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("Uses bun for JS"));
        assert!(content.contains("90%"));
    }

    // ── append_to_markdown with empty facts does nothing ──

    #[test]
    fn append_to_markdown_empty_facts_noop() {
        let tmp = TempDir::new().unwrap();
        let store = AutoMemoryStore::for_project(tmp.path());
        let md_path = tmp.path().join("memory.md");
        store.append_to_markdown(&[], &md_path).unwrap();
        // File should not exist since we wrote nothing
        assert!(!md_path.exists());
    }
}
