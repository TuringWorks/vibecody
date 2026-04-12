#![allow(dead_code)]
//! Branch locking to prevent concurrent agent modifications.
//!
//! Claw-code parity Wave 2: prevents two agent sessions from editing the same
//! branch simultaneously, avoiding merge conflicts and corrupted state.
//!
//! # Collision detection (Wave 2 extension)
//!
//! Detects three collision types:
//! - `Exact` — two agents trying to lock the same branch
//! - `ParentChild` — one branch is a prefix of the other (e.g. "feature" vs "feature/login")
//! - `NestedModule` — path-based module hierarchy (e.g. "src/auth" vs "src/auth/oauth")
//!
//! Read locks coexist; Write and Exclusive locks conflict.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Lock Entry ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchLock {
    pub branch: String,
    pub session_id: String,
    pub acquired_at_ms: u64,
    /// TTL in ms; 0 = no expiry.
    pub ttl_ms: u64,
    pub reason: String,
}

impl BranchLock {
    pub fn new(branch: impl Into<String>, session_id: impl Into<String>, at_ms: u64, ttl_ms: u64, reason: impl Into<String>) -> Self {
        Self { branch: branch.into(), session_id: session_id.into(), acquired_at_ms: at_ms, ttl_ms, reason: reason.into() }
    }

    /// True if the lock has expired at `now_ms`.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.ttl_ms > 0 && now_ms > self.acquired_at_ms + self.ttl_ms
    }

    /// Age of lock in seconds.
    pub fn age_seconds(&self, now_ms: u64) -> u64 {
        (now_ms - self.acquired_at_ms) / 1000
    }
}

// ─── Lock Result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LockResult {
    Acquired,
    AlreadyOwned,
    Denied { held_by: String, age_seconds: u64 },
    Expired { previous_holder: String },
}

// ─── Lock Registry ────────────────────────────────────────────────────────────

pub struct BranchLockRegistry {
    locks: HashMap<String, BranchLock>,
    /// Default TTL for new locks in ms (0 = no expiry).
    pub default_ttl_ms: u64,
}

impl BranchLockRegistry {
    pub fn new(default_ttl_ms: u64) -> Self { Self { locks: HashMap::new(), default_ttl_ms } }

    /// Try to acquire a lock on `branch` for `session_id`.
    pub fn acquire(&mut self, branch: &str, session_id: &str, now_ms: u64, reason: &str) -> LockResult {
        // Expire stale lock first
        if let Some(existing) = self.locks.get(branch) {
            if existing.is_expired(now_ms) {
                let prev = existing.session_id.clone();
                self.locks.remove(branch);
                // Fall through to acquire
                self.locks.insert(branch.to_string(),
                    BranchLock::new(branch, session_id, now_ms, self.default_ttl_ms, reason));
                return LockResult::Expired { previous_holder: prev };
            }
            if existing.session_id == session_id {
                return LockResult::AlreadyOwned;
            }
            return LockResult::Denied { held_by: existing.session_id.clone(), age_seconds: existing.age_seconds(now_ms) };
        }
        self.locks.insert(branch.to_string(),
            BranchLock::new(branch, session_id, now_ms, self.default_ttl_ms, reason));
        LockResult::Acquired
    }

    /// Release lock if held by `session_id`.
    pub fn release(&mut self, branch: &str, session_id: &str) -> bool {
        if self.locks.get(branch).map(|l| l.session_id == session_id).unwrap_or(false) {
            self.locks.remove(branch);
            true
        } else {
            false
        }
    }

    /// Force-release a lock regardless of owner (admin operation).
    pub fn force_release(&mut self, branch: &str) -> Option<BranchLock> {
        self.locks.remove(branch)
    }

    /// Renew lock TTL for `session_id`.
    pub fn renew(&mut self, branch: &str, session_id: &str, now_ms: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(branch) {
            if lock.session_id == session_id {
                lock.acquired_at_ms = now_ms;
                return true;
            }
        }
        false
    }

    pub fn is_locked(&self, branch: &str, now_ms: u64) -> bool {
        self.locks.get(branch).map(|l| !l.is_expired(now_ms)).unwrap_or(false)
    }

    pub fn lock_info(&self, branch: &str) -> Option<&BranchLock> { self.locks.get(branch) }

    /// All active (non-expired) locks at `now_ms`.
    pub fn active_locks(&self, now_ms: u64) -> Vec<&BranchLock> {
        self.locks.values().filter(|l| !l.is_expired(now_ms)).collect()
    }

