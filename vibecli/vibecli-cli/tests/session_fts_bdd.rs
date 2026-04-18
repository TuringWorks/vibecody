/*!
 * BDD tests for session FTS5 search (Phase 1 of the memory-as-infrastructure
 * redesign). Exercises project-scoped recall, all-scope recall, cleanup on
 * session delete, and snippet highlighting.
 *
 * Run with: cargo test --test session_fts_bdd
 */
use cucumber::{World, given, then, when};
use tempfile::NamedTempFile;
use vibecli_cli::session_store::{FtsHit, SearchScope, SessionStore};

#[derive(Default, World)]
pub struct FtsWorld {
    store: Option<SessionStore>,
    hits: Vec<FtsHit>,
}

impl std::fmt::Debug for FtsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FtsWorld")
            .field("store", &self.store.as_ref().map(|_| "<SessionStore>"))
            .field("hits", &self.hits)
            .finish()
    }
}

impl FtsWorld {
    fn store(&mut self) -> &SessionStore {
        if self.store.is_none() {
            let f = NamedTempFile::new().expect("tempfile");
            let path = f.path().to_owned();
            std::mem::forget(f);
            self.store = Some(SessionStore::open(path).expect("open store"));
        }
        self.store.as_ref().unwrap()
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a session "([^"]+)" in project "([^"]+)" with a user message "([^"]+)"$"#)]
fn given_session_with_user_message(w: &mut FtsWorld, id: String, project: String, msg: String) {
    let store = w.store();
    store
        .insert_session_with_project(&id, "task", "claude", "claude-3", &project)
        .expect("insert session");
    store.insert_message(&id, "user", &msg).expect("insert msg");
}

#[given(
    regex = r#"^a session "([^"]+)" in project "([^"]+)" with an assistant message "([^"]+)"$"#
)]
fn given_session_with_assistant_message(
    w: &mut FtsWorld,
    id: String,
    project: String,
    msg: String,
) {
    let store = w.store();
    store
        .insert_session_with_project(&id, "task", "claude", "claude-3", &project)
        .expect("insert session");
    store
        .insert_message(&id, "assistant", &msg)
        .expect("insert msg");
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the agent searches for "([^"]+)" with no scope$"#)]
fn when_search_all(w: &mut FtsWorld, query: String) {
    let hits = w
        .store()
        .search_fts(&query, SearchScope::All, 50)
        .expect("search");
    w.hits = hits;
}

#[when(regex = r#"^the agent searches for "([^"]+)" scoped to project "([^"]+)"$"#)]
fn when_search_scoped(w: &mut FtsWorld, query: String, project: String) {
    let hits = w
        .store()
        .search_fts(&query, SearchScope::Project(project), 50)
        .expect("search");
    w.hits = hits;
}

#[when(regex = r#"^the session "([^"]+)" is deleted$"#)]
fn when_session_deleted(w: &mut FtsWorld, id: String) {
    w.store().delete_session(&id).expect("delete");
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(expr = "exactly {int} hit is returned")]
fn then_exactly_n_hit_singular(w: &mut FtsWorld, n: usize) {
    assert_eq!(w.hits.len(), n, "hits = {:?}", w.hits);
}

#[then(expr = "exactly {int} hits are returned")]
fn then_exactly_n_hits(w: &mut FtsWorld, n: usize) {
    assert_eq!(w.hits.len(), n, "hits = {:?}", w.hits);
}

#[then(expr = "the top hit belongs to session {string}")]
fn then_top_hit_session(w: &mut FtsWorld, id: String) {
    let top = w.hits.first().expect("no hits");
    assert_eq!(top.session_id, id, "top hit = {:?}", top);
}

#[then(expr = "the top hit snippet contains {string}")]
fn then_top_hit_snippet_contains(w: &mut FtsWorld, needle: String) {
    let top = w.hits.first().expect("no hits");
    assert!(
        top.snippet.contains(&needle),
        "snippet {:?} did not contain {:?}",
        top.snippet,
        needle
    );
}

fn main() {
    futures::executor::block_on(FtsWorld::run("tests/features/session_fts.feature"));
}
