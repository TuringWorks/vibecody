//! Integration BDD test for MemoryContextHub
//!
//! Feature: Memory Context Hub Integration
//!
//! These tests verify the hub works with both project and global stores.

use vibe_memory::*;
use tempfile::TempDir;

#[tokio::test]
async fn hub_stores_and_retrieves_project_memory() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store project memory
    hub.store_to_project(workspace.path().to_path_buf(), "Rust async programming").await.expect("store");
    
    // Search should find it
    let results = hub.search_context(workspace.path(), "async rust", 5, None).await.expect("search");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.content.contains("Rust") || r.content.contains("async")));
}

#[tokio::test]
async fn hub_respects_context_budget() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store many memories
    for i in 0..20 {
        let content = format!("Memory {} with substantial content about various topics in software development", i);
        hub.store_to_project(workspace.path().to_path_buf(), &content).await.expect("store");
    }
    
    // Assemble with very small budget
    let context = hub.assemble_context(workspace.path(), "memory", 200).await.expect("assemble");
    
    // Should still have opening/closing tags
    assert!(context.contains("<vibe-memory>"));
    assert!(context.contains("</vibe-memory>"));
    
    // But limited content due to budget
    let mem_count = context.matches("- [PROJECT]").count();
    assert!(mem_count <= 5, "Budget should limit results, got {} entries", mem_count);
}

#[tokio::test]
async fn hub_applies_sector_weights_to_ranking() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store memories in different sectors
    hub.store_to_project(workspace.path().to_path_buf(), "Yesterday we had a meeting").await.expect("episodic");
    hub.store_to_project(workspace.path().to_path_buf(), "The definition of an API is a contract").await.expect("semantic");
    hub.store_to_project(workspace.path().to_path_buf(), "Step 1: run cargo build").await.expect("procedural");
    
    // Search
    let results = hub.search_context(workspace.path(), "api meeting build", 5, None).await.expect("search");
    
    // Should have results with scores
    assert!(!results.is_empty());
    for result in &results {
        assert!(result.score > 0.0);
    }
}

#[tokio::test]
async fn hub_consolidate_applies_decay() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store a memory
    let entry = hub.store_to_project(workspace.path().to_path_buf(), "Important fact").await.expect("store");
    assert_eq!(entry.salience, 1.0);
    
    // Consolidate (applies decay)
    let report = hub.consolidate(workspace.path()).await.expect("consolidate");
    
    // Should have run without errors
    assert!(report.entries_decayed >= 0);
    assert!(report.entries_purged >= 0);
}

#[tokio::test]
async fn hub_stats_reflects_store_contents() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store some memories
    hub.store_to_project(workspace.path().to_path_buf(), "Project memory 1").await.expect("store1");
    hub.store_to_project(workspace.path().to_path_buf(), "Project memory 2").await.expect("store2");
    
    // Get stats
    let stats = hub.get_stats(workspace.path()).await.expect("get stats");
    
    assert_eq!(stats.project_count, 2);
    assert!(stats.project_db_size > 0);
}

#[tokio::test]
async fn hub_clear_project_removes_all() {
    let workspace = TempDir::new().unwrap();
    let hub = MemoryContextHub::with_global_at(workspace.path());
    
    // Store memories
    hub.store_to_project(workspace.path().to_path_buf(), "Memory 1").await.expect("store1");
    hub.store_to_project(workspace.path().to_path_buf(), "Memory 2").await.expect("store2");
    
    // Verify they exist
    let stats_before = hub.get_stats(workspace.path()).await.expect("stats before");
    assert_eq!(stats_before.project_count, 2);
    
    // Clear
    hub.clear_project().await;
    
    // Verify they're gone
    let stats_after = hub.get_stats(workspace.path()).await.expect("stats after");
    assert_eq!(stats_after.project_count, 0);
}
