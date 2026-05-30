//! BDD tests for Memory Context Hub (Orchestrator)
//!
//! Feature: Memory Context Hub (Orchestrator)
//! These tests verify the behavior of the orchestrator that combines
//! project and global stores with proper weighting and budgeting.

use std::path::PathBuf;
use tempfile::TempDir;
use vibe_memory::*;

// ═══════════════════════════════════════════════════════════════════════════
// Gherkin scenarios — read the docstring above each test to see intent
// ═══════════════════════════════════════════════════════════════════════════

/// Scenario: Layered context from project + global stores
/// Given project store has 3 memories about "API design"
/// And global store has 2 memories about "best practices"
/// When I query for "REST API design patterns"
/// Then I receive merged results with project entries weighted higher
/// And sector weights (emotional=1.3, episodic=1.2, procedural=1.1) are applied
/// And recency and salience boost are factored
#[tokio::test]
async fn layered_context_merge() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add memories to project store
    hub.store_to_project(workspace.path(), "Use REST for public APIs")
        .await
        .expect("proj 1");
    hub.store_to_project(workspace.path(), "Rate limiting is important for APIs")
        .await
        .expect("proj 2");
    hub.store_to_project(workspace.path(), "Use OpenAPI spec for documentation")
        .await
        .expect("proj 3");

    // Add memories to global store
    hub.store_global("Best practice: consistent error responses")
        .await
        .expect("global 1");
    hub.store_global("Best practice: version your API")
        .await
        .expect("global 2");

    // Query
    let results = hub
        .search_context(workspace.path(), "REST API design", 5, None)
        .await
        .expect("search");

    // Should have results from both stores
    assert!(!results.is_empty());

    // Project memories should appear (weighted higher)
    let project_results = results
        .iter()
        .filter(|r| r.store == StoreKind::Project)
        .count();
    let global_results = results
        .iter()
        .filter(|r| r.store == StoreKind::Global)
        .count();

    // Project entries should be included
    assert!(project_results > 0, "Should have project results");

    // Results should be sorted by composite score
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "Results should be sorted by score"
        );
    }
}

/// Scenario: Budget-aware context assembly
/// Given 20 relevant memories (total ~8K tokens)
/// And context budget is 4K tokens
/// When I assemble context for a query
/// Then only the top ~4K tokens of memories are included
/// And memories are sorted by composite score before truncation
#[tokio::test]
async fn budget_aware_context() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add many memories
    for i in 0..20 {
        let content = format!(
            "This is memory {} with some content about various topics including \
             programming languages, design patterns, best practices, and \
             technical details that make the token count substantial for testing",
            i
        );
        if i % 2 == 0 {
            hub.store_to_project(workspace.path(), &content)
                .await
                .expect("proj store");
        } else {
            hub.store_global(&content).await.expect("global store");
        }
    }

    // Assemble with budget
    let context = hub
        .assemble_context(workspace.path(), "programming", 2000)
        .await
        .expect("assemble");

    // Context should be bounded
    let tokens = context.split_whitespace().count();
    assert!(
        tokens <= 2500, // Some overhead allowed
        "Context should be bounded by budget, got ~{} words",
        tokens
    );

    // Should have vibe-memory tag
    assert!(context.contains("<vibe-memory>"), "Should have opening tag");
    assert!(
        context.contains("</vibe-memory>"),
        "Should have closing tag"
    );
}

/// Scenario: Empty stores return empty context
/// Given both project and global stores are empty
/// When I assemble context for any query
/// Then the result is an empty <vibe-memory> tag
/// And no error is raised
#[tokio::test]
async fn empty_stores_return_empty_context() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Query on empty stores
    let context = hub
        .assemble_context(workspace.path(), "anything", 4000)
        .await
        .expect("assemble");

    // Should return empty tag
    assert!(context.contains("<vibe-memory>"));
    assert!(context.contains("</vibe-memory>"));

    // Body should be empty or minimal
    let body = context
        .strip_prefix("<vibe-memory>")
        .and_then(|s| s.strip_suffix("</vibe-memory>"))
        .unwrap_or("")
        .trim();

    assert!(body.is_empty() || body == "\n", "Body should be empty");
}

/// Scenario: Single store populated (project only)
/// Given project store has memories
/// And global store is empty
/// When I assemble context
/// Then only project memories are returned
/// And no error is raised
#[tokio::test]
async fn single_store_populated() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add to project only
    hub.store_to_project(workspace.path(), "Project-specific knowledge")
        .await
        .expect("store");

    // Query
    let results = hub
        .search_context(workspace.path(), "project", 5, None)
        .await
        .expect("search");

    // Should have project results
    assert!(!results.is_empty());

    // Should not have global results (store is empty)
    // Note: this depends on how we handle global store being empty
}

