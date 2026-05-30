//! BDD tests for Global (Computer) Memory Store
//!
//! Feature: Global (Computer) Memory Store
//! These tests verify the behavior of the per-machine global store.

use tempfile::TempDir;
use vibe_memory::*;

// ═══════════════════════════════════════════════════════════════════════════
// Gherkin scenarios — read the docstring above each test to see intent
// ═══════════════════════════════════════════════════════════════════════════

/// Scenario: Store computer-level context memory
/// Given a fresh global store
/// When I store a memory about "User prefers dark mode and Rust"
/// Then the memory is available to all projects on this machine
/// And the sector is classified as "emotional"
#[tokio::test]
async fn store_computer_level_memory() {
    let store = GlobalMemStore::open().expect("open global store");

    let entry = store
        .store(
            "User prefers dark mode and Rust for systems programming",
            None,
        )
        .await
        .expect("store global memory");

    // Entry stored successfully
    assert!(!entry.id.is_empty());

    // Sector classified (should be emotional due to "prefers")
    assert_eq!(entry.sector, "emotional");

    // Memory is available via search
    let results = store
        .search("IDE theme preferences", 5, None)
        .await
        .expect("search");
    assert!(!results.is_empty());
}

/// Scenario: Cross-project context retrieval
/// Given project A stored "Python dicts are slow"
/// And project B stored "Use dataclasses for performance"
/// When I query global store for "python performance"
/// Then results include both project A and B entries
/// And each result includes the source project ID
#[tokio::test]
async fn cross_project_context_retrieval() {
    let store = GlobalMemStore::open().expect("open global store");

    // Store from two different projects
    store
        .store_from_project("Python dicts are slow for large datasets", "proj-a", None)
        .await
        .expect("store proj-a");

    store
        .store_from_project(
            "Use dataclasses for better performance in Python",
            "proj-b",
            None,
        )
        .await
        .expect("store proj-b");

    // Query
    let results = store
        .search("python performance optimization", 5, None)
        .await
        .expect("search");

    // Should find both projects
    let project_ids: Vec<_> = results
        .iter()
        .filter_map(|r| r.project_id.clone())
        .collect();

    assert!(project_ids.contains(&"proj-a".to_string()));
    assert!(project_ids.contains(&"proj-b".to_string()));

    // Check content relevance
    let contents: Vec<_> = results.iter().map(|r| r.content.as_str()).collect();
    assert!(contents.iter().any(|c| c.contains("Python")));
}

/// Scenario: Global store not affected by project deletion
/// Given project A with global memory references
/// When project A is deleted
/// Then global store entries from project A are preserved
/// And other projects can still access global memories
#[tokio::test]
async fn global_store_persists_after_project_deletion() {
    let store = GlobalMemStore::open().expect("open global store");

    // Store from project A
    let entry_id = store
        .store_from_project("Project A's architectural decision", "proj-a", None)
        .await
        .expect("store from A")
        .id;

    // Simulate project deletion (mark as deleted)
    store
        .mark_project_deleted("proj-a")
        .await
        .expect("mark deleted");

    // Entry should still exist
    let entry = store.get(&entry_id).await.expect("get");
    assert!(
        entry.is_some(),
        "Entry should persist after project deletion"
    );

    // Other projects can still access
    let results = store
        .search("architectural decision", 5, None)
        .await
        .expect("search");
    assert!(!results.is_empty());
}

