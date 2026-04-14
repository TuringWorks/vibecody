#![allow(dead_code)]
//! Session tree — in-file JSONL tree branching.
//! Pi-mono gap bridge: Phase A1.
//!
//! Every session entry carries an `id` + optional `parent_id`, forming a
//! directed tree stored in a single JSONL file.  Users can navigate branches,
//! continue from any historical point, fold/unfold subtrees, and create new
//! branches without creating additional files.  Custom entry kinds persist
//! extension state without polluting LLM context.

// ---------------------------------------------------------------------------
// Tiny deterministic ID generator (no external deps)
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id() -> String {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Stable across tests when seeded from seq; good-enough for non-crypto IDs.
    format!("st-{:016x}", seq ^ 0xdead_beef_cafe_0000)
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Opaque entry identifier (UUID-like string).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntryId(pub String);

/// Reference to a parent entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParentId(pub String);

/// The payload of a session entry.
#[derive(Debug, Clone)]
pub enum EntryKind {
    Message { role: String, content: String },
    ToolCall { name: String, args: String, result: String },
    Compaction { summary: String, files_touched: Vec<String> },
    BranchSummary { label: String },
    Custom { type_name: String, payload: String },
}

impl EntryKind {
    fn kind_tag(&self) -> &'static str {
        match self {
            EntryKind::Message { .. } => "message",
            EntryKind::ToolCall { .. } => "tool_call",
            EntryKind::Compaction { .. } => "compaction",
            EntryKind::BranchSummary { .. } => "branch_summary",
            EntryKind::Custom { .. } => "custom",
        }
    }
}

/// A single node in the session tree.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: EntryId,
    pub parent_id: Option<ParentId>,
    pub kind: EntryKind,
    pub timestamp_ms: u64,
    pub label: Option<String>,
}

// ---------------------------------------------------------------------------
// SessionTree
// ---------------------------------------------------------------------------

/// In-memory representation of the tree.  Serialize with [`SessionTree::serialize_jsonl`].
#[derive(Debug, Default)]
pub struct SessionTree {
    entries: Vec<SessionEntry>,
}

