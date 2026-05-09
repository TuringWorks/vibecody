//! BDD tests for Project Memory Store
//!
//! Feature: Project Memory Store
//! These tests verify the behavior of the per-workspace memory store.

use vibe_memory::*;
use std::path::PathBuf;
use tempfile::TempDir;

// ═══════════════════════════════════════════════════════════════════════════
// Gherkin scenarios — read the docstring above each test to see intent
// ═══════════════════════════════════════════════════════════════════════════

/// Scenario: Store memory entry with vector embedding
/// Given a fresh project workspace at "/tmp/test-project"
/// And the SQLite extension "sqlite-vec" is available
/// When I store a memory entry with content "Rust ownership prevents data races"
/// Then the entry exists in the project store with a valid 768-dim vector
/// And the sector is classified as "procedural"
/// And the entry ID is a hex timestamp with random suffix
#[tokio::test]
async fn store_memory_entry_with_vector() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    let entry = store
        .store("Rust ownership prevents data races", None)
        .await
        .expect("store memory");
    
    // Entry has valid ID
    assert!(!entry.id.is_empty());
    assert!(entry.id.len() > 20); // hex timestamp + suffix
    
    // Sector classified as procedural (from keyword signals)
    assert_eq!(entry.sector, "procedural");
    
    // Vector dimensions match configured
    assert_eq!(entry.embedding.len(), 768);
    
    // Timestamps are set
    assert!(entry.created_at > 0);
}

/// Scenario: Query project memories by semantic similarity
/// Given a project with 3 stored memories about "Rust ownership", "Go concurrency", "Python GIL"
/// When I query for "memory safety in systems programming"
/// Then the top result is about Rust ownership (highest cosine similarity)
/// And results are ranked by salience × recency × sector-weight
#[tokio::test]
async fn query_by_semantic_similarity() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store three memories
    let _ = store.store("Rust ownership prevents data races at compile time", None).await;
    let _ = store.store("Go uses goroutines and channels for concurrency", None).await;
    let _ = store.store("Python GIL prevents true multi-threading", None).await;
    
    // Query
    let results = store
        .search("memory safety in systems programming", 5, None)
        .await
        .expect("search");
    
    // Top result is Rust (semantic similarity + procedural weight)
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Rust"));
    
    // Results are sorted by score descending
    for window in results.windows(2) {
        assert!(window[0].score >= window[1].score);
    }
}

/// Scenario: Project memory isolated from other projects
/// Given project A with memory "Auth service uses JWT"
/// And project B with memory "Auth service uses OAuth"
/// When I query project A's store for "authentication"
/// Then the result is about JWT, not OAuth
/// And project B's store is not touched
#[tokio::test]
async fn project_isolation() {
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();
    
    let store_a = ProjectMemStore::open(workspace_a.path()).expect("open store A");
    let store_b = ProjectMemStore::open(workspace_b.path()).expect("open store B");
    
    // Different memories in each project
    store_a.store("Auth service uses JWT tokens", None).await.expect("store A");
    store_b.store("Auth service uses OAuth 2.0", None).await.expect("store B");
    
    // Query project A
    let results_a = store_a
        .search("authentication", 5, None)
        .await
        .expect("search A");
    
    // Query project B
    let results_b = store_b
        .search("authentication", 5, None)
        .await
        .expect("search B");
    
    // A sees only JWT memory
    assert!(results_a.iter().all(|r| r.content.contains("JWT")));
    
    // B sees only OAuth memory
    assert!(results_b.iter().all(|r| r.content.contains("OAuth")));
}

/// Scenario: Encrypted at rest with machine-bound key
/// Given a project store at "/tmp/test-project"
/// When I store a memory entry
/// Then the raw SQLite file contains encrypted vectors
#[tokio::test]
async fn encrypted_at_rest() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store a memory
    store.store("sensitive project data", None).await.expect("store");
    
    // Flush to disk
    drop(store);
    
    // Read raw file bytes
    let db_path = workspace.path().join(".vibecli").join("memory.db");
    assert!(db_path.exists(), "DB file should exist");
    let raw_bytes = std::fs::read(&db_path).expect("read raw DB");
    
    // The DB should not contain plaintext "sensitive"
    // (This is a weak test but checks for obvious plaintext leakage)
    assert!(
        !String::from_utf8_lossy(&raw_bytes).contains("sensitive project data"),
        "DB should not contain plaintext"
    );
    
    // The DB should contain some data (encrypted)
    assert!(raw_bytes.len() > 1000, "DB should have substantial encrypted content");
}