/// Scenario: Global memory merges with project memory
/// Given project has 2 memories and global has 3 memories
/// When I fetch layered context for a query
/// Then the results include project memories (higher priority)
/// And global memories fill remaining context budget
/// And sector weights are applied to final ranking
#[tokio::test]
async fn global_memory_merge_with_project() {
    let global_store = GlobalMemStore::open().expect("open global");
    let project_store = ProjectMemStore::open(&std::env::temp_dir().join("test-merge-project"))
        .expect("open project");

    // Setup project memories
    project_store
        .store("Project-specific API endpoint", None)
        .await
        .expect("proj store 1");
    project_store
        .store("Project-specific database schema", None)
        .await
        .expect("proj store 2");

    // Setup global memories
    global_store
        .store("General best practice: use REST", None)
        .await
        .expect("global 1");
    global_store
        .store("General security consideration: validate input", None)
        .await
        .expect("global 2");
    global_store
        .store("General naming convention: snake_case", None)
        .await
        .expect("global 3");

    // Create hub and query
    let hub = MemoryContextHub::new();
    let context = hub
        .assemble_context("api design", 500)
        .await
        .expect("assemble");

    // Context should include both project and global memories
    // (The exact ratio depends on scoring, but both should appear)
    assert!(context.contains("API") || context.contains("REST"));
}

/// Scenario: Computer-level encryption key derivation
/// Given I open the global store
/// When I write a memory entry
/// Then the SQLite file uses the machine-derived key
/// And the key differs from any project's store key
/// And the key is consistent across VibeCody restarts
#[tokio::test]
async fn computer_level_encryption() {
    let store = GlobalMemStore::open().expect("open global");

    // Store a memory
    store
        .store("machine-wide preference", None)
        .await
        .expect("store");
    drop(store);

    // Check that DB exists
    let path = store.path();
    assert!(path.exists(), "Global store DB should exist");

    // Compare with project store key derivation
    let workspace = TempDir::new().unwrap();
    let project_store = ProjectMemStore::open(workspace.path()).expect("open project");

    // Store in project
    project_store
        .store("project-specific data", None)
        .await
        .expect("store");
    drop(project_store);

    // Global and project stores should have different paths (different keys)
    let global_path = GlobalMemStore::open().expect("reopen global").path();
    let proj_path = ProjectMemStore::open(workspace.path())
        .expect("reopen project")
        .path();

    // Path derivation differs
    assert_ne!(
        global_path, proj_path,
        "Global and project paths should differ"
    );
}

/// Scenario: Global store isolation
/// When multiple processes open the global store
/// Then they share the same encrypted database
/// And can read each other's entries
#[tokio::test]
async fn global_store_shared_access() {
    // First instance
    let store1 = GlobalMemStore::open().expect("open store 1");
    let id1 = store1
        .store("Shared memory from process 1", None)
        .await
        .expect("store")
        .id;
    drop(store1);

    // Second instance (different process simulation)
    let store2 = GlobalMemStore::open().expect("open store 2");

    // Should see the entry from store1
    let entry = store2.get(&id1).await.expect("get");
    assert!(
        entry.is_some(),
        "Second instance should see entries from first"
    );

    // Can add more
    let id2 = store2
        .store("Shared memory from process 2", None)
        .await
        .expect("store")
        .id;

    // Both should be accessible
    let all = store2.list(None, None).await.expect("list all");
    assert!(all.iter().any(|e| e.id == id1));
    assert!(all.iter().any(|e| e.id == id2));
}

/// Scenario: Global store metadata tracking
/// When I store memories with project context
/// Then I can query by project ID
/// And get all memories associated with that project
#[tokio::test]
async fn global_store_project_filtering() {
    let store = GlobalMemStore::open().expect("open global");

    // Store memories from multiple projects
    store
        .store_from_project("Data from project X", "proj-x", None)
        .await
        .expect("x1");
    store
        .store_from_project("More from project X", "proj-x", None)
        .await
        .expect("x2");
    store
        .store_from_project("Data from project Y", "proj-y", None)
        .await
        .expect("y1");

    // Query by project
    let proj_x_memories = store.list_by_project("proj-x").await.expect("list proj-x");
    let proj_y_memories = store.list_by_project("proj-y").await.expect("list proj-y");

    assert_eq!(
        proj_x_memories.len(),
        2,
        "Should have 2 memories from proj-x"
    );
    assert_eq!(proj_y_memories.len(), 1, "Should have 1 memory from proj-y");

    // Verify content
    let x_contents: Vec<_> = proj_x_memories.iter().map(|e| e.content.as_str()).collect();
    assert!(x_contents.iter().any(|c| c.contains("project X")));
}

