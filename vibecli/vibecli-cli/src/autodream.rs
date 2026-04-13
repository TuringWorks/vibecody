//! autodream — Background memory consolidation and relevance ranking.

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub created_at: u64,
    pub access_count: u32,
}

impl MemoryEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>, created_at: u64) -> Self {
        Self { key: key.into(), value: value.into(), created_at, access_count: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct ConsolidationPolicy {
    pub max_age_secs: u64,
    pub max_entries: usize,
    pub deduplicate_keys: bool,
}

impl Default for ConsolidationPolicy {
    fn default() -> Self {
        Self { max_age_secs: 30 * 86_400, max_entries: 500, deduplicate_keys: true }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConsolidationResult {
    pub merged: usize,
    pub pruned: usize,
    pub kept: usize,
    pub entries: Vec<MemoryEntry>,
}

pub struct AutoDream { policy: ConsolidationPolicy }

impl AutoDream {
    pub fn new(policy: ConsolidationPolicy) -> Self { Self { policy } }

    pub fn consolidate(&self, mut entries: Vec<MemoryEntry>) -> ConsolidationResult {
        let now_approx = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
        };
        let cutoff = now_approx.saturating_sub(self.policy.max_age_secs);

        let before = entries.len();
        entries.retain(|e| e.created_at >= cutoff);
        let pruned_age = before - entries.len();

        // Deduplicate: keep highest access_count per key
        let mut pruned_dup = 0;
        if self.policy.deduplicate_keys {
            let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let mut deduped: Vec<MemoryEntry> = Vec::new();
            for e in entries {
                if let Some(&idx) = seen.get(&e.key) {
                    if e.access_count > deduped[idx].access_count {
                        deduped[idx] = e;
                    } else { pruned_dup += 1; }
                } else {
                    seen.insert(e.key.clone(), deduped.len());
                    deduped.push(e);
                }
            }
            entries = deduped;
        }

        // Cap by max_entries (evict lowest access_count)
        if entries.len() > self.policy.max_entries {
            entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
            let pruned_cap = entries.len() - self.policy.max_entries;
            entries.truncate(self.policy.max_entries);
            let kept = entries.len();
            return ConsolidationResult { merged: pruned_dup, pruned: pruned_age + pruned_dup + pruned_cap, kept, entries };
        }

        let kept = entries.len();
        ConsolidationResult { merged: pruned_dup, pruned: pruned_age + pruned_dup, kept, entries }
    }

    pub fn rank_by_relevance(&self, mut entries: Vec<MemoryEntry>) -> Vec<MemoryEntry> {
        entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        entries
    }
}