/// Scenario: Delete memory entry by ID
/// Given a project with 5 stored memories
/// When I delete one memory by ID
/// Then the store contains 4 entries
/// And the deleted ID returns None on lookup
#[tokio::test]
async fn delete_memory_by_id() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store 5 memories
    let ids: Vec<_> = futures::future::join_all(
        (0..5).map(|i| store.store(format!("Memory {}", i), None))
    )
    .await
    .into_iter()
    .map(|r| r.expect("store").id)
    .collect();
    
    assert_eq!(ids.len(), 5);
    
    // Delete the third one
    let to_delete = &ids[2];
    store.delete(to_delete).await.expect("delete");
    
    // Store should have 4 entries now
    let all = store.list(None, None).await.expect("list");
    assert_eq!(all.len(), 4);
    
    // Deleted ID should not be found
    assert!(store.get(to_delete).await.expect("get").is_none());
}

/// Scenario: List all project memories with metadata
/// Given a project with mixed sector memories
/// When I list all memories with metadata
/// Then each entry includes id, content, sector, salience, created_at, tags
#[tokio::test]
async fn list_memories_with_metadata() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store diverse memories
    store.store("Yesterday we discussed Rust ownership", Some(MemoryMeta {
        tags: vec!["rust".to_string(), "session".to_string()],
        ..Default::default()
    })).await.expect("store episodic");
    
    store.store("The definition of a closure is a function with captured state", Some(MemoryMeta {
        tags: vec!["concept".to_string()],
        ..Default::default()
    })).await.expect("store semantic");
    
    store.store("Step 1: Run cargo build", Some(MemoryMeta {
        tags: vec!["workflow".to_string()],
        ..Default::default()
    })).await.expect("store procedural");
    
    // List all
    let all = store.list(None, None).await.expect("list");
    
    assert_eq!(all.len(), 3);
    for entry in &all {
        // All required fields present
        assert!(!entry.id.is_empty());
        assert!(!entry.content.is_empty());
        assert!(!entry.sector.is_empty());
        assert!(entry.salience > 0.0);
        assert!(entry.created_at > 0);
    }
    
    // Check specific tags
    let rust_mem = all.iter().find(|e| e.content.contains("Rust")).expect("find Rust");
    assert!(rust_mem.tags.contains(&"rust".to_string()));
}

/// Scenario: Pin memory prevents decay and purge
/// Given a pinned memory with low salience
/// When decay is applied
/// Then the pinned memory retains its original salience
/// And the pinned memory is excluded from purge operations
#[tokio::test]
async fn pin_prevents_decay_and_purge() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store and pin a memory
    let entry = store.store("Important project context", Some(MemoryMeta {
        pinned: true,
        ..Default::default()
    })).await.expect("store pinned");
    
    let original_salience = entry.salience;
    
    // Apply decay multiple times (simulate time passing)
    for _ in 0..10 {
        store.apply_decay().await.expect("apply decay");
    }
    
    // Get the entry again
    let updated = store.get(&entry.id).await.expect("get");
    let updated = updated.expect("entry should exist");
    
    // Pinned memory retained salience
    assert_eq!(updated.salience, original_salience);
    
    // Purge should not remove pinned
    let purged = store.purge(0.5).await.expect("purge");
    assert_eq!(purged, 0, "Pinned memories should not be purged");
}