impl SessionTree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append a new entry under `parent_id` (or as a root if `None`).
    /// Returns the new entry's id.
    pub fn append(&mut self, parent_id: Option<&str>, kind: EntryKind) -> EntryId {
        let id = EntryId(next_id());
        self.entries.push(SessionEntry {
            id: id.clone(),
            parent_id: parent_id.map(|s| ParentId(s.to_owned())),
            kind,
            timestamp_ms: 0,
            label: None,
        });
        id
    }

    /// Create a new branch starting from an existing entry.
    /// Returns an error if `entry_id` is not found.
    pub fn branch_from(&mut self, entry_id: &str, kind: EntryKind) -> Result<EntryId, String> {
        let exists = self.entries.iter().any(|e| e.id.0 == entry_id);
        if !exists {
            return Err(format!("entry '{}' not found", entry_id));
        }
        Ok(self.append(Some(entry_id), kind))
    }

    /// Return direct children of `entry_id`.  Pass `None` to get root-level entries.
    pub fn children_of(&self, entry_id: Option<&str>) -> Vec<&SessionEntry> {
        self.entries.iter().filter(|e| {
            match entry_id {
                None => e.parent_id.is_none(),
                Some(pid) => e.parent_id.as_ref().map(|p| p.0.as_str()) == Some(pid),
            }
        }).collect()
    }

    /// Return the path from the root to `entry_id`, inclusive (root first).
    pub fn path_to(&self, entry_id: &str) -> Vec<&SessionEntry> {
        // Build an id → entry map for O(n) lookup.
        let mut path: Vec<&SessionEntry> = Vec::new();
        let mut current = entry_id;
        loop {
            let found = self.entries.iter().find(|e| e.id.0 == current);
            match found {
                None => break,
                Some(e) => {
                    path.push(e);
                    match &e.parent_id {
                        None => break,
                        Some(pid) => current = pid.0.as_str(),
                    }
                }
            }
        }
        path.reverse();
        path
    }

    /// Return all leaf entries (entries with no children).
    pub fn leaf_entries(&self) -> Vec<&SessionEntry> {
        self.entries.iter().filter(|e| {
            !self.entries.iter().any(|c| {
                c.parent_id.as_ref().map(|p| p.0.as_str()) == Some(e.id.0.as_str())
            })
        }).collect()
    }

    /// Return the active branch: path from root to the most-recently-added leaf.
    pub fn active_branch(&self) -> Vec<&SessionEntry> {
        match self.entries.last() {
            None => Vec::new(),
            Some(last) => self.path_to(&last.id.0),
        }
    }

    /// Return every entry that is NOT part of the subtree rooted at `entry_id`.
    /// (i.e. entries that would remain if the subtree were folded.)
    pub fn fold_subtree(&self, entry_id: &str) -> Vec<&SessionEntry> {
        let subtree_ids = self.collect_subtree_ids(entry_id);
        self.entries.iter().filter(|e| !subtree_ids.contains(&e.id.0)).collect()
    }

    /// Attach a human-readable label to an entry.  Returns `false` if not found.
    pub fn label_entry(&mut self, entry_id: &str, label: &str) -> bool {
        match self.entries.iter_mut().find(|e| e.id.0 == entry_id) {
            None => false,
            Some(e) => { e.label = Some(label.to_owned()); true }
        }
    }

    /// Total number of entries in the tree.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Number of leaf nodes (= number of distinct branches).
    pub fn branch_count(&self) -> usize {
        self.leaf_entries().len()
    }

    // ------------------------------------------------------------------
    // Serialisation
    // ------------------------------------------------------------------

    /// Serialize the tree to JSONL — one JSON object per line.
    pub fn serialize_jsonl(&self) -> String {
        let mut lines: Vec<String> = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            lines.push(Self::entry_to_json(e));
        }
        lines.join("\n")
    }

    /// Deserialize from JSONL.  Each line must be a valid JSON object
    /// produced by [`Self::serialize_jsonl`].
    pub fn deserialize_jsonl(s: &str) -> Result<Self, String> {
        let mut entries = Vec::new();
        for (lineno, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let entry = Self::entry_from_json(line)
                .map_err(|e| format!("line {}: {}", lineno + 1, e))?;
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Collect ids of `root` and all its descendants.
    fn collect_subtree_ids(&self, root: &str) -> Vec<String> {
        let mut ids = vec![root.to_owned()];
        let mut queue = vec![root.to_owned()];
        while let Some(current) = queue.pop() {
            for child in self.children_of(Some(&current)) {
                ids.push(child.id.0.clone());
                queue.push(child.id.0.clone());
            }
        }
        ids
    }

    fn entry_to_json(e: &SessionEntry) -> String {
        let parent_field = match &e.parent_id {
            None => "null".to_owned(),
            Some(p) => format!("\"{}\"", json_escape(&p.0)),
        };
        let label_field = match &e.label {
            None => "null".to_owned(),
            Some(l) => format!("\"{}\"", json_escape(l)),
        };
        let kind_tag = e.kind.kind_tag();
        let kind_payload = Self::kind_to_json(&e.kind);
        format!(
            "{{\"id\":\"{}\",\"parent_id\":{},\"kind\":\"{}\",\"payload\":{},\"ts\":{},\"label\":{}}}",
            json_escape(&e.id.0),
            parent_field,
            kind_tag,
            kind_payload,
            e.timestamp_ms,
            label_field,
        )
    }

    fn kind_to_json(kind: &EntryKind) -> String {
        match kind {
            EntryKind::Message { role, content } =>
                format!("{{\"role\":\"{}\",\"content\":{}}}", json_escape(role), quoted(content)),
            EntryKind::ToolCall { name, args, result } =>
                format!("{{\"name\":\"{}\",\"args\":{},\"result\":{}}}",
                    json_escape(name), quoted(args), quoted(result)),
            EntryKind::Compaction { summary, files_touched } => {
                let files: Vec<String> = files_touched.iter().map(|f| format!("\"{}\"", json_escape(f))).collect();
                format!("{{\"summary\":{},\"files\":[{}]}}", quoted(summary), files.join(","))
            }
            EntryKind::BranchSummary { label } =>
                format!("{{\"label\":{}}}", quoted(label)),
            EntryKind::Custom { type_name, payload } =>
                format!("{{\"type_name\":\"{}\",\"payload\":{}}}", json_escape(type_name), quoted(payload)),
        }
    }

    fn entry_from_json(s: &str) -> Result<SessionEntry, String> {
        // Minimal hand-rolled parser — avoids pulling in serde for this module.
        let id = extract_str(s, "\"id\":")
            .ok_or("missing id")?;
        let parent_id = extract_nullable_str(s, "\"parent_id\":");
        let kind_tag = extract_str(s, "\"kind\":")
            .ok_or("missing kind")?;
        let ts = extract_u64(s, "\"ts\":")
            .unwrap_or(0);
        let label = extract_nullable_str(s, "\"label\":");

        // Locate the payload object for kind-specific fields.
        let payload_str = extract_object(s, "\"payload\":")
            .unwrap_or_default();

        let kind = match kind_tag.as_str() {
            "message" => {
                let role = extract_str(&payload_str, "\"role\":").unwrap_or_default();
                let content = extract_str(&payload_str, "\"content\":").unwrap_or_default();
                EntryKind::Message { role, content }
            }
            "tool_call" => {
                let name = extract_str(&payload_str, "\"name\":").unwrap_or_default();
                let args = extract_str(&payload_str, "\"args\":").unwrap_or_default();
                let result = extract_str(&payload_str, "\"result\":").unwrap_or_default();
                EntryKind::ToolCall { name, args, result }
            }
            "compaction" => {
                let summary = extract_str(&payload_str, "\"summary\":").unwrap_or_default();
                let files_touched = extract_str_array(&payload_str, "\"files\":");
                EntryKind::Compaction { summary, files_touched }
            }
            "branch_summary" => {
                let label = extract_str(&payload_str, "\"label\":").unwrap_or_default();
                EntryKind::BranchSummary { label }
            }
            "custom" | _ => {
                let type_name = extract_str(&payload_str, "\"type_name\":").unwrap_or_else(|| kind_tag.clone());
                let payload = extract_str(&payload_str, "\"payload\":").unwrap_or_default();
                EntryKind::Custom { type_name, payload }
            }
        };

        Ok(SessionEntry {
            id: EntryId(id),
            parent_id: parent_id.map(ParentId),
            kind,
            timestamp_ms: ts,
            label,
        })
    }
}

// ---------------------------------------------------------------------------
// Minimal JSON helpers (no serde dependency)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

/// Extract the value of a JSON string field by key prefix, e.g. `"id":`.
fn extract_str(s: &str, key: &str) -> Option<String> {
    let start = s.find(key)?;
    let after_key = &s[start + key.len()..];
    let after_key = after_key.trim_start();
    if after_key.starts_with("null") { return None; }
    if !after_key.starts_with('"') { return None; }
    let inner = &after_key[1..];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(result),
            '\\' => match chars.next()? {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                other => result.push(other),
            },
            other => result.push(other),
        }
    }
    None
}

