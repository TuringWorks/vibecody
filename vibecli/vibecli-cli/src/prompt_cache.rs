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
