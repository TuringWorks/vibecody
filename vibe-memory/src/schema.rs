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
            -- NOT encrypted. The `encryption` Cargo feature (chacha20poly1305)
            -- exists but is off by default and unused: nothing in this crate
            -- calls it, and `store()` writes the caller's text verbatim into
            -- both columns. These comments used to say "encrypted content" /
            -- "encrypted separately", which is a security claim the code does
            -- not honour — treat this DB as plaintext until the feature is
            -- actually implemented and enabled.
            content         BLOB NOT NULL,        -- plaintext content
            content_text    TEXT NOT NULL,        -- plaintext, for search
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
            ttl_expires_at  INTEGER,              -- epoch seconds; NULL = never expires
            
            -- Vector storage (extension-specific)
            embedding       BLOB,                 -- vec/f32 array, plaintext (bincode)
            -- Identity of the model that produced `embedding`, as
            -- `vibe_embed::ModelRef::slug`. Vectors from two models are not
            -- comparable and cosine similarity will not tell you so, which is
            -- why this is stored per row rather than assumed per database.
            -- NULL = written before model tagging existed.
            embedding_model TEXT,
            embedding_dim   INTEGER
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
            
            -- `src_id` is always an entry in *this* store, so the cascade is
            -- correct. `dst_id` deliberately has no foreign key: a
            -- cross-project waypoint points at an entry in a different store
            -- (a different database file), so referential integrity cannot
            -- hold. The FK that used to be here made
            -- `add_waypoint_cross_project` fail with a constraint violation
            -- 100% of the time — the schema forbade exactly what the function
            -- existed to do.
            FOREIGN KEY (src_id) REFERENCES memory_entries(id) ON DELETE CASCADE
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
/// Add columns that `CREATE TABLE IF NOT EXISTS` cannot retrofit onto a
/// database created by an earlier version.
///
/// `ttl_expires_at` is the case that prompted this: the field existed on
/// `MemoryEntry`, `store_with_ttl()` computed a value for it, and
/// `cleanup_expired()` queried it — but the table never had the column. Writes
/// silently dropped the expiry and cleanup failed outright with
/// "no such column: ttl_expires_at", so the TTL feature did nothing at all.
fn migrate_entries_table(conn: &Connection) -> SqliteResult<()> {
    // Columns added after the table shipped. Each is nullable, so an existing
    // row is left alone rather than being back-filled with an invented value:
    // we do not know which model embedded a pre-tagging row, and guessing
    // would be worse than admitting it — see `VectorTag::accepts`.
    const ADDED: &[(&str, &str)] = &[
        ("ttl_expires_at", "INTEGER"),
        // Which embedding model produced `embedding`, as `ModelRef::slug`.
        // NULL means "written before model tagging existed".
        ("embedding_model", "TEXT"),
        // Length of `embedding` as stored. Denormalised so a search can skip
        // incomparable rows without deserialising every blob.
        ("embedding_dim", "INTEGER"),
    ];

    let mut existing = conn.prepare("SELECT 1 FROM pragma_table_info('memory_entries') WHERE name = ?1")?;
    for (name, ty) in ADDED {
        if !existing.exists([name])? {
            conn.execute(
                &format!("ALTER TABLE memory_entries ADD COLUMN {name} {ty}"),
                [],
            )?;
        }
    }
    drop(existing);

    // Cheap filter for the common case — one model, many memories.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entries_embedding_model ON memory_entries(embedding_model)",
        [],
    )?;
    Ok(())
}

/// Rebuild `waypoints` without the `dst_id` foreign key.
///
/// `CREATE TABLE IF NOT EXISTS` cannot alter an existing table, and SQLite has
/// no `DROP CONSTRAINT`, so an already-created store keeps the FK that makes
/// cross-project waypoints impossible. Detect it and rebuild once.
fn migrate_waypoints_table(conn: &Connection) -> SqliteResult<()> {
    let dst_fk_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_foreign_key_list('waypoints') WHERE \"from\" = 'dst_id'",
        [],
        |row| row.get(0),
    )?;
    if dst_fk_count == 0 {
        return Ok(());
    }
    // Foreign keys must be off while swapping the table, or the copy trips the
    // very constraint being removed.
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys=OFF;
        BEGIN;
        CREATE TABLE waypoints_new (
            id          TEXT PRIMARY KEY,
            src_id      TEXT NOT NULL,
            dst_id      TEXT NOT NULL,
            weight      REAL NOT NULL DEFAULT 0.5,
            cross_project INTEGER NOT NULL DEFAULT 0,
            created_at  INTEGER NOT NULL,
            FOREIGN KEY (src_id) REFERENCES memory_entries(id) ON DELETE CASCADE
        );
        INSERT INTO waypoints_new (id, src_id, dst_id, weight, cross_project, created_at)
            SELECT id, src_id, dst_id, weight, cross_project, created_at FROM waypoints;
        DROP TABLE waypoints;
        ALTER TABLE waypoints_new RENAME TO waypoints;
        COMMIT;
        PRAGMA foreign_keys=ON;
        "#,
    )?;
    Ok(())
}

pub fn initialize_store(conn: &Connection) -> SqliteResult<()> {
    create_entries_table(conn)?;
    migrate_entries_table(conn)?;
    create_waypoints_table(conn)?;
    migrate_waypoints_table(conn)?;
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