fn extract_nullable_str(s: &str, key: &str) -> Option<String> {
    let start = s.find(key)?;
    let after_key = s[start + key.len()..].trim_start();
    if after_key.starts_with("null") { return None; }
    extract_str(s, key)
}

fn extract_u64(s: &str, key: &str) -> Option<u64> {
    let start = s.find(key)?;
    let after_key = s[start + key.len()..].trim_start();
    let end = after_key.find(|c: char| !c.is_ascii_digit()).unwrap_or(after_key.len());
    after_key[..end].parse().ok()
}

/// Extract the raw JSON object value for a key, e.g. `"payload":{...}`.
fn extract_object(s: &str, key: &str) -> Option<String> {
    let start = s.find(key)?;
    let after_key = s[start + key.len()..].trim_start();
    if !after_key.starts_with('{') { return None; }
    let mut depth = 0usize;
    let mut end = 0;
    for (i, c) in after_key.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { end = i + 1; break; }
            }
            _ => {}
        }
    }
    Some(after_key[..end].to_owned())
}

/// Extract a JSON array of strings, e.g. `"files":["a","b"]`.
fn extract_str_array(s: &str, key: &str) -> Vec<String> {
    let start = match s.find(key) { Some(i) => i, None => return Vec::new() };
    let after_key = s[start + key.len()..].trim_start();
    if !after_key.starts_with('[') { return Vec::new(); }
    let end = match after_key.find(']') { Some(i) => i, None => return Vec::new() };
    let inner = &after_key[1..end];
    // Split naively by `","` — sufficient for file paths without embedded quotes.
    inner.split(',').filter_map(|tok| {
        let tok = tok.trim();
        if tok.starts_with('"') && tok.ends_with('"') && tok.len() >= 2 {
            Some(tok[1..tok.len()-1].to_owned())
        } else { None }
    }).collect()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> EntryKind {
        EntryKind::Message { role: role.to_owned(), content: content.to_owned() }
    }

    #[test]
    fn test_append_linear() {
        let mut tree = SessionTree::new();
        let id1 = tree.append(None, msg("user", "hello"));
        let id2 = tree.append(Some(&id1.0), msg("assistant", "hi"));
        assert_eq!(tree.entry_count(), 2);
        let kids = tree.children_of(Some(&id1.0));
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].id, id2);
    }

    #[test]
    fn test_branch_from() {
        let mut tree = SessionTree::new();
        let root = tree.append(None, msg("user", "start"));
        let a = tree.append(Some(&root.0), msg("assistant", "branch A"));
        let b = tree.branch_from(&root.0, msg("assistant", "branch B")).unwrap();
        assert_ne!(a, b);
        assert_eq!(tree.children_of(Some(&root.0)).len(), 2);
        assert_eq!(tree.branch_count(), 2);
    }

    #[test]
    fn test_branch_from_nonexistent_fails() {
        let mut tree = SessionTree::new();
        assert!(tree.branch_from("ghost-id", msg("user", "x")).is_err());
    }

    #[test]
    fn test_path_to() {
        let mut tree = SessionTree::new();
        let r = tree.append(None, msg("user", "root"));
        let m = tree.append(Some(&r.0), msg("assistant", "mid"));
        let l = tree.append(Some(&m.0), msg("user", "leaf"));
        let path = tree.path_to(&l.0);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, r);
        assert_eq!(path[1].id, m);
        assert_eq!(path[2].id, l);
    }

    #[test]
    fn test_path_to_unknown_returns_empty() {
        let tree = SessionTree::new();
        assert!(tree.path_to("nope").is_empty());
    }

    #[test]
    fn test_leaf_entries() {
        let mut tree = SessionTree::new();
        let r = tree.append(None, msg("user", "root"));
        let _ = tree.append(Some(&r.0), msg("assistant", "leaf-a"));
        let _ = tree.append(Some(&r.0), msg("assistant", "leaf-b"));
        let leaves = tree.leaf_entries();
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn test_active_branch_is_last_appended() {
        let mut tree = SessionTree::new();
        let r = tree.append(None, msg("user", "r"));
        let m = tree.append(Some(&r.0), msg("assistant", "m"));
        let l = tree.append(Some(&m.0), msg("user", "l"));
        let branch = tree.active_branch();
        assert_eq!(branch.last().unwrap().id, l);
    }

    #[test]
    fn test_fold_subtree() {
        let mut tree = SessionTree::new();
        let r = tree.append(None, msg("user", "r"));
        let a = tree.append(Some(&r.0), msg("assistant", "a"));
        let _ = tree.append(Some(&a.0), msg("user", "a-child"));
        let b = tree.append(Some(&r.0), msg("assistant", "b"));

        // Fold the subtree rooted at `a` — should keep `r` and `b`.
        let visible = tree.fold_subtree(&a.0);
        let ids: Vec<&str> = visible.iter().map(|e| e.id.0.as_str()).collect();
        assert!(ids.contains(&r.0.as_str()));
        assert!(ids.contains(&b.0.as_str()));
        assert!(!ids.contains(&a.0.as_str()));
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_label_entry() {
        let mut tree = SessionTree::new();
        let id = tree.append(None, msg("user", "hi"));
        assert!(tree.label_entry(&id.0, "checkpoint-1"));
        let entry = tree.entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(entry.label.as_deref(), Some("checkpoint-1"));
        assert!(!tree.label_entry("nonexistent", "x"));
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut tree = SessionTree::new();
        let r = tree.append(None, msg("user", "hello\nworld"));
        let a = tree.append(Some(&r.0), msg("assistant", "reply"));
        let _ = tree.append(Some(&a.0), EntryKind::Compaction {
            summary: "compacted 10 msgs".to_owned(),
            files_touched: vec!["src/main.rs".to_owned(), "Cargo.toml".to_owned()],
        });
        let _ = tree.append(Some(&r.0), EntryKind::Custom {
            type_name: "plugin_state".to_owned(),
            payload: "{\"active\":true}".to_owned(),
        });
        tree.label_entry(&r.0, "start");

        let jsonl = tree.serialize_jsonl();
        let restored = SessionTree::deserialize_jsonl(&jsonl).expect("deserialize ok");
        assert_eq!(restored.entry_count(), 4);

        // Check root label survived.
        let root = restored.entries.iter().find(|e| e.id == r).unwrap();
        assert_eq!(root.label.as_deref(), Some("start"));

        // Check content with newlines survived.
        if let EntryKind::Message { content, .. } = &root.kind {
            assert_eq!(content, "hello\nworld");
        } else { panic!("wrong kind"); }

        // Check compaction files array survived.
        let compact = restored.entries.iter().find(|e| {
            matches!(&e.kind, EntryKind::Compaction { .. })
        }).unwrap();
        if let EntryKind::Compaction { files_touched, .. } = &compact.kind {
            assert_eq!(files_touched.len(), 2);
        } else { panic!("wrong kind"); }
    }

    #[test]
    fn test_tool_call_roundtrip() {
        let mut tree = SessionTree::new();
        let _ = tree.append(None, EntryKind::ToolCall {
            name: "read_file".to_owned(),
            args: "{\"path\":\"src/lib.rs\"}".to_owned(),
            result: "pub mod foo;".to_owned(),
        });
        let jsonl = tree.serialize_jsonl();
        let rt = SessionTree::deserialize_jsonl(&jsonl).unwrap();
        if let EntryKind::ToolCall { name, .. } = &rt.entries[0].kind {
            assert_eq!(name, "read_file");
        } else { panic!("wrong kind"); }
    }

    #[test]
    fn test_branch_summary_roundtrip() {
        let mut tree = SessionTree::new();
        let _ = tree.append(None, EntryKind::BranchSummary { label: "experiment-v2".to_owned() });
        let rt = SessionTree::deserialize_jsonl(&tree.serialize_jsonl()).unwrap();
        if let EntryKind::BranchSummary { label } = &rt.entries[0].kind {
            assert_eq!(label, "experiment-v2");
        } else { panic!("wrong kind"); }
    }
}