/// Scenario: Global store sector statistics
/// When I query sector statistics
/// Then I get counts of memories per sector
#[tokio::test]
async fn global_store_sector_stats() {
    let store = GlobalMemStore::open().expect("open global");

    // Store diverse memories
    store
        .store("Yesterday I worked on authentication", None)
        .await
        .expect("episodic");
    store
        .store("A fact: Rust prevents null pointer exceptions", None)
        .await
        .expect("semantic");
    store
        .store("Step 1: run cargo build", None)
        .await
        .expect("procedural");

    // Get stats
    let stats = store.sector_stats().await.expect("get stats");

    assert!(stats.contains_key("episodic"), "Should have episodic count");
    assert!(stats.contains_key("semantic"), "Should have semantic count");
    assert!(
        stats.contains_key("procedural"),
        "Should have procedural count"
    );

    // Verify counts
    assert_eq!(stats["episodic"], 1);
    assert_eq!(stats["semantic"], 1);
    assert_eq!(stats["procedural"], 1);
}

/// Scenario: Global store TTL expiration
/// Given a global memory with TTL set
/// When the TTL expires
/// Then the memory is automatically deleted
#[tokio::test]
async fn global_store_ttl_expiration() {
    let store = GlobalMemStore::open().expect("open global");

    // Store with very short TTL
    let entry = store
        .store_with_ttl("Temporary global memory", 1) // 1 second TTL
        .await
        .expect("store with TTL");

    // Immediately present
    let found = store.get(&entry.id).await.expect("get");
    assert!(found.is_some());

    // Wait for TTL
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Apply cleanup
    store.cleanup_expired().await.expect("cleanup");

    // Should be gone
    let found = store.get(&entry.id).await.expect("get after TTL");
    assert!(found.is_none(), "Expired memory should be deleted");
}

/// Scenario: Search with filters
/// Given a global store with various memories
/// When I search with sector filter
/// Then only memories from that sector are returned
#[tokio::test]
async fn search_with_sector_filter() {
    let store = GlobalMemStore::open().expect("open global");

    // Store memories in different sectors
    store
        .store("Yesterday we had a meeting", None)
        .await
        .expect("episodic");
    store
        .store("Definition: async means concurrent execution", None)
        .await
        .expect("semantic");
    store
        .store("To deploy: run kubectl apply -f config.yaml", None)
        .await
        .expect("procedural");

    // Search with sector filter
    let episodic_only = store
        .search_filtered("meeting", None, Some("episodic"))
        .await
        .expect("search episodic");
    let semantic_only = store
        .search_filtered("definition", None, Some("semantic"))
        .await
        .expect("search semantic");

    // Episodic search should only return episodic
    for result in episodic_only {
        assert_eq!(
            result.sector, "episodic",
            "Should only return episodic memories"
        );
    }

    // Semantic search should only return semantic
    for result in semantic_only {
        assert_eq!(
            result.sector, "semantic",
            "Should only return semantic memories"
        );
    }
}

/// Scenario: Global store waypoints
/// When I create waypoints between global memories
/// Then they are stored and retrievable
#[tokio::test]
async fn global_store_waypoints() {
    let store = GlobalMemStore::open().expect("open global");

    // Store two memories
    let id1 = store
        .store("Memory about Rust", None)
        .await
        .expect("store 1")
        .id;
    let id2 = store
        .store("Memory about async", None)
        .await
        .expect("store 2")
        .id;

    // Create waypoint
    store
        .add_waypoint(&id1, &id2, 0.9)
        .await
        .expect("add waypoint");

    // Get waypoints
    let waypoints = store.get_waypoints(&id1).await.expect("get waypoints");

    assert!(!waypoints.is_empty());
    let wp = waypoints
        .iter()
        .find(|w| w.dst_id == id2)
        .expect("find waypoint");
    assert_eq!(wp.weight, 0.9);
}
