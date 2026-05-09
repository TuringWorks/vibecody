//! SQLite schema definitions for vibe-memory stores.
//!
//! Both project and global stores share the same schema, but use different
//! database files with different encryption keys.

use rusqlite::{Connection, Result as SqliteResult};

/// Create the memory entries table with vector storage.
pub fn create_entries_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS memory_entries (
            id              TEXT PRIMARY KEY,
            content         BLOB NOT NULL,        -- encrypted content
            content_text    TEXT NOT NULL,        -- for search (encrypted separately)
            sector          TEXT NOT NULL,         -- episodic|semantic|procedural|emotional|reflective
            salience        REAL NOT NULL DEFAULT 1.0,
            decay_lambda    REAL NOT NULL DEFAULT 0.01,
            created_at      INTEGER NOT NULL,     -- epoch seconds
            updated_at      INTEGER NOT NULL,
            last_seen_at    INTEGER NOT NULL,
            version         INTEGER NOT NULL DEFAULT 1,
            pinned          INTEGER NOT NULL DEFAULT 0,
            tags            TEXT NOT NULL DEFAULT '[]',  -- JSON array
            metadata        TEXT NOT NULL DEFAULT '{}', -- JSON object
            project_id      TEXT,                 -- for global store cross-project tracking
            session_id      TEXT,
            
            -- Vector storage (extension-specific)
            embedding       BLOB                  -- vec/f32 array, encrypted
        );
        "#,
        [],
    )?;

    // Index for sector queries
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_entries_sector 
        ON memory_entries(sector);
        "#,
        [],
    )?;

    // Index for salience-based pruning
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_entries_salience 
        ON memory_entries(salience);
        "#,
        [],
    )?;

    // Index for timestamp-based queries
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_entries_created 
        ON memory_entries(created_at);
        "#,
        [],
    )?;

    // Index for pinning
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_entries_pinned 
        ON memory_entries(pinned) 
        WHERE pinned = 1;
        "#,
        [],
    )?;

    Ok(())
}

/// Create the waypoints table for associative graph.
pub fn create_waypoints_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS waypoints (
            id          TEXT PRIMARY KEY,
            src_id      TEXT NOT NULL,
            dst_id      TEXT NOT NULL,
            weight      REAL NOT NULL DEFAULT 0.5,
            cross_project INTEGER NOT NULL DEFAULT 0,
            created_at  INTEGER NOT NULL,
            
            FOREIGN KEY (src_id) REFERENCES memory_entries(id) ON DELETE CASCADE,
            FOREIGN KEY (dst_id) REFERENCES memory_entries(id) ON DELETE CASCADE
        );
        "#,
        [],
    )?;

    // Index for source lookups
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_waypoints_src 
        ON waypoints(src_id);
        "#,
        [],
    )?;

    // Index for destination lookups
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_waypoints_dst 
        ON waypoints(dst_id);
        "#,
        [],
    )?;

    Ok(())
}

/// Create the meta table for store configuration.
pub fn create_meta_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS store_meta (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  INTEGER NOT NULL
        );
        "#,
        [],
    )?;

    // Insert initial version
    conn.execute(
        r#"
        INSERT OR IGNORE INTO store_meta (key, value, updated_at)
        VALUES ('version', '1', unixepoch('now'));
        "#,
        [],
    )?;

    Ok(())
}

/// Initialize a new memory store with all tables.
pub fn initialize_store(conn: &Connection) -> SqliteResult<()> {
    create_entries_table(conn)?;
    create_waypoints_table(conn)?;
    create_meta_table(conn)?;

    // Enable WAL mode for better concurrent access
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
        "#,
    )?;

    Ok(())
}
