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

#[cfg(test)]
mod tests {
    use super::*;

    fn now_secs() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    fn entry(key: &str, value: &str, age_secs: u64, access: u32) -> MemoryEntry {
        let created_at = now_secs().saturating_sub(age_secs);
        let mut e = MemoryEntry::new(key, value, created_at);
        e.access_count = access;
        e
    }

    #[test]
    fn test_consolidate_keeps_fresh() {
        let ad = AutoDream::new(ConsolidationPolicy { max_age_secs: 1_000_000, ..Default::default() });
        let entries = vec![entry("k1", "v1", 100, 0)];
        let result = ad.consolidate(entries);
        assert_eq!(result.kept, 1);
    }

    #[test]
    fn test_consolidate_prunes_old() {
        let ad = AutoDream::new(ConsolidationPolicy { max_age_secs: 50, ..Default::default() });
        // created_at = 1_000_000 - 200 = 999_800; cutoff = now - 50 ≈ current_time - 50
        // This entry is certainly older than 50s from now (now >> 1_000_000)
        let e = MemoryEntry::new("k", "v", 0); // created_at = 0, very old
        let result = ad.consolidate(vec![e]);
        assert_eq!(result.kept, 0);
    }

    #[test]
    fn test_consolidate_deduplicates() {
        let ad = AutoDream::new(ConsolidationPolicy::default());
        let entries = vec![
            entry("key", "v1", 0, 5),
            entry("key", "v2", 0, 10),
        ];
        let result = ad.consolidate(entries);
        assert_eq!(result.kept, 1);
        assert_eq!(result.entries[0].access_count, 10);
    }

    #[test]
    fn test_consolidate_max_entries() {
        let ad = AutoDream::new(ConsolidationPolicy { max_entries: 2, deduplicate_keys: false, ..Default::default() });
        let entries = vec![
            entry("a", "1", 0, 1), entry("b", "2", 0, 5), entry("c", "3", 0, 3),
        ];
        let result = ad.consolidate(entries);
        assert_eq!(result.kept, 2);
    }

    #[test]
    fn test_rank_by_relevance_sorted() {
        let ad = AutoDream::new(ConsolidationPolicy::default());
        let entries = vec![entry("a", "1", 0, 1), entry("b", "2", 0, 10), entry("c", "3", 0, 5)];
        let ranked = ad.rank_by_relevance(entries);
        assert_eq!(ranked[0].access_count, 10);
        assert_eq!(ranked[1].access_count, 5);
    }

    #[test]
    fn test_memory_entry_new() {
        let e = MemoryEntry::new("key", "val", 42);
        assert_eq!(e.key, "key");
        assert_eq!(e.access_count, 0);
    }
}