    /// Release all locks held by a session (cleanup on session end).
    pub fn release_all_for_session(&mut self, session_id: &str) -> Vec<String> {
        let to_remove: Vec<String> = self.locks.iter()
            .filter(|(_, l)| l.session_id == session_id)
            .map(|(b, _)| b.clone())
            .collect();
        for b in &to_remove { self.locks.remove(b); }
        to_remove
    }
}

impl Default for BranchLockRegistry {
    fn default() -> Self { Self::new(300_000) } // 5 min default TTL
}

// ── CollisionType ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionType {
    Exact,
    ParentChild,
    NestedModule,
}

impl std::fmt::Display for CollisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact => write!(f, "exact"),
            Self::ParentChild => write!(f, "parent_child"),
            Self::NestedModule => write!(f, "nested_module"),
        }
    }
}

// ── LockIntent ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockIntent {
    Read,
    Write,
    Exclusive,
}

impl std::fmt::Display for LockIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Exclusive => write!(f, "exclusive"),
        }
    }
}

impl LockIntent {
    /// Returns true if this intent conflicts with an existing lock.
    /// Read + Read is always safe; anything else conflicts.
    pub fn conflicts_with(&self, other: &LockIntent) -> bool {
        !matches!((self, other), (LockIntent::Read, LockIntent::Read))
    }
}

// ── LockEntry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub branch: String,
    pub lane_id: String,
    pub intent: LockIntent,
    pub acquired_at: u64,
}

// ── Collision ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Collision {
    pub collision_type: CollisionType,
    pub existing_lock: LockEntry,
    pub requested_branch: String,
}

// ── CollisionRegistry ─────────────────────────────────────────────────────────
//
// A separate, Arc-based registry for parallel-agent collision detection.
// Named `CollisionRegistry` to avoid clashing with the session-based
// `BranchLockRegistry` above.

#[derive(Debug, Clone, Default)]
pub struct CollisionRegistry {
    locks: Arc<Mutex<HashMap<String, LockEntry>>>,
}