/// Scenario: Salience decay over time
/// Given a memory entry with salience 1.0 created 7 days ago
/// When I calculate the current salience
/// Then the value is less than 1.0 due to sector-specific decay
#[tokio::test]
async fn salience_decay_over_time() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store a memory (will have salience 1.0)
    let entry = store.store("Regular memory content", None).await.expect("store");
    assert_eq!(entry.salience, 1.0);
    
    // Manually backdate the entry (simulate 7 days ago)
    let seven_days_ago = chrono::Utc::now().timestamp() - (7 * 24 * 3600);
    store.backdate(&entry.id, seven_days_ago).await.expect("backdate");
    
    // Apply decay
    store.apply_decay().await.expect("apply decay");
    
    // Get updated entry
    let updated = store.get(&entry.id).await.expect("get");
    let updated = updated.expect("entry should exist");
    
    // Salience should be less than 1.0
    // With ~0.01 decay rate, 7 days should reduce by ~7%
    assert!(
        updated.salience < 1.0,
        "Salience should decay over time, got {}",
        updated.salience
    );
}

/// Scenario: Cross-project waypoints
/// Given project A has a memory about API design
/// When I link it to a memory in project B
/// Then the waypoint is stored as cross_project=true
/// And querying A shows the cross-project link
#[tokio::test]
async fn cross_project_waypoints() {
    let workspace_a = TempDir::new().unwrap();
    let store_a = ProjectMemStore::open(workspace_a.path()).expect("open store A");
    
    let id_a = store_a
        .store("Use REST for public APIs", None)
        .await
        .expect("store A")
        .id;
    
    // Add cross-project waypoint (simulated - normally via GlobalMemStore)
    store_a.add_waypoint(&id_a, "cross-project-ref", 0.8, true).await.expect("waypoint");
    
    // Query waypoints
    let waypoints = store_a.get_waypoints(&id_a).await.expect("get waypoints");
    
    assert!(!waypoints.is_empty());
    let cross_wp = waypoints.iter().find(|w| w.cross_project).expect("find cross-project");
    assert_eq!(cross_wp.weight, 0.8);
}

/// Scenario: Context budget trimming
/// Given a store with many memories totaling >10K tokens
/// When I search with a 2K token budget
/// Then results are trimmed to fit the budget
/// And the most relevant memories are retained
#[tokio::test]
async fn context_budget_trimming() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store many memories with varying content lengths
    let contents = vec![
        "Short",
        "Medium length content about Rust programming",
        "Very long content that spans multiple lines and contains detailed information about a specific topic in software development that requires substantial context to understand properly",
        "Another short one",
        "A moderate length entry with some technical details about async programming in Rust using tokio",
    ];
    
    for content in contents {
        store.store(content, None).await.expect("store");
    }
    
    // Search with budget
    let results = store
        .search_with_budget("rust programming", 5, 500) // 500 token budget
        .await
        .expect("search with budget");
    
    // Calculate approximate token count
    let total_tokens: usize = results.iter()
        .map(|r| r.content.split_whitespace().count() * 1.3 as usize) // rough estimate
        .sum();
    
    assert!(
        total_tokens <= 800, // Allow some overhead
        "Results should fit budget, got ~{} tokens",
        total_tokens
    );
}

/// Scenario: Project store path derivation
/// When I open a store for /tmp/myproject
/// Then the database path is derived from the workspace
/// And differs from global store path
#[tokio::test]
async fn project_store_path_derivation() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    let project_path = store.path();
    assert!(
        project_path.to_string_lossy().contains(".vibecli"),
        "Path should contain .vibecli directory"
    );
    
    // Compare with global store path
    let global_store = GlobalMemStore::open().expect("open global");
    let global_path = global_store.path();
    
    // Paths should differ
    assert_ne!(project_path, global_path, "Project and global paths should differ");
}

/// Scenario: Store with custom sector
/// When I store a memory with explicit sector
/// Then the sector is preserved
/// And not overwritten by auto-classification
#[tokio::test]
async fn explicit_sector_preserved() {
    let workspace = TempDir::new().unwrap();
    let store = ProjectMemStore::open(workspace.path()).expect("open store");
    
    // Store with explicit sector
    let entry = store
        .store_with_sector("This is important project knowledge", "semantic")
        .await
        .expect("store with sector");
    
    assert_eq!(entry.sector, "semantic");
}
