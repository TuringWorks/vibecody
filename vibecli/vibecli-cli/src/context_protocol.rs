#![allow(dead_code)]
//! Unified Context Protocol — standardized context sharing between AI coding tools.
//!
//! Like LSP standardized language features, this standardizes context sharing
//! across AI coding assistants, enabling interoperability and composability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextType {
    FileContent,
    GitDiff,
    GitHistory,
    TerminalOutput,
    ErrorLog,
    TestResults,
    DependencyGraph,
    ApiDocs,
    SearchResults,
    CustomContext(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContextPriority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Low = 1,
    Background = 0,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextScope {
    File,
    Directory,
    Workspace,
    Project,
    Organization,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShareMode {
    ReadOnly,
    ReadWrite,
    Snapshot,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    LRU,
    Priority,
    FIFO,
    SmallestFirst,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub context_type: ContextType,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub priority: ContextPriority,
    pub scope: ContextScope,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub token_count: usize,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

impl ContextItem {
    /// Convenience constructor for a file-content context item.
    pub fn file(path: &str, content: &str) -> Self {
        let token_count = ContextSerializer::estimate_tokens(content);
        Self {
            id: path.to_string(),
            context_type: ContextType::FileContent,
            content: content.to_string(),
            metadata: HashMap::new(),
            priority: ContextPriority::Medium,
            scope: ContextScope::File,
            file_path: Some(path.to_string()),
            line_range: None,
            token_count,
            created_at: 0,
            expires_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub items: Vec<ContextItem>,
    pub max_tokens: usize,
    pub used_tokens: usize,
    pub eviction_policy: EvictionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    pub total_tokens: usize,
    pub system_tokens: usize,
    pub user_tokens: usize,
    pub reserved_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextShare {
    pub id: String,
    pub items: Vec<String>,
    pub mode: ShareMode,
    pub shared_with: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextMetrics {
    pub total_items: u64,
    pub total_tokens: u64,
    pub evictions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_item_size: f64,
}

// ---------------------------------------------------------------------------
// ContextManager
// ---------------------------------------------------------------------------

pub struct ContextManager {
    pub window: ContextWindow,
    pub budget: ContextBudget,
    pub shares: HashMap<String, ContextShare>,
    pub metrics: ContextMetrics,
    /// Maps item id -> index in window.items
    pub index: HashMap<String, usize>,
    /// Monotonic counter used for LRU / FIFO ordering
    access_counter: u64,
    /// Tracks last access per item id (for LRU)
    access_order: HashMap<String, u64>,
    /// Tracks insertion order per item id (for FIFO)
    insert_order: HashMap<String, u64>,
    insert_counter: u64,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            window: ContextWindow {
                items: Vec::new(),
                max_tokens,
                used_tokens: 0,
                eviction_policy: EvictionPolicy::LRU,
            },
            budget: ContextBudget {
                total_tokens: max_tokens,
                system_tokens: 0,
                user_tokens: 0,
                reserved_tokens: 0,
            },
            shares: HashMap::new(),
            metrics: ContextMetrics::default(),
            index: HashMap::new(),
            access_counter: 0,
            access_order: HashMap::new(),
            insert_order: HashMap::new(),
            insert_counter: 0,
        }
    }

    /// Add an item, evicting if necessary. Returns error if a single item exceeds max_tokens.
    pub fn add(&mut self, item: ContextItem) -> Result<(), String> {
        if item.token_count > self.window.max_tokens {
            return Err(format!(
                "Item '{}' has {} tokens which exceeds max_tokens {}",
                item.id, item.token_count, self.window.max_tokens
            ));
        }

        // Remove existing item with same id if present
        if self.index.contains_key(&item.id) {
            self.remove(&item.id.clone())?;
        }

        // Evict until we have room
        while self.window.used_tokens + item.token_count > self.window.max_tokens {
            let evicted = self.evict_one();
            if !evicted {
                return Err("Cannot make room for item".to_string());
            }
        }

        let id = item.id.clone();
        let tokens = item.token_count;
        let idx = self.window.items.len();
        self.window.items.push(item);
        self.window.used_tokens += tokens;
        self.index.insert(id.clone(), idx);

        self.access_counter += 1;
        self.access_order.insert(id.clone(), self.access_counter);
        self.insert_counter += 1;
        self.insert_order.insert(id, self.insert_counter);

        self.update_metrics();
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if let Some(&idx) = self.index.get(id) {
            let tokens = self.window.items[idx].token_count;
            self.window.items.remove(idx);
            self.window.used_tokens = self.window.used_tokens.saturating_sub(tokens);
            self.index.remove(id);
            self.access_order.remove(id);
            self.insert_order.remove(id);
            // Rebuild index after removal
            self.rebuild_index();
            self.update_metrics();
            Ok(())
        } else {
            Err(format!("Item '{}' not found", id))
        }
    }

    pub fn get(&mut self, id: &str) -> Option<&ContextItem> {
        if let Some(&idx) = self.index.get(id) {
            self.access_counter += 1;
            self.access_order.insert(id.to_string(), self.access_counter);
            self.metrics.cache_hits += 1;
            Some(&self.window.items[idx])
        } else {
            self.metrics.cache_misses += 1;
            None
        }
    }

    pub fn query(&self, context_type: &ContextType) -> Vec<&ContextItem> {
        self.window
            .items
            .iter()
            .filter(|item| &item.context_type == context_type)
            .collect()
    }

    pub fn query_by_file(&self, path: &str) -> Vec<&ContextItem> {
        self.window
            .items
            .iter()
            .filter(|item| item.file_path.as_deref() == Some(path))
            .collect()
    }

    pub fn prioritize(&mut self, id: &str, priority: ContextPriority) -> Result<(), String> {
        if let Some(&idx) = self.index.get(id) {
            self.window.items[idx].priority = priority;
            Ok(())
        } else {
            Err(format!("Item '{}' not found", id))
        }
    }

    /// Evict items based on policy until within budget. Returns number of items evicted.
    pub fn evict_to_budget(&mut self) -> usize {
        let mut evicted = 0;
        while self.window.used_tokens > self.window.max_tokens && !self.window.items.is_empty() {
            if self.evict_one() {
                evicted += 1;
            } else {
                break;
            }
        }
        evicted
    }

    pub fn token_usage(&self) -> (usize, usize) {
        (self.window.used_tokens, self.window.max_tokens)
    }

    pub fn share(
        &mut self,
        item_ids: Vec<String>,
        mode: ShareMode,
        targets: Vec<String>,
    ) -> Result<String, String> {
        // Validate all items exist
        for id in &item_ids {
            if !self.index.contains_key(id) {
                return Err(format!("Item '{}' not found", id));
            }
        }

        let share_id = format!("share-{}", self.shares.len() + 1);
        let share = ContextShare {
            id: share_id.clone(),
            items: item_ids,
            mode,
            shared_with: targets,
            created_at: 0,
        };
        self.shares.insert(share_id.clone(), share);
        Ok(share_id)
    }

    pub fn list_shares(&self) -> Vec<&ContextShare> {
        self.shares.values().collect()
    }

    /// Select highest-priority items that fit in the given token budget.
    pub fn build_prompt(&self, budget: usize) -> Vec<&ContextItem> {
        let mut sorted: Vec<&ContextItem> = self.window.items.iter().collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut result = Vec::new();
        let mut used = 0;
        for item in sorted {
            if used + item.token_count <= budget {
                result.push(item);
                used += item.token_count;
            }
        }
        result
    }

    pub fn get_metrics(&self) -> &ContextMetrics {
        &self.metrics
    }

    pub fn clear(&mut self) {
        self.window.items.clear();
        self.window.used_tokens = 0;
        self.index.clear();
        self.access_order.clear();
        self.insert_order.clear();
        self.shares.clear();
        self.metrics = ContextMetrics::default();
    }

    pub fn count(&self) -> usize {
        self.window.items.len()
    }

    // --- internal helpers ---

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, item) in self.window.items.iter().enumerate() {
            self.index.insert(item.id.clone(), i);
        }
    }

    fn update_metrics(&mut self) {
        let count = self.window.items.len() as u64;
        self.metrics.total_items = count;
        self.metrics.total_tokens = self.window.used_tokens as u64;
        self.metrics.avg_item_size = if count > 0 {
            self.window.used_tokens as f64 / count as f64
        } else {
            0.0
        };
    }

    /// Evict a single item according to the eviction policy. Returns true if evicted.
    fn evict_one(&mut self) -> bool {
        if self.window.items.is_empty() {
            return false;
        }

        let victim_id = match self.window.eviction_policy {
            EvictionPolicy::LRU => {
                // Find item with lowest (oldest) access counter
                self.window
                    .items
                    .iter()
                    .min_by_key(|item| self.access_order.get(&item.id).copied().unwrap_or(0))
                    .map(|item| item.id.clone())
            }
            EvictionPolicy::Priority => {
                // Evict lowest priority first, then by oldest access
                self.window
                    .items
                    .iter()
                    .min_by(|a, b| {
                        a.priority.cmp(&b.priority).then_with(|| {
                            let a_access =
                                self.access_order.get(&a.id).copied().unwrap_or(0);
                            let b_access =
                                self.access_order.get(&b.id).copied().unwrap_or(0);
                            a_access.cmp(&b_access)
                        })
                    })
                    .map(|item| item.id.clone())
            }
            EvictionPolicy::FIFO => {
                // Evict the item inserted first
                self.window
                    .items
                    .iter()
                    .min_by_key(|item| self.insert_order.get(&item.id).copied().unwrap_or(0))
                    .map(|item| item.id.clone())
            }
            EvictionPolicy::SmallestFirst => {
                self.window
                    .items
                    .iter()
                    .min_by_key(|item| item.token_count)
                    .map(|item| item.id.clone())
            }
        };

        if let Some(id) = victim_id {
            let _ = self.remove(&id);
            self.metrics.evictions += 1;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// ContextSerializer
// ---------------------------------------------------------------------------

pub struct ContextSerializer;

impl ContextSerializer {
    pub fn to_json(items: &[&ContextItem]) -> String {
        // We serialize a Vec of owned clones
        let owned: Vec<ContextItem> = items.iter().map(|i| (*i).clone()).collect();
        serde_json::to_string_pretty(&owned).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn from_json(json: &str) -> Result<Vec<ContextItem>, String> {
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))
    }

    /// Compressed format for token efficiency: one line per item, pipe-delimited.
    pub fn to_compact(items: &[&ContextItem]) -> String {
        items
            .iter()
            .map(|item| {
                format!(
                    "{}|{:?}|{:?}|{}|{}",
                    item.id,
                    item.context_type,
                    item.priority,
                    item.token_count,
                    item.content.replace('\n', "\\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Approximate token count: words * 1.3
    pub fn estimate_tokens(text: &str) -> usize {
        let word_count = text.split_whitespace().count();
        (word_count as f64 * 1.3).ceil() as usize
    }
}

// ---------------------------------------------------------------------------
// ContextCache
// ---------------------------------------------------------------------------

pub struct ContextCache {
    capacity: usize,
    store: HashMap<String, ContextItem>,
    access_order: Vec<String>,
    hits: u64,
    misses: u64,
}

impl ContextCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            store: HashMap::new(),
            access_order: Vec::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&ContextItem> {
        if self.store.contains_key(key) {
            self.hits += 1;
            // Move to end (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.to_string());
            self.store.get(key)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn put(&mut self, key: &str, item: ContextItem) {
        if self.store.contains_key(key) {
            self.access_order.retain(|k| k != key);
        } else if self.store.len() >= self.capacity {
            // Evict LRU
            if let Some(lru_key) = self.access_order.first().cloned() {
                self.store.remove(&lru_key);
                self.access_order.remove(0);
            }
        }
        self.store.insert(key.to_string(), item);
        self.access_order.push(key.to_string());
    }

    pub fn invalidate(&mut self, key: &str) {
        self.store.remove(key);
        self.access_order.retain(|k| k != key);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, tokens: usize, priority: ContextPriority) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            context_type: ContextType::FileContent,
            content: "x".repeat(tokens),
            metadata: HashMap::new(),
            priority,
            scope: ContextScope::File,
            file_path: None,
            line_range: None,
            token_count: tokens,
            created_at: 0,
            expires_at: None,
        }
    }

    fn make_item_with_file(id: &str, tokens: usize, path: &str) -> ContextItem {
        let mut item = make_item(id, tokens, ContextPriority::Medium);
        item.file_path = Some(path.to_string());
        item
    }

    fn make_item_typed(id: &str, tokens: usize, ct: ContextType) -> ContextItem {
        let mut item = make_item(id, tokens, ContextPriority::Medium);
        item.context_type = ct;
        item
    }

    // --- Basic add / remove / get ---

    #[test]
    fn test_add_item() {
        let mut mgr = ContextManager::new(1000);
        let item = make_item("a", 100, ContextPriority::High);
        assert!(mgr.add(item).is_ok());
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_add_exceeds_max() {
        let mut mgr = ContextManager::new(50);
        let item = make_item("a", 100, ContextPriority::High);
        assert!(mgr.add(item).is_err());
    }

    #[test]
    fn test_remove_item() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        assert!(mgr.remove("a").is_ok());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut mgr = ContextManager::new(1000);
        assert!(mgr.remove("nope").is_err());
    }

    #[test]
    fn test_get_item() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        assert!(mgr.get("a").is_some());
        assert_eq!(mgr.get("a").unwrap().id, "a");
    }

    #[test]
    fn test_get_missing() {
        let mut mgr = ContextManager::new(1000);
        assert!(mgr.get("nope").is_none());
    }

    #[test]
    fn test_add_duplicate_replaces() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::Low)).unwrap();
        mgr.add(make_item("a", 200, ContextPriority::High)).unwrap();
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.get("a").unwrap().token_count, 200);
    }

    // --- Query ---

    #[test]
    fn test_query_by_type() {
        let mut mgr = ContextManager::new(5000);
        mgr.add(make_item_typed("a", 10, ContextType::FileContent)).unwrap();
        mgr.add(make_item_typed("b", 10, ContextType::GitDiff)).unwrap();
        mgr.add(make_item_typed("c", 10, ContextType::FileContent)).unwrap();
        let results = mgr.query(&ContextType::FileContent);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_type_empty() {
        let mgr = ContextManager::new(5000);
        assert!(mgr.query(&ContextType::ErrorLog).is_empty());
    }

    #[test]
    fn test_query_by_file() {
        let mut mgr = ContextManager::new(5000);
        mgr.add(make_item_with_file("a", 10, "src/main.rs")).unwrap();
        mgr.add(make_item_with_file("b", 10, "src/lib.rs")).unwrap();
        mgr.add(make_item_with_file("c", 10, "src/main.rs")).unwrap();
        let results = mgr.query_by_file("src/main.rs");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_file_none() {
        let mut mgr = ContextManager::new(5000);
        mgr.add(make_item("a", 10, ContextPriority::Low)).unwrap();
        assert!(mgr.query_by_file("nope.rs").is_empty());
    }

    // --- Prioritize ---

    #[test]
    fn test_prioritize() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::Low)).unwrap();
        mgr.prioritize("a", ContextPriority::Critical).unwrap();
        assert_eq!(mgr.get("a").unwrap().priority, ContextPriority::Critical);
    }

    #[test]
    fn test_prioritize_missing() {
        let mut mgr = ContextManager::new(1000);
        assert!(mgr.prioritize("nope", ContextPriority::High).is_err());
    }

    // --- Eviction: LRU ---

    #[test]
    fn test_eviction_lru() {
        let mut mgr = ContextManager::new(200);
        mgr.window.eviction_policy = EvictionPolicy::LRU;
        mgr.add(make_item("a", 100, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("b", 100, ContextPriority::Medium)).unwrap();
        // Access "a" so "b" is LRU
        let _ = mgr.get("a");
        // Adding "c" should evict "b"
        mgr.add(make_item("c", 100, ContextPriority::Medium)).unwrap();
        assert!(mgr.get("a").is_some());
        assert_eq!(mgr.count(), 2);
        // "b" was evicted
        assert!(mgr.index.get("b").is_none());
    }

    #[test]
    fn test_eviction_lru_oldest_evicted() {
        let mut mgr = ContextManager::new(150);
        mgr.window.eviction_policy = EvictionPolicy::LRU;
        mgr.add(make_item("a", 50, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("b", 50, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("c", 50, ContextPriority::Medium)).unwrap();
        // Now full at 150. Access b and c, leaving a as LRU.
        let _ = mgr.get("b");
        let _ = mgr.get("c");
        mgr.add(make_item("d", 50, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("a").is_none()); // a evicted
        assert!(mgr.get("b").is_some());
        assert!(mgr.get("c").is_some());
        assert!(mgr.get("d").is_some());
    }

    // --- Eviction: Priority ---

    #[test]
    fn test_eviction_priority() {
        let mut mgr = ContextManager::new(200);
        mgr.window.eviction_policy = EvictionPolicy::Priority;
        mgr.add(make_item("high", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("low", 100, ContextPriority::Low)).unwrap();
        // Adding another should evict "low"
        mgr.add(make_item("med", 100, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("low").is_none());
        assert!(mgr.get("high").is_some());
    }

    #[test]
    fn test_eviction_priority_keeps_critical() {
        let mut mgr = ContextManager::new(200);
        mgr.window.eviction_policy = EvictionPolicy::Priority;
        mgr.add(make_item("crit", 100, ContextPriority::Critical)).unwrap();
        mgr.add(make_item("bg", 100, ContextPriority::Background)).unwrap();
        mgr.add(make_item("new", 100, ContextPriority::High)).unwrap();
        assert!(mgr.index.get("bg").is_none());
        assert!(mgr.get("crit").is_some());
    }

    // --- Eviction: FIFO ---

    #[test]
    fn test_eviction_fifo() {
        let mut mgr = ContextManager::new(200);
        mgr.window.eviction_policy = EvictionPolicy::FIFO;
        mgr.add(make_item("first", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("second", 100, ContextPriority::Low)).unwrap();
        // Even though "first" has higher priority, FIFO evicts it first
        mgr.add(make_item("third", 100, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("first").is_none());
        assert!(mgr.get("second").is_some());
    }

    #[test]
    fn test_eviction_fifo_order() {
        let mut mgr = ContextManager::new(150);
        mgr.window.eviction_policy = EvictionPolicy::FIFO;
        mgr.add(make_item("a", 50, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("b", 50, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("c", 50, ContextPriority::Medium)).unwrap();
        // Access "a" to prove FIFO ignores access order
        let _ = mgr.get("a");
        mgr.add(make_item("d", 50, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("a").is_none()); // a evicted despite recent access
    }

    // --- Eviction: SmallestFirst ---

    #[test]
    fn test_eviction_smallest_first() {
        let mut mgr = ContextManager::new(200);
        mgr.window.eviction_policy = EvictionPolicy::SmallestFirst;
        mgr.add(make_item("big", 120, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("small", 80, ContextPriority::Medium)).unwrap();
        // Full at 200. Adding 90 needs 90 free -> evict small(80), then still short,
        // so this tests that smallest is evicted first.
        // Use a value that fits after one eviction: need 120+new <= 200 => new <= 80
        mgr.add(make_item("new", 80, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("small").is_none()); // smallest evicted
        assert!(mgr.get("big").is_some());
        assert!(mgr.get("new").is_some());
    }

    #[test]
    fn test_eviction_smallest_first_multiple() {
        let mut mgr = ContextManager::new(100);
        mgr.window.eviction_policy = EvictionPolicy::SmallestFirst;
        mgr.add(make_item("a", 30, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("b", 20, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("c", 50, ContextPriority::Medium)).unwrap();
        // Full at 100. Need 40 free -> evict b(20) then a(30) = 50 freed
        mgr.add(make_item("d", 40, ContextPriority::Medium)).unwrap();
        assert!(mgr.index.get("b").is_none());
        assert!(mgr.index.get("a").is_none());
        assert!(mgr.get("c").is_some());
        assert!(mgr.get("d").is_some());
    }

    // --- evict_to_budget ---

    #[test]
    fn test_evict_to_budget() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 400, ContextPriority::Low)).unwrap();
        mgr.add(make_item("b", 400, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("c", 200, ContextPriority::High)).unwrap();
        // Shrink budget
        mgr.window.max_tokens = 500;
        let evicted = mgr.evict_to_budget();
        assert!(evicted > 0);
        assert!(mgr.window.used_tokens <= 500);
    }

    #[test]
    fn test_evict_to_budget_already_within() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::Low)).unwrap();
        let evicted = mgr.evict_to_budget();
        assert_eq!(evicted, 0);
    }

    // --- Token usage ---

    #[test]
    fn test_token_usage() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 200, ContextPriority::Low)).unwrap();
        let (used, max) = mgr.token_usage();
        assert_eq!(used, 300);
        assert_eq!(max, 1000);
    }

    // --- Sharing ---

    #[test]
    fn test_share_items() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 100, ContextPriority::Low)).unwrap();
        let share_id = mgr
            .share(
                vec!["a".into(), "b".into()],
                ShareMode::ReadOnly,
                vec!["agent-2".into()],
            )
            .unwrap();
        assert!(!share_id.is_empty());
        assert_eq!(mgr.list_shares().len(), 1);
    }

    #[test]
    fn test_share_missing_item() {
        let mut mgr = ContextManager::new(1000);
        assert!(mgr
            .share(
                vec!["nope".into()],
                ShareMode::ReadOnly,
                vec!["x".into()]
            )
            .is_err());
    }

    #[test]
    fn test_share_multiple() {
        let mut mgr = ContextManager::new(5000);
        mgr.add(make_item("a", 10, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 10, ContextPriority::High)).unwrap();
        mgr.share(vec!["a".into()], ShareMode::ReadOnly, vec!["x".into()])
            .unwrap();
        mgr.share(vec!["b".into()], ShareMode::Snapshot, vec!["y".into()])
            .unwrap();
        assert_eq!(mgr.list_shares().len(), 2);
    }

    // --- Prompt building ---

    #[test]
    fn test_build_prompt_respects_budget() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 200, ContextPriority::Medium)).unwrap();
        mgr.add(make_item("c", 300, ContextPriority::Low)).unwrap();
        let prompt = mgr.build_prompt(250);
        let total: usize = prompt.iter().map(|i| i.token_count).sum();
        assert!(total <= 250);
    }

    #[test]
    fn test_build_prompt_priority_order() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("low", 100, ContextPriority::Low)).unwrap();
        mgr.add(make_item("crit", 100, ContextPriority::Critical)).unwrap();
        mgr.add(make_item("high", 100, ContextPriority::High)).unwrap();
        let prompt = mgr.build_prompt(200);
        assert_eq!(prompt.len(), 2);
        assert_eq!(prompt[0].id, "crit");
        assert_eq!(prompt[1].id, "high");
    }

    #[test]
    fn test_build_prompt_empty() {
        let mgr = ContextManager::new(1000);
        assert!(mgr.build_prompt(500).is_empty());
    }

    #[test]
    fn test_build_prompt_zero_budget() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        assert!(mgr.build_prompt(0).is_empty());
    }

    // --- Metrics ---

    #[test]
    fn test_metrics_tracking() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 200, ContextPriority::Low)).unwrap();
        let m = mgr.get_metrics();
        assert_eq!(m.total_items, 2);
        assert_eq!(m.total_tokens, 300);
        assert!((m.avg_item_size - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_cache_hits_misses() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        let _ = mgr.get("a"); // hit
        let _ = mgr.get("nope"); // miss
        let _ = mgr.get("a"); // hit
        assert_eq!(mgr.get_metrics().cache_hits, 2);
        assert_eq!(mgr.get_metrics().cache_misses, 1);
    }

    #[test]
    fn test_metrics_evictions() {
        let mut mgr = ContextManager::new(100);
        mgr.add(make_item("a", 60, ContextPriority::Low)).unwrap();
        mgr.add(make_item("b", 60, ContextPriority::High)).unwrap();
        assert!(mgr.get_metrics().evictions > 0);
    }

    // --- Clear / count ---

    #[test]
    fn test_clear() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("a", 100, ContextPriority::High)).unwrap();
        mgr.add(make_item("b", 100, ContextPriority::Low)).unwrap();
        mgr.clear();
        assert_eq!(mgr.count(), 0);
        assert_eq!(mgr.token_usage().0, 0);
    }

    // --- Serialization ---

    #[test]
    fn test_serialize_roundtrip() {
        let item = make_item("a", 100, ContextPriority::High);
        let items = vec![&item];
        let json = ContextSerializer::to_json(&items);
        let parsed = ContextSerializer::from_json(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "a");
        assert_eq!(parsed[0].token_count, 100);
    }

    #[test]
    fn test_serialize_empty() {
        let items: Vec<&ContextItem> = vec![];
        let json = ContextSerializer::to_json(&items);
        let parsed = ContextSerializer::from_json(&json).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_serialize_invalid_json() {
        assert!(ContextSerializer::from_json("not json").is_err());
    }

    #[test]
    fn test_compact_format() {
        let item = make_item("a", 50, ContextPriority::High);
        let items = vec![&item];
        let compact = ContextSerializer::to_compact(&items);
        assert!(compact.contains("a|"));
        assert!(compact.contains("FileContent"));
    }

    #[test]
    fn test_compact_multiple_items() {
        let a = make_item("a", 10, ContextPriority::High);
        let b = make_item("b", 20, ContextPriority::Low);
        let items = vec![&a, &b];
        let compact = ContextSerializer::to_compact(&items);
        let lines: Vec<&str> = compact.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    // --- Token estimation ---

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(ContextSerializer::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_single_word() {
        let est = ContextSerializer::estimate_tokens("hello");
        assert_eq!(est, 2); // ceil(1 * 1.3) = 2
    }

    #[test]
    fn test_estimate_tokens_multiple_words() {
        let est = ContextSerializer::estimate_tokens("hello world foo bar");
        assert_eq!(est, 6); // ceil(4 * 1.3) = 6
    }

    #[test]
    fn test_estimate_tokens_ten_words() {
        let est = ContextSerializer::estimate_tokens("a b c d e f g h i j");
        assert_eq!(est, 13); // ceil(10 * 1.3) = 13
    }

    // --- Cache ---

    #[test]
    fn test_cache_put_get() {
        let mut cache = ContextCache::new(10);
        cache.put("a", make_item("a", 100, ContextPriority::High));
        assert!(cache.get("a").is_some());
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ContextCache::new(10);
        assert!(cache.get("nope").is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ContextCache::new(2);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        cache.put("b", make_item("b", 10, ContextPriority::Low));
        cache.put("c", make_item("c", 10, ContextPriority::Low));
        // "a" should be evicted (LRU)
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn test_cache_lru_access_updates() {
        let mut cache = ContextCache::new(2);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        cache.put("b", make_item("b", 10, ContextPriority::Low));
        // Access "a" to make "b" the LRU
        let _ = cache.get("a");
        cache.put("c", make_item("c", 10, ContextPriority::Low));
        assert!(cache.get("b").is_none()); // b evicted
        assert!(cache.get("a").is_some());
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = ContextCache::new(10);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        cache.invalidate("a");
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn test_cache_hit_rate_zero() {
        let cache = ContextCache::new(10);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = ContextCache::new(10);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        let _ = cache.get("a"); // hit
        let _ = cache.get("b"); // miss
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_hit_rate_all_hits() {
        let mut cache = ContextCache::new(10);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        let _ = cache.get("a");
        let _ = cache.get("a");
        let _ = cache.get("a");
        assert!((cache.hit_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = ContextCache::new(10);
        cache.put("a", make_item("a", 10, ContextPriority::Low));
        cache.put("a", make_item("a", 20, ContextPriority::High));
        let item = cache.get("a").unwrap();
        assert_eq!(item.token_count, 20);
        assert_eq!(item.priority, ContextPriority::High);
    }

    // --- Custom context type ---

    #[test]
    fn test_custom_context_type() {
        let mut mgr = ContextManager::new(1000);
        let mut item = make_item("a", 50, ContextPriority::Medium);
        item.context_type = ContextType::CustomContext("my-tool-output".to_string());
        mgr.add(item).unwrap();
        let results = mgr.query(&ContextType::CustomContext("my-tool-output".to_string()));
        assert_eq!(results.len(), 1);
    }

    // --- Scope and metadata ---

    #[test]
    fn test_item_with_metadata() {
        let mut item = make_item("a", 50, ContextPriority::Medium);
        item.metadata.insert("language".into(), "rust".into());
        item.metadata.insert("version".into(), "1.0".into());
        let mut mgr = ContextManager::new(1000);
        mgr.add(item).unwrap();
        let got = mgr.get("a").unwrap();
        assert_eq!(got.metadata.get("language").unwrap(), "rust");
    }

    #[test]
    fn test_item_with_line_range() {
        let mut item = make_item("a", 50, ContextPriority::Medium);
        item.line_range = Some((10, 20));
        let mut mgr = ContextManager::new(1000);
        mgr.add(item).unwrap();
        assert_eq!(mgr.get("a").unwrap().line_range, Some((10, 20)));
    }

    #[test]
    fn test_item_with_expiry() {
        let mut item = make_item("a", 50, ContextPriority::Medium);
        item.expires_at = Some(9999999);
        let items = vec![&item];
        let json = ContextSerializer::to_json(&items);
        let parsed = ContextSerializer::from_json(&json).unwrap();
        assert_eq!(parsed[0].expires_at, Some(9999999));
    }

    // --- Serialization of all context types ---

    #[test]
    fn test_serialize_all_context_types() {
        let types = vec![
            ContextType::FileContent,
            ContextType::GitDiff,
            ContextType::GitHistory,
            ContextType::TerminalOutput,
            ContextType::ErrorLog,
            ContextType::TestResults,
            ContextType::DependencyGraph,
            ContextType::ApiDocs,
            ContextType::SearchResults,
            ContextType::CustomContext("test".into()),
        ];
        for ct in types {
            let item = make_item_typed("x", 10, ct);
            let items = vec![&item];
            let json = ContextSerializer::to_json(&items);
            let parsed = ContextSerializer::from_json(&json).unwrap();
            assert_eq!(parsed[0].context_type, item.context_type);
        }
    }

    // --- Priority ordering ---

    #[test]
    fn test_priority_ordering() {
        assert!(ContextPriority::Critical > ContextPriority::High);
        assert!(ContextPriority::High > ContextPriority::Medium);
        assert!(ContextPriority::Medium > ContextPriority::Low);
        assert!(ContextPriority::Low > ContextPriority::Background);
    }

    // --- Multiple evictions ---

    #[test]
    fn test_multiple_evictions_to_fit() {
        let mut mgr = ContextManager::new(100);
        mgr.window.eviction_policy = EvictionPolicy::Priority;
        mgr.add(make_item("a", 30, ContextPriority::Low)).unwrap();
        mgr.add(make_item("b", 30, ContextPriority::Background)).unwrap();
        mgr.add(make_item("c", 40, ContextPriority::Medium)).unwrap();
        // Adding 80-token item requires evicting multiple items
        mgr.add(make_item("d", 80, ContextPriority::Critical)).unwrap();
        assert!(mgr.get("d").is_some());
        assert!(mgr.window.used_tokens <= 100);
    }

    // --- Share modes ---

    #[test]
    fn test_share_modes() {
        let mut mgr = ContextManager::new(5000);
        mgr.add(make_item("a", 10, ContextPriority::High)).unwrap();

        let modes = vec![
            ShareMode::ReadOnly,
            ShareMode::ReadWrite,
            ShareMode::Snapshot,
            ShareMode::Stream,
        ];
        for mode in modes {
            let id = mgr
                .share(vec!["a".into()], mode.clone(), vec!["agent".into()])
                .unwrap();
            let share = mgr.shares.get(&id).unwrap();
            assert_eq!(share.mode, mode);
        }
    }

    // --- Build prompt skips items that don't fit ---

    #[test]
    fn test_build_prompt_skips_large() {
        let mut mgr = ContextManager::new(1000);
        mgr.add(make_item("big", 500, ContextPriority::High)).unwrap();
        mgr.add(make_item("small", 50, ContextPriority::Critical)).unwrap();
        let prompt = mgr.build_prompt(100);
        assert_eq!(prompt.len(), 1);
        assert_eq!(prompt[0].id, "small");
    }
}