impl CollisionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if `a` is a parent of `b` or vice-versa (b starts with "a/" or a starts with "b/").
    pub fn is_parent_child(a: &str, b: &str) -> bool {
        if a == b {
            return false;
        }
        let a_prefix = format!("{a}/");
        let b_prefix = format!("{b}/");
        b.starts_with(&a_prefix) || a.starts_with(&b_prefix)
    }

    /// Returns true if `a` and `b` share a common module path prefix (splitting on '/').
    pub fn is_nested_module(a: &str, b: &str) -> bool {
        if a == b {
            return false;
        }
        let a_parts: Vec<&str> = a.split('/').collect();
        let b_parts: Vec<&str> = b.split('/').collect();
        if a_parts.len() < b_parts.len() {
            b_parts.starts_with(&a_parts)
        } else {
            a_parts.starts_with(&b_parts)
        }
    }

    /// Detect all collisions for a requested branch + intent against current locks.
    pub fn detect_collisions(&self, branch: &str, intent: &LockIntent) -> Vec<Collision> {
        let locks = self.locks.lock().unwrap();
        let mut collisions = Vec::new();

        for (locked_branch, entry) in locks.iter() {
            if !intent.conflicts_with(&entry.intent) {
                continue;
            }

            if locked_branch == branch {
                collisions.push(Collision {
                    collision_type: CollisionType::Exact,
                    existing_lock: entry.clone(),
                    requested_branch: branch.to_string(),
                });
            } else if Self::is_nested_module(locked_branch, branch) {
                collisions.push(Collision {
                    collision_type: CollisionType::NestedModule,
                    existing_lock: entry.clone(),
                    requested_branch: branch.to_string(),
                });
            } else if Self::is_parent_child(locked_branch, branch) {
                collisions.push(Collision {
                    collision_type: CollisionType::ParentChild,
                    existing_lock: entry.clone(),
                    requested_branch: branch.to_string(),
                });
            }
        }
        collisions
    }

    /// Deduplicate collisions: if Exact exists for the same branch keep Exact, drop ParentChild/NestedModule.
    pub fn deduplicate_collisions(mut collisions: Vec<Collision>) -> Vec<Collision> {
        let has_exact: std::collections::HashSet<String> = collisions
            .iter()
            .filter(|c| c.collision_type == CollisionType::Exact)
            .map(|c| c.existing_lock.branch.clone())
            .collect();
        collisions.retain(|c| {
            c.collision_type == CollisionType::Exact
                || !has_exact.contains(&c.existing_lock.branch)
        });
        collisions
    }

    /// Attempt to acquire a lock. Returns `Err` with collisions if any exist.
    /// Same `lane_id` can re-acquire (upgrade) without collision.
    pub fn acquire(
        &self,
        branch: &str,
        lane_id: &str,
        intent: LockIntent,
    ) -> Result<(), Vec<Collision>> {
        {
            let mut locks = self.locks.lock().unwrap();
            if let Some(existing) = locks.get(branch) {
                if existing.lane_id == lane_id {
                    // Upgrade — always allowed
                    locks.insert(
                        branch.to_string(),
                        LockEntry {
                            branch: branch.to_string(),
                            lane_id: lane_id.to_string(),
                            intent,
                            acquired_at: now_millis(),
                        },
                    );
                    return Ok(());
                }
            }
        } // release the lock guard before calling detect_collisions

        let collisions = self.detect_collisions(branch, &intent);
        if !collisions.is_empty() {
            return Err(collisions);
        }

        let mut locks = self.locks.lock().unwrap();
        locks.insert(
            branch.to_string(),
            LockEntry {
                branch: branch.to_string(),
                lane_id: lane_id.to_string(),
                intent,
                acquired_at: now_millis(),
            },
        );
        Ok(())
    }

    /// Release a lock. Returns true if the lock was held by the given lane.
    pub fn release(&self, branch: &str, lane_id: &str) -> bool {
        let mut locks = self.locks.lock().unwrap();
        if let Some(entry) = locks.get(branch) {
            if entry.lane_id == lane_id {
                locks.remove(branch);
                return true;
            }
        }
        false
    }

    pub fn lock_count(&self) -> usize {
        self.locks.lock().unwrap().len()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> BranchLockRegistry { BranchLockRegistry::new(60_000) }

    #[test]
    fn test_acquire_success() {
        let mut r = reg();
        assert_eq!(r.acquire("main", "s1", 0, "feat"), LockResult::Acquired);
    }

    #[test]
    fn test_acquire_same_session_already_owned() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        assert_eq!(r.acquire("main", "s1", 1000, "feat"), LockResult::AlreadyOwned);
    }

    #[test]
    fn test_acquire_denied_by_other_session() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        let result = r.acquire("main", "s2", 10_000, "other");
        assert!(matches!(result, LockResult::Denied { held_by, .. } if held_by == "s1"));
    }

    #[test]
    fn test_acquire_after_expiry() {
        let mut r = BranchLockRegistry::new(5_000); // 5s TTL
        r.acquire("main", "s1", 0, "feat");
        let result = r.acquire("main", "s2", 6_000, "new");
        assert!(matches!(result, LockResult::Expired { previous_holder } if previous_holder == "s1"));
    }

    #[test]
    fn test_release_by_owner() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        assert!(r.release("main", "s1"));
        assert!(!r.is_locked("main", 0));
    }

    #[test]
    fn test_release_by_non_owner_fails() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        assert!(!r.release("main", "s2"));
        assert!(r.is_locked("main", 0));
    }

    #[test]
    fn test_force_release() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        let removed = r.force_release("main");
        assert!(removed.is_some());
        assert!(!r.is_locked("main", 0));
    }

    #[test]
    fn test_renew_extends_ttl() {
        let mut r = BranchLockRegistry::new(5_000);
        r.acquire("main", "s1", 0, "feat");
        r.renew("main", "s1", 4_000); // renew at 4s
        // Without renewal would have expired at 5s; with renewal, not expired at 8s
        assert!(!r.lock_info("main").unwrap().is_expired(8_000));
    }

    #[test]
    fn test_renew_wrong_session_fails() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "feat");
        assert!(!r.renew("main", "s2", 1000));
    }

    #[test]
    fn test_is_locked_false_when_not_acquired() {
        let r = reg();
        assert!(!r.is_locked("main", 0));
    }

    #[test]
    fn test_active_locks_excludes_expired() {
        let mut r = BranchLockRegistry::new(5_000);
        r.acquire("main", "s1", 0, "a");
        r.acquire("dev", "s2", 0, "b");
        let active = r.active_locks(6_000); // both expired
        assert!(active.is_empty());
    }

    #[test]
    fn test_active_locks_includes_fresh() {
        let mut r = BranchLockRegistry::new(60_000);
        r.acquire("main", "s1", 0, "a");
        r.acquire("dev", "s2", 0, "b");
        let active = r.active_locks(1_000);
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_release_all_for_session() {
        let mut r = reg();
        r.acquire("main", "s1", 0, "a");
        r.acquire("dev", "s1", 0, "b");
        r.acquire("feature", "s2", 0, "c");
        let released = r.release_all_for_session("s1");
        assert_eq!(released.len(), 2);
        assert!(!r.is_locked("main", 0));
        assert!(!r.is_locked("dev", 0));
        assert!(r.is_locked("feature", 0));
    }

    #[test]
    fn test_lock_age_calculation() {
        let lock = BranchLock::new("main", "s1", 0, 60_000, "feat");
        assert_eq!(lock.age_seconds(5_000), 5);
    }

    #[test]
    fn test_lock_not_expired_within_ttl() {
        let lock = BranchLock::new("main", "s1", 0, 60_000, "feat");
        assert!(!lock.is_expired(30_000));
    }

    #[test]
    fn test_lock_expired_past_ttl() {
        let lock = BranchLock::new("main", "s1", 0, 60_000, "feat");
        assert!(lock.is_expired(61_000));
    }

    #[test]
    fn test_no_expiry_when_ttl_zero() {
        let lock = BranchLock::new("main", "s1", 0, 0, "permanent");
        assert!(!lock.is_expired(999_999_999));
    }

    // ── CollisionRegistry tests ────────────────────────────────────────────────

    #[test]
    fn acquire_succeeds_on_empty_registry() {
        let reg = CollisionRegistry::new();
        assert!(reg.acquire("feature/auth", "lane-1", LockIntent::Write).is_ok());
    }

    #[test]
    fn acquire_fails_on_exact_collision() {
        let reg = CollisionRegistry::new();
        reg.acquire("feature/auth", "lane-1", LockIntent::Write).unwrap();
        let err = reg.acquire("feature/auth", "lane-2", LockIntent::Write).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].collision_type, CollisionType::Exact);
    }

    #[test]
    fn detect_parent_child_collision() {
        let reg = CollisionRegistry::new();
        reg.acquire("feature", "lane-1", LockIntent::Write).unwrap();
        let err = reg.acquire("feature/login", "lane-2", LockIntent::Write).unwrap_err();
        assert!(err.iter().any(|c| c.collision_type == CollisionType::ParentChild
            || c.collision_type == CollisionType::NestedModule));
    }

    #[test]
    fn detect_nested_module_collision() {
        let reg = CollisionRegistry::new();
        reg.acquire("src/auth", "lane-1", LockIntent::Write).unwrap();
        let err = reg.acquire("src/auth/oauth", "lane-2", LockIntent::Write).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn is_parent_child_recognizes_hierarchy() {
        assert!(CollisionRegistry::is_parent_child("feature", "feature/login"));
        assert!(CollisionRegistry::is_parent_child("feature/login", "feature"));
    }

    #[test]
    fn is_parent_child_rejects_sibling() {
        assert!(!CollisionRegistry::is_parent_child("feature/auth", "feature/ui"));
    }

    #[test]
    fn release_allows_reacquisition() {
        let reg = CollisionRegistry::new();
        reg.acquire("develop", "lane-1", LockIntent::Write).unwrap();
        assert!(reg.release("develop", "lane-1"));
        assert!(reg.acquire("develop", "lane-2", LockIntent::Write).is_ok());
    }

    #[test]
    fn same_lane_can_reacquire() {
        let reg = CollisionRegistry::new();
        reg.acquire("main", "lane-1", LockIntent::Read).unwrap();
        assert!(reg.acquire("main", "lane-1", LockIntent::Write).is_ok());
    }

    #[test]
    fn read_read_does_not_collide() {
        let reg = CollisionRegistry::new();
        reg.acquire("main", "lane-1", LockIntent::Read).unwrap();
        assert!(reg.acquire("main", "lane-2", LockIntent::Read).is_ok());
    }

    #[test]
    fn deduplicate_removes_redundant_parent_child() {
        let entry = LockEntry {
            branch: "feature".into(),
            lane_id: "lane-1".into(),
            intent: LockIntent::Write,
            acquired_at: 0,
        };
        let collisions = vec![
            Collision {
                collision_type: CollisionType::Exact,
                existing_lock: entry.clone(),
                requested_branch: "feature".into(),
            },
            Collision {
                collision_type: CollisionType::ParentChild,
                existing_lock: entry.clone(),
                requested_branch: "feature/sub".into(),
            },
        ];
        let deduped = CollisionRegistry::deduplicate_collisions(collisions);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].collision_type, CollisionType::Exact);
    }
}
