#![allow(dead_code)]
//! Markdown document management with revision history for company orchestration.
//!
//! Documents can be linked to tasks and goals. Every edit creates an
//! append-only revision record, preserving full history.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub company_id: String,
    pub title: String,
    pub content: String,
    pub linked_task_id: Option<String>,
    pub linked_goal_id: Option<String>,
    pub author_agent_id: Option<String>,
    pub revision: i64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRevision {
    pub id: i64,
    pub document_id: String,
    pub revision: i64,
    pub content: String,
    pub author_agent_id: Option<String>,
    pub created_at: u64,
}

// ── DocumentStore ─────────────────────────────────────────────────────────────

pub struct DocumentStore<'a> {
    conn: &'a Connection,
}

impl<'a> DocumentStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS documents (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                title           TEXT NOT NULL,
                content         TEXT NOT NULL DEFAULT '',
                linked_task_id  TEXT,
                linked_goal_id  TEXT,
                author_agent_id TEXT,
                revision        INTEGER NOT NULL DEFAULT 1,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_documents_company ON documents(company_id);

            CREATE TABLE IF NOT EXISTS document_revisions (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id     TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                revision        INTEGER NOT NULL,
                content         TEXT NOT NULL,
                author_agent_id TEXT,
                created_at      INTEGER NOT NULL
            );
        "#)?;
        Ok(())
    }

    pub fn create(
        &self,
        company_id: &str,
        title: &str,
        content: &str,
        author_agent_id: Option<&str>,
        linked_task_id: Option<&str>,
        linked_goal_id: Option<&str>,
    ) -> Result<Document> {
        let doc = Document {
            id: new_id(),
            company_id: company_id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            linked_task_id: linked_task_id.map(|s| s.to_string()),
            linked_goal_id: linked_goal_id.map(|s| s.to_string()),
            author_agent_id: author_agent_id.map(|s| s.to_string()),
            revision: 1,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.conn.execute(
            "INSERT INTO documents (id, company_id, title, content, linked_task_id, linked_goal_id, author_agent_id, revision, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                doc.id, doc.company_id, doc.title, doc.content,
                doc.linked_task_id, doc.linked_goal_id, doc.author_agent_id,
                doc.revision, doc.created_at as i64, doc.updated_at as i64,
            ],
        )?;
        // Record initial revision
        self.conn.execute(
            "INSERT INTO document_revisions (document_id, revision, content, author_agent_id, created_at) VALUES (?1,?2,?3,?4,?5)",
            params![doc.id, 1i64, doc.content, doc.author_agent_id, doc.created_at as i64],
        )?;
        Ok(doc)
    }

    pub fn get(&self, id: &str) -> Result<Option<Document>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, title, content, linked_task_id, linked_goal_id,
                    author_agent_id, revision, created_at, updated_at
             FROM documents WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Document {
                id: row.get(0)?,
                company_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                linked_task_id: row.get(4)?,
                linked_goal_id: row.get(5)?,
                author_agent_id: row.get(6)?,
                revision: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
                updated_at: row.get::<_, i64>(9)? as u64,
            })
        })?;
        rows.next().transpose().map_err(|e| e.into())
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<Document>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, title, content, linked_task_id, linked_goal_id,
                    author_agent_id, revision, created_at, updated_at
             FROM documents WHERE company_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                company_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                linked_task_id: row.get(4)?,
                linked_goal_id: row.get(5)?,
                author_agent_id: row.get(6)?,
                revision: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
                updated_at: row.get::<_, i64>(9)? as u64,
            })
        })?;
        rows.collect::<rusqlite::Result<_>>().map_err(|e| e.into())
    }

    /// Update document content — auto-increments revision and records history.
    pub fn update(
        &self,
        id: &str,
        title: Option<&str>,
        content: Option<&str>,
        author_agent_id: Option<&str>,
    ) -> Result<Document> {
        let doc = self.get(id)?.context("document not found")?;
        let new_revision = doc.revision + 1;
        if let Some(t) = title {
            self.conn.execute(
                "UPDATE documents SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![t, now_ms() as i64, id],
            )?;
        }
        if let Some(c) = content {
            self.conn.execute(
                "UPDATE documents SET content = ?1, revision = ?2, updated_at = ?3 WHERE id = ?4",
                params![c, new_revision, now_ms() as i64, id],
            )?;
            // Append revision record
            self.conn.execute(
                "INSERT INTO document_revisions (document_id, revision, content, author_agent_id, created_at) VALUES (?1,?2,?3,?4,?5)",
                params![id, new_revision, c, author_agent_id, now_ms() as i64],
            )?;
        }
        self.get(id)?.context("document not found after update")
    }

    pub fn list_revisions(&self, document_id: &str) -> Result<Vec<DocumentRevision>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, document_id, revision, content, author_agent_id, created_at
             FROM document_revisions WHERE document_id = ?1 ORDER BY revision DESC",
        )?;
        let rows = stmt.query_map(params![document_id], |row| {
            Ok(DocumentRevision {
                id: row.get(0)?,
                document_id: row.get(1)?,
                revision: row.get(2)?,
                content: row.get(3)?,
                author_agent_id: row.get(4)?,
                created_at: row.get::<_, i64>(5)? as u64,
            })
        })?;
        rows.collect::<rusqlite::Result<_>>().map_err(|e| e.into())
    }
}

