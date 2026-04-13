//! prompt_cache — Deterministic prefix caching for LLM prompts.

use std::collections::HashMap;

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for b in s.bytes() { h = h.wrapping_mul(1_099_511_628_211) ^ b as u64; }
    h
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CacheKey(pub u64);

impl CacheKey {
    pub fn from_parts(system: &str, tools: &str, config: &str) -> Self {
        let combined = format!("{}\x00{}\x00{}", system, tools, config);
        Self(fnv1a(&combined))
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub hit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
}

#[derive(Debug, Default)]
pub struct PromptCache {
    store: HashMap<CacheKey, CacheEntry>,
    stats: CacheStats,
}

impl PromptCache {
    pub fn new() -> Self { Self::default() }

    pub fn get_or_insert(&mut self, system: &str, tools: &str, config: &str) -> &CacheEntry {
        let key = CacheKey::from_parts(system, tools, config);
        if self.store.contains_key(&key) {
            self.stats.hits += 1;
            self.store.get_mut(&key).unwrap().hit = true;
        } else {
            self.stats.misses += 1;
            self.store.insert(key, CacheEntry { key, hit: false });
            self.stats.entries = self.store.len();
        }
        self.store.get(&key).unwrap()
    }

    pub fn invalidate(&mut self, key: CacheKey) {
        if self.store.remove(&key).is_some() {
            self.stats.entries = self.store.len();
        }
    }

    pub fn stats(&self) -> &CacheStats { &self.stats }

    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 { 0.0 } else { self.stats.hits as f64 / total as f64 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = CacheKey::from_parts("sys", "tools", "cfg");
        let k2 = CacheKey::from_parts("sys", "tools", "cfg");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_different_parts() {
        let k1 = CacheKey::from_parts("sys", "tools", "cfg1");
        let k2 = CacheKey::from_parts("sys", "tools", "cfg2");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_miss_on_first_lookup() {
        let mut c = PromptCache::new();
        let entry = c.get_or_insert("sys", "tools", "cfg");
        assert!(!entry.hit);
        assert_eq!(c.stats().misses, 1);
    }

    #[test]
    fn test_hit_on_second_lookup() {
        let mut c = PromptCache::new();
        c.get_or_insert("sys", "tools", "cfg");
        let entry = c.get_or_insert("sys", "tools", "cfg");
        assert!(entry.hit);
        assert_eq!(c.stats().hits, 1);
    }

    #[test]
    fn test_hit_rate_zero_when_empty() {
        let c = PromptCache::new();
        assert_eq!(c.hit_rate(), 0.0);
    }

    #[test]
    fn test_hit_rate_correct() {
        let mut c = PromptCache::new();
        c.get_or_insert("s", "t", "c"); // miss
        c.get_or_insert("s", "t", "c"); // hit
        assert!((c.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_invalidate_removes_entry() {
        let mut c = PromptCache::new();
        let key = CacheKey::from_parts("s", "t", "c");
        c.get_or_insert("s", "t", "c");
        c.invalidate(key);
        assert_eq!(c.stats().entries, 0);
    }

    #[test]
    fn test_entry_count_grows() {
        let mut c = PromptCache::new();
        c.get_or_insert("s1", "t", "c");
        c.get_or_insert("s2", "t", "c");
        assert_eq!(c.stats().entries, 2);
    }
}