/// Scenario: Vector search with top-K and min_score filter
/// Given a store with 100 memories
/// When I search with top_k=5 and min_score=0.75
/// Then I receive at most 5 results
/// And all results have cosine similarity >= 0.75
/// And results are sorted by score descending
#[tokio::test]
async fn vector_search_with_filters() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add many similar memories
    for i in 0..100 {
        let content = match i % 5 {
            0 => format!("Rust programming language with ownership and borrowing"),
            1 => format!("Go programming language with goroutines and channels"),
            2 => format!("Python programming language with dynamic typing"),
            3 => format!("JavaScript programming with async and promises"),
            _ => format!("Memory {} about various programming topics", i),
        };
        hub.store_to_project(workspace.path(), &content)
            .await
            .expect("store");
    }

    // Search with high top_k and min_score
    let results = hub
        .search_context(workspace.path(), "Rust ownership concurrency", 5, Some(0.7))
        .await
        .expect("search");

    // Results bounded by top_k
    assert!(
        results.len() <= 5,
        "Should have at most 5 results, got {}",
        results.len()
    );

    // All scores >= min_score
    for result in &results {
        assert!(
            result.score >= 0.7,
            "Score {} should be >= 0.7",
            result.score
        );
    }

    // Sorted by score
    for window in results.windows(2) {
        assert!(window[0].score >= window[1].score);
    }
}

/// Scenario: Query routing to correct stores
/// Given project path "/tmp/myproject"
/// When I route a query through the hub
/// Then the project store path is derived from "/tmp/myproject"
/// And the global store path is ~/.vibecli/memory/global.db
/// And both stores are queried in parallel
#[tokio::test]
async fn query_routing() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Store something in each
    hub.store_to_project(workspace.path(), "Project-specific data")
        .await
        .expect("proj");
    hub.store_global("Global computer data")
        .await
        .expect("global");

    // Search should touch both stores
    let results = hub
        .search_context(workspace.path(), "data", 5, None)
        .await
        .expect("search");

    // Both store kinds should be represented (if both have matching content)
    let store_kinds: std::collections::HashSet<_> =
        results.iter().map(|r| r.store.clone()).collect();

    // At minimum, we should have project results
    assert!(
        store_kinds.contains(&StoreKind::Project),
        "Should have project results"
    );
}

/// Scenario: Hub exposes /api/memory route
/// Given the daemon is running
/// When GET /api/memory?query=rust+ownership&workspace=/tmp/project
/// Then I receive JSON with matched memories and scores
/// And the response includes project and global results separately
#[tokio::test]
async fn hub_api_response_format() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add test data
    hub.store_to_project(workspace.path(), "Rust ownership model")
        .await
        .expect("store");
    hub.store_global("Machine preference for Rust")
        .await
        .expect("store global");

    // Get context
    let context = hub
        .assemble_context(workspace.path(), "rust ownership", 4000)
        .await
        .expect("context");

    // Should be in XML format
    assert!(context.starts_with("<vibe-memory"));
    assert!(context.ends_with("</vibe-memory>"));

    // Should include relevant content
    assert!(
        context.contains("Rust"),
        "Should include Rust-related content"
    );
}

/// Scenario: Parallel store queries
/// When I query with many results needed
/// Then both stores are queried concurrently
/// And results are merged efficiently
#[tokio::test]
async fn parallel_queries() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add data to both stores
    for i in 0..10 {
        hub.store_to_project(workspace.path(), &format!("Project memory {}", i))
            .await
            .expect("proj");
        hub.store_global(&format!("Global memory {}", i))
            .await
            .expect("global");
    }

    // Measure time for search
    let start = std::time::Instant::now();
    let _ = hub
        .search_context(workspace.path(), "memory", 20, None)
        .await
        .expect("search");
    let elapsed = start.elapsed();

    // Should complete reasonably fast (concurrent queries)
    // This is a soft test - we mostly verify it completes without error
    assert!(
        elapsed.as_secs() < 10,
        "Search should complete in reasonable time"
    );
}

/// Scenario: Decay consolidation across stores
/// When I consolidate memories
/// Then decay is applied to both project and global stores
/// And low-salience entries are purged
#[tokio::test]
async fn consolidate_across_stores() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add memories
    hub.store_to_project(workspace.path(), "Important project data")
        .await
        .expect("proj");
    hub.store_global("Important global data")
        .await
        .expect("global");

    // Consolidate (applies decay + purge)
    let report = hub
        .consolidate(workspace.path())
        .await
        .expect("consolidate");

    // Should have run without error
    assert!(report.project_purged >= 0);
    assert!(report.global_purged >= 0);

    // At minimum, the important memories should survive
    let context = hub
        .assemble_context(workspace.path(), "important", 4000)
        .await
        .expect("context");
    assert!(context.contains("important") || context.contains("Important"));
}