// ── Display helpers ───────────────────────────────────────────────────────────

impl Document {
    pub fn summary_line(&self) -> String {
        let link = match (&self.linked_task_id, &self.linked_goal_id) {
            (Some(t), _) => format!("[task:{}]", &t[..8.min(t.len())]),
            (_, Some(g)) => format!("[goal:{}]", &g[..8.min(g.len())]),
            _ => String::new(),
        };
        format!(
            "v{} {}  {}  [{}]",
            self.revision, self.title, link,
            &self.id[..8.min(self.id.len())]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    // ── Create ───────────────────────────────────────────────────────────────

    #[test]
    fn given_new_document_when_created_then_returned_with_revision_one() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Spec", "# Hello", None, None, None).unwrap();
        assert_eq!(doc.title, "Spec");
        assert_eq!(doc.content, "# Hello");
        assert_eq!(doc.revision, 1);
        assert_eq!(doc.company_id, "co1");
    }

    #[test]
    fn given_new_document_when_initial_revision_listed_then_one_revision_present() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Plan", "v1 content", None, None, None).unwrap();
        let revs = store.list_revisions(&doc.id).unwrap();
        assert_eq!(revs.len(), 1);
        assert_eq!(revs[0].revision, 1);
        assert_eq!(revs[0].content, "v1 content");
    }

    #[test]
    fn given_document_with_task_link_when_created_then_linked_task_id_stored() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Task Doc", "", None, Some("task-abc"), None).unwrap();
        assert_eq!(doc.linked_task_id.as_deref(), Some("task-abc"));
    }

    #[test]
    fn given_document_with_goal_link_when_created_then_linked_goal_id_stored() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Goal Doc", "", None, None, Some("goal-xyz")).unwrap();
        assert_eq!(doc.linked_goal_id.as_deref(), Some("goal-xyz"));
    }

    // ── Get ──────────────────────────────────────────────────────────────────

    #[test]
    fn given_created_document_when_get_by_id_then_returns_document() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "RFC-001", "body", None, None, None).unwrap();
        let fetched = store.get(&doc.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "RFC-001");
    }

    #[test]
    fn given_nonexistent_id_when_get_then_returns_none() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.get("no-such-id").unwrap();
        assert!(result.is_none());
    }

    // ── List ─────────────────────────────────────────────────────────────────

    #[test]
    fn given_multiple_documents_when_list_then_returns_all_for_company() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        store.create("co1", "Doc A", "", None, None, None).unwrap();
        store.create("co1", "Doc B", "", None, None, None).unwrap();
        store.create("co2", "Doc C", "", None, None, None).unwrap();
        let list = store.list("co1").unwrap();
        assert_eq!(list.len(), 2);
        // co2 docs are excluded
        let co2_list = store.list("co2").unwrap();
        assert_eq!(co2_list.len(), 1);
    }

    #[test]
    fn given_no_documents_when_list_then_returns_empty() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let list = store.list("co-empty").unwrap();
        assert!(list.is_empty());
    }

    // ── Update ───────────────────────────────────────────────────────────────

    #[test]
    fn given_document_when_content_updated_then_revision_increments() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Living Doc", "v1", None, None, None).unwrap();
        let updated = store.update(&doc.id, None, Some("v2"), Some("agent-1")).unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.content, "v2");
    }

    #[test]
    fn given_document_when_title_updated_then_title_changes_without_revision_bump() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Old Title", "body", None, None, None).unwrap();
        let updated = store.update(&doc.id, Some("New Title"), None, None).unwrap();
        assert_eq!(updated.title, "New Title");
        // title-only update should not bump revision
        assert_eq!(updated.revision, 1);
    }

    #[test]
    fn given_document_when_updated_twice_then_two_additional_revision_records() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Iter Doc", "init", None, None, None).unwrap();
        store.update(&doc.id, None, Some("v2"), None).unwrap();
        store.update(&doc.id, None, Some("v3"), None).unwrap();
        let revs = store.list_revisions(&doc.id).unwrap();
        // initial + 2 updates = 3 revisions, ordered DESC
        assert_eq!(revs.len(), 3);
        assert_eq!(revs[0].revision, 3);
    }

    #[test]
    fn given_nonexistent_doc_when_update_then_error() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.update("ghost-id", Some("Title"), None, None);
        assert!(result.is_err());
    }

    // ── Revisions ────────────────────────────────────────────────────────────

    #[test]
    fn given_revision_record_when_listed_then_author_agent_id_is_stored() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Authored", "body", Some("agent-auth"), None, None).unwrap();
        let revs = store.list_revisions(&doc.id).unwrap();
        assert_eq!(revs[0].author_agent_id.as_deref(), Some("agent-auth"));
    }

    #[test]
    fn given_updated_document_when_revisions_listed_then_newest_first() {
        let conn = make_conn();
        let store = DocumentStore::new(&conn);
        store.ensure_schema().unwrap();
        let doc = store.create("co1", "Order Test", "r1", None, None, None).unwrap();
        store.update(&doc.id, None, Some("r2"), None).unwrap();
        let revs = store.list_revisions(&doc.id).unwrap();
        assert_eq!(revs[0].revision, 2);
        assert_eq!(revs[1].revision, 1);
    }
}