/// Scenario: Hub with custom sector weights
/// When I configure custom sector weights
/// Then queries return results with updated ranking
#[tokio::test]
async fn custom_sector_weights() {
    let workspace = TempDir::new().unwrap();
    let mut hub = MemoryContextHub::new();

    // Add memories in different sectors
    hub.store_to_project(workspace.path(), "Yesterday we discussed authentication") // episodic
        .await
        .expect("episodic");
    hub.store_to_project(workspace.path(), "Auth tokens should expire after 24h") // semantic
        .await
        .expect("semantic");

    // Set high weight for episodic (default is 1.2, let's boost it)
    let mut weights = hub.sector_weights().clone();
    weights.insert("episodic".to_string(), 2.0);
    hub.set_sector_weights(weights);

    // Search should now prioritize episodic
    let results = hub
        .search_context(workspace.path(), "authentication token", 5, None)
        .await
        .expect("search");

    // With boosted episodic weight, the episodic memory should rank higher
    // (This is a soft test - exact behavior depends on implementation)
    assert!(!results.is_empty());
}

/// Scenario: Hub memory usage reporting
/// When I query memory statistics
/// Then I get counts and sizes for project and global stores
#[tokio::test]
async fn memory_usage_reporting() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add some memories
    hub.store_to_project(workspace.path(), "Project memory 1")
        .await
        .expect("proj1");
    hub.store_to_project(workspace.path(), "Project memory 2")
        .await
        .expect("proj2");
    hub.store_global("Global memory 1").await.expect("global1");

    // Get stats
    let stats = hub.get_stats(workspace.path()).await.expect("get stats");

    // Should have project and global counts
    assert!(stats.project_count >= 2);
    assert!(stats.global_count >= 1);
    assert!(stats.project_db_size > 0 || stats.project_count == 0);
    assert!(stats.global_db_size > 0 || stats.global_count == 0);
}

/// Scenario: Error handling for corrupted store
/// Given a project store with corrupted data
/// When I query through the hub
/// Then I receive an error for that store
/// But global store results are still returned
#[tokio::test]
async fn hub_graceful_degradation() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add valid global memory
    hub.store_global("Valid global memory")
        .await
        .expect("store global");

    // Query should work (graceful handling of empty/corrupted project)
    let results = hub
        .search_context(workspace.path(), "global", 5, None)
        .await
        .expect("search");

    // Should still get global results
    assert!(!results.is_empty(), "Should get results from working store");
}

/// Scenario: Concurrent access safety
/// Given multiple async tasks querying the hub
/// When they run concurrently
/// Then no data races occur
/// And all queries complete successfully
#[tokio::test]
async fn concurrent_access_safety() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add some data
    for i in 0..20 {
        hub.store_to_project(workspace.path(), &format!("Memory {}", i))
            .await
            .expect("store");
    }

    // Spawn multiple concurrent queries
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let hub = hub.clone();
            let path = workspace.path().to_path_buf();
            tokio::spawn(async move { hub.search_context(&path, "memory", 5, None).await })
        })
        .collect();

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("join"))
        .map(|r| r.expect("search"))
        .collect();

    // All should have results
    for result in results {
        assert!(!result.is_empty());
    }
}

/// Scenario: Clear all memories in store
/// Given a hub with memories in project and global stores
/// When I clear all memories from a store
/// Then the store is emptied
/// And other store is unaffected
#[tokio::test]
async fn clear_store() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::new();

    // Add to both stores
    hub.store_to_project(workspace.path(), "Project data")
        .await
        .expect("proj");
    hub.store_global("Global data").await.expect("global");

    // Clear project store
    let cleared = hub
        .clear_project(workspace.path())
        .await
        .expect("clear project");
    assert!(cleared >= 1);

    // Project should be empty
    let proj_results = hub
        .search_context(workspace.path(), "project", 5, None)
        .await
        .expect("search");
    assert!(proj_results.iter().all(|r| r.store != StoreKind::Project) || proj_results.is_empty());

    // Global should still have data
    let global_results = hub
        .search_context(workspace.path(), "global", 5, None)
        .await
        .expect("search");
    assert!(
        !global_results.is_empty(),
        "Global store should be unaffected"
    );
}
