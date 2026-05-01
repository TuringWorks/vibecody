//! Recap & Resume — Phase F1.3 resume surface.
//!
//! `POST /v1/resume` accepts either an existing recap id or a raw
//! (kind, subject_id) pair, plus optional overrides, and returns a
//! handle the client can poll via `GET /v1/resume/:handle`.
//!
//! F1.3 scope (per docs/design/recap-resume/01-session.md "RPC contract"):
//! - kind = "session" only; "job" / "diff_chain" return 400.
//! - branch=true forks via `SessionStore::fork_session` (returns new
//!   session_id); branch=false returns the source session_id.
//! - primed_message_count = messages up to `from_message` (inclusive)
//!   when set, else the full transcript count.
//! - ready becomes true immediately on successful resume — F1.3 has
//!   no async warm-up; future slices may add provider warmup if it
//!   becomes a real wait.
//!
//! Storage: handles live in an in-memory `ResumeRegistry` keyed by
//! handle id. Daemon restarts forget handles — the contract is
//! short-lived (clients re-resume on reconnect) which keeps the slice
//! tractable. Persisting to a `resumes` table is a follow-on if the
//! UX needs durable handles across restarts.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::session_store::SessionStore;

// ── Wire shapes ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ResumeRequest {
    /// Resume from a stored recap. When set, the recap's resume_hint
    /// is the default; per-field overrides below take precedence.
    #[serde(default)]
    pub from_recap_id: Option<String>,
    /// Alternative entry: resume directly from a subject. Requires
    /// `kind` to disambiguate the store. Mutually exclusive with
    /// `from_recap_id` — if both are present, `from_recap_id` wins.
    #[serde(default)]
    pub from_subject_id: Option<String>,
    /// Required when `from_subject_id` is set; ignored otherwise (the
    /// recap row carries its own kind). F1.3 supports "session" only.
    #[serde(default)]
    pub kind: Option<String>,
    /// Override `recap.resume_hint.from_message` (cursor into the
    /// transcript). `None` resumes from the tail.
    #[serde(default)]
    pub from_message: Option<i64>,
    /// Override `recap.resume_hint.seed_instruction` (pre-fills the
    /// next prompt input on the client).
    #[serde(default)]
    pub seed_instruction: Option<String>,
    /// Override `recap.resume_hint.branch_on_resume`. `None` falls
    /// back to the recap's hint, then to `false` (tail-resume).
    #[serde(default)]
    pub branch: Option<bool>,
    /// Telemetry/activity tracking — which client initiated the
    /// resume. Stored in the registry but never mandatory.
    #[serde(default)]
    pub client: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeResponse {
    pub handle: String,
    /// Equal to the source `subject_id` when `branched = false`,
    /// the new fork id when `branched = true`.
    pub resumed_session_id: String,
    pub primed_message_count: i64,
    pub ready: bool,
    /// Echoed back so clients that pipelined a `GET /v1/resume/:handle`
    /// can correlate without storing the request locally.
    pub branched: bool,
    /// The cursor used (after applying overrides). `None` = tail.
    pub from_message: Option<i64>,
    /// The seed instruction the client should pre-fill, if any.
    pub seed_instruction: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Internal registry record; identical to the wire response plus
/// provenance (client label, source recap id) so future audit /
/// telemetry surfaces can read it.
#[derive(Debug, Clone)]
pub struct ResumeRecord {
    pub response: ResumeResponse,
    pub source_recap_id: Option<String>,
    pub source_subject_id: String,
    pub client: Option<String>,
}

// ── Registry ────────────────────────────────────────────────────────────────

/// In-memory map handle → ResumeRecord. Tests construct one directly;
/// the HTTP handlers reach into the global via `global_registry()`.
#[derive(Debug, Default)]
pub struct ResumeRegistry {
    inner: Mutex<HashMap<String, ResumeRecord>>,
}

impl ResumeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, record: ResumeRecord) {
        let handle = record.response.handle.clone();
        self.inner.lock().unwrap().insert(handle, record);
    }

    pub fn get(&self, handle: &str) -> Option<ResumeRecord> {
        self.inner.lock().unwrap().get(handle).cloned()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().is_empty()
    }
}

/// Process-wide registry used by the HTTP handlers. Tests skip this
/// and pass an explicit `&ResumeRegistry` to the pure helpers.
pub fn global_registry() -> &'static ResumeRegistry {
    static REGISTRY: OnceLock<ResumeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ResumeRegistry::new)
}

// ── Pure helpers — drive the routes ────────────────────────────────────────

/// HTTP-shaped result so handlers can lift it into `(StatusCode, Json(_))`
/// without re-implementing the error mapping. Status is a u16 to keep
/// this module independent of axum.
pub struct HelperOutcome {
    pub status: u16,
    pub body: serde_json::Value,
}

const HTTP_OK: u16 = 200;
const HTTP_BAD_REQUEST: u16 = 400;
const HTTP_NOT_FOUND: u16 = 404;
const HTTP_INTERNAL: u16 = 500;

/// `POST /v1/resume` — generate a resume handle.
pub fn do_v1_resume_post(
    store: &SessionStore,
    registry: &ResumeRegistry,
    req: &ResumeRequest,
) -> HelperOutcome {
    // Resolve the source: prefer recap id when both are supplied,
    // since the recap's resume_hint is the canonical default for the
    // overrides below.
    let (source_subject_id, recap_hint, source_recap_id) =
        match (&req.from_recap_id, &req.from_subject_id, &req.kind) {
            (Some(recap_id), _, _) => {
                let recap = match store.get_recap_by_id(recap_id) {
                    Ok(Some(r)) => r,
                    Ok(None) => {
                        return err(
                            HTTP_NOT_FOUND,
                            format!("recap {recap_id:?} not found"),
                        );
                    }
                    Err(e) => return err(HTTP_INTERNAL, format!("recap load: {e}")),
                };
                if !matches!(recap.kind, crate::recap::RecapKind::Session) {
                    return err(
                        HTTP_BAD_REQUEST,
                        format!(
                            "recap kind {:?} not supported in F1.3; only \"session\" is implemented",
                            recap.kind
                        ),
                    );
                }
                (
                    recap.subject_id.clone(),
                    recap.resume_hint.clone(),
                    Some(recap_id.clone()),
                )
            }
            (None, Some(subject_id), kind_opt) => {
                let kind = match kind_opt.as_deref() {
                    Some(k) => k,
                    None => {
                        return err(
                            HTTP_BAD_REQUEST,
                            "kind is required when from_recap_id is omitted".to_string(),
                        );
                    }
                };
                if kind != "session" {
                    return err(
                        HTTP_BAD_REQUEST,
                        format!(
                            "kind {kind:?} not supported in F1.3; only \"session\" is implemented"
                        ),
                    );
                }
                (subject_id.clone(), None, None)
            }
            (None, None, _) => {
                return err(
                    HTTP_BAD_REQUEST,
                    "either from_recap_id or from_subject_id is required".to_string(),
                );
            }
        };

    // Verify the source session exists. This 404s cleanly when a
    // recap row references a session that's been pruned, instead of
    // silently fabricating a handle.
    let source_detail = match store.get_session_detail(&source_subject_id) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return err(
                HTTP_NOT_FOUND,
                format!("session {source_subject_id:?} not found"),
            );
        }
        Err(e) => return err(HTTP_INTERNAL, format!("session load: {e}")),
    };

    // Apply overrides: explicit request fields beat recap.resume_hint
    // beat F1.3 defaults (None / false).
    let from_message = req
        .from_message
        .or_else(|| recap_hint.as_ref().and_then(|h| h.from_message));
    let seed_instruction = req
        .seed_instruction
        .clone()
        .or_else(|| recap_hint.as_ref().and_then(|h| h.seed_instruction.clone()));
    let branched = req
        .branch
        .or_else(|| recap_hint.as_ref().map(|h| h.branch_on_resume))
        .unwrap_or(false);

    // Compute primed_message_count before any fork so the count
    // reflects what was on the wire when the user requested the
    // resume — not what's now in the (possibly mutated) fork.
    let primed = match from_message {
        Some(cursor) => source_detail
            .messages
            .iter()
            .filter(|m| m.id <= cursor)
            .count() as i64,
        None => source_detail.messages.len() as i64,
    };

    // Branch: fork the source into a new session id. Without
    // forking, the resume edits the original session in place
    // (matches the design's branch_on_resume = false default for
    // tail resumes).
    let resumed_session_id = if branched {
        let new_id = format!(
            "fork-{}-{}",
            &source_subject_id[..source_subject_id.len().min(12)],
            uuid::Uuid::new_v4().simple(),
        );
        if let Err(e) = store.fork_session(&source_subject_id, &new_id) {
            return err(HTTP_INTERNAL, format!("fork failed: {e}"));
        }
        new_id
    } else {
        source_subject_id.clone()
    };

    let response = ResumeResponse {
        handle: new_handle_id(),
        resumed_session_id,
        primed_message_count: primed,
        ready: true, // F1.3: no async warm-up, ready immediately.
        branched,
        from_message,
        seed_instruction,
        created_at: Utc::now(),
    };
    let record = ResumeRecord {
        response: response.clone(),
        source_recap_id,
        source_subject_id,
        client: req.client.clone(),
    };
    registry.insert(record);

    match serde_json::to_value(&response) {
        Ok(v) => HelperOutcome {
            status: HTTP_OK,
            body: v,
        },
        Err(e) => err(HTTP_INTERNAL, format!("serialize: {e}")),
    }
}

/// `GET /v1/resume/:handle` — return the registered record, or 404.
pub fn do_v1_resume_get(
    registry: &ResumeRegistry,
    handle: &str,
) -> HelperOutcome {
    match registry.get(handle) {
        Some(record) => match serde_json::to_value(&record.response) {
            Ok(v) => HelperOutcome {
                status: HTTP_OK,
                body: v,
            },
            Err(e) => err(HTTP_INTERNAL, format!("serialize: {e}")),
        },
        None => err(HTTP_NOT_FOUND, "resume handle not found".to_string()),
    }
}

// ── J1.3b: kind=job resume helper ──────────────────────────────────────────
//
// `POST /v1/resume` with kind=job spawns a *new* job whose lineage points at
// the parent via `parent_job_id` + `resumed_from_recap_id` (J1.1's columns).
// The parent job must be in a terminal status — running jobs return 409.
//
// Async because `JobManager` lives behind a tokio `Mutex`. The session-side
// `do_v1_resume_post` stays sync and routes here when it sees a job recap;
// the HTTP handler also reaches in directly when the request carries
// `kind=job` without a recap id.

/// Resolve the source for a job-kind resume. Returns the source job_id +
/// the recap's resume_hint (if a recap was supplied) + the source recap id.
async fn resolve_job_resume_source(
    jm: &crate::job_manager::JobManager,
    req: &ResumeRequest,
) -> Result<
    (
        String,
        Option<crate::recap::ResumeHint>,
        Option<String>,
    ),
    HelperOutcome,
> {
    if let Some(recap_id) = &req.from_recap_id {
        let recap = match jm.get_job_recap_by_id(recap_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err(err(
                    HTTP_NOT_FOUND,
                    format!("recap {recap_id:?} not found"),
                ));
            }
            Err(e) => return Err(err(HTTP_INTERNAL, format!("recap load: {e}"))),
        };
        if !matches!(recap.kind, crate::recap::RecapKind::Job) {
            return Err(err(
                HTTP_BAD_REQUEST,
                format!(
                    "recap kind {:?} mismatched; expected \"job\"",
                    recap.kind
                ),
            ));
        }
        return Ok((
            recap.subject_id.clone(),
            recap.resume_hint.clone(),
            Some(recap_id.clone()),
        ));
    }
    if let Some(subject_id) = &req.from_subject_id {
        return Ok((subject_id.clone(), None, None));
    }
    Err(err(
        HTTP_BAD_REQUEST,
        "either from_recap_id or from_subject_id is required".to_string(),
    ))
}

/// `POST /v1/resume` (kind=job) — spawns a new job linked to the parent.
pub async fn do_v1_resume_post_job(
    jm: &crate::job_manager::JobManager,
    registry: &ResumeRegistry,
    req: &ResumeRequest,
) -> HelperOutcome {
    let (parent_job_id, recap_hint, source_recap_id) =
        match resolve_job_resume_source(jm, req).await {
            Ok(t) => t,
            Err(e) => return e,
        };

    // Parent job must exist + be terminal — design `02-job.md` says
    // running jobs return 409 to keep "Resume" idempotent.
    let parent = match jm.get(&parent_job_id).await {
        Some(j) => j,
        None => {
            return err(
                HTTP_NOT_FOUND,
                format!("job {parent_job_id:?} not found"),
            );
        }
    };
    let parent_status = match crate::job_manager::JobStatus::parse(&parent.status) {
        Some(s) => s,
        None => {
            return err(
                HTTP_INTERNAL,
                format!(
                    "parent job has unknown status {:?}",
                    parent.status
                ),
            );
        }
    };
    if !parent_status.is_terminal() {
        return HelperOutcome {
            status: 409,
            body: serde_json::json!({
                "error": format!(
                    "parent job {parent_job_id:?} is in non-terminal status {:?}",
                    parent.status
                )
            }),
        };
    }

    let seed_instruction = req
        .seed_instruction
        .clone()
        .or_else(|| recap_hint.as_ref().and_then(|h| h.seed_instruction.clone()));
    let task = match seed_instruction.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => format!("Resume of {}", parent.task),
    };

    let (workspace_root, approval) = match jm
        .get_workspace_and_approval(&parent_job_id)
        .await
    {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            return err(
                HTTP_INTERNAL,
                format!("parent job {parent_job_id:?} row vanished"),
            );
        }
        Err(e) => return err(HTTP_INTERNAL, format!("parent ctx load: {e}")),
    };

    let new_req = crate::job_manager::CreateJobReq {
        task: task.clone(),
        provider: parent.provider.clone(),
        approval,
        workspace_root,
        priority: parent.priority,
        webhook_url: parent.webhook_url.clone(),
        tags: parent.tags.clone(),
        quota_bucket: None,
    };
    let new_id = match jm.create(new_req).await {
        Ok(id) => id,
        Err(e) => return err(HTTP_INTERNAL, format!("create resumed job: {e}")),
    };

    // Best-effort: link the new job to its parent + the source recap.
    // A failure here doesn't abort the resume — the new job already
    // exists and the user can re-link manually if needed.
    let recap_id_for_link = source_recap_id.clone().unwrap_or_default();
    if let Err(e) = jm
        .set_parent_link(&new_id, &parent_job_id, &recap_id_for_link)
        .await
    {
        eprintln!(
            "[resume] set_parent_link failed for {new_id}: {e}"
        );
    }

    let response = ResumeResponse {
        handle: new_handle_id(),
        resumed_session_id: new_id,
        // Jobs don't have a "primed message count"; events stream live.
        // Echo 0 so the field stays present for cross-kind clients.
        primed_message_count: 0,
        // Jobs are queued-on-create — `ready=true` mirrors the F1.3
        // session-tail-resume semantics: the daemon has accepted the
        // intent. Polling status on the new job_id reveals running.
        ready: true,
        branched: true, // every job-resume produces a fresh job_id
        from_message: None,
        seed_instruction,
        created_at: Utc::now(),
    };
    let record = ResumeRecord {
        response: response.clone(),
        source_recap_id,
        source_subject_id: parent_job_id,
        client: req.client.clone(),
    };
    registry.insert(record);

    match serde_json::to_value(&response) {
        Ok(v) => HelperOutcome {
            status: HTTP_OK,
            body: v,
        },
        Err(e) => err(HTTP_INTERNAL, format!("serialize: {e}")),
    }
}

fn err(status: u16, message: String) -> HelperOutcome {
    HelperOutcome {
        status,
        body: serde_json::json!({"error": message}),
    }
}

fn new_handle_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recap::heuristic_recap;
    use crate::session_store::SessionStore;

    fn fixture() -> (SessionStore, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().join("sessions.db")).unwrap();
        let sid = "F13-resume-test".to_string();
        store
            .insert_session_with_parent(
                &sid,
                "Refactor the auth middleware",
                "mock",
                "test-model",
                None,
                0,
            )
            .unwrap();
        store
            .insert_message(&sid, "user", "Refactor the auth middleware")
            .unwrap();
        store.insert_message(&sid, "assistant", "Sure").unwrap();
        store
            .insert_message(&sid, "user", "Wire refresh tokens")
            .unwrap();
        (store, sid, dir)
    }

    fn store_recap(store: &SessionStore, sid: &str) -> crate::recap::Recap {
        let detail = store
            .get_session_detail(sid)
            .unwrap()
            .expect("session must exist");
        let recap = heuristic_recap(&detail);
        store.insert_recap(&recap).expect("insert recap")
    }

    // ── Source resolution + auth-shaped 4xx ─────────────────────────────

    #[test]
    fn resume_post_requires_recap_id_or_subject_id() {
        let (store, _, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: None,
            kind: None,
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_BAD_REQUEST);
        assert!(
            out.body["error"]
                .as_str()
                .is_some_and(|s| s.contains("from_recap_id") && s.contains("from_subject_id")),
            "error must list both options; got: {}",
            out.body
        );
    }

    #[test]
    fn resume_post_subject_id_path_requires_kind() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid),
            kind: None,
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_BAD_REQUEST);
        assert!(out.body["error"]
            .as_str()
            .is_some_and(|s| s.contains("kind is required")));
    }

    #[test]
    fn resume_post_unknown_kind_returns_400() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid),
            kind: Some("diff_chain".to_string()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_BAD_REQUEST);
        assert!(out.body["error"]
            .as_str()
            .is_some_and(|s| s.contains("diff_chain")));
    }

    #[test]
    fn resume_post_missing_recap_returns_404() {
        let (store, _, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: Some("ghost-recap".to_string()),
            from_subject_id: None,
            kind: None,
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_NOT_FOUND);
        assert!(out.body["error"]
            .as_str()
            .is_some_and(|s| s.contains("ghost-recap")));
    }

    #[test]
    fn resume_post_missing_session_returns_404() {
        let (store, _, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some("ghost-session".to_string()),
            kind: Some("session".to_string()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_NOT_FOUND);
        assert!(out.body["error"]
            .as_str()
            .is_some_and(|s| s.contains("ghost-session")));
    }

    // ── Happy paths ──────────────────────────────────────────────────────

    #[test]
    fn resume_post_subject_id_path_creates_handle() {
        // Subject-only resume: no recap exists yet but the session
        // does. Pinning that the route doesn't require a pre-existing
        // recap row — the design supports raw session resume too.
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid.clone()),
            kind: Some("session".to_string()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: Some("vibeui".to_string()),
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_OK);
        assert_eq!(out.body["resumed_session_id"], sid);
        assert_eq!(out.body["ready"], true);
        assert_eq!(out.body["branched"], false);
        // 3 messages in fixture, no cursor → all primed.
        assert_eq!(out.body["primed_message_count"], 3);
        assert_eq!(reg.len(), 1, "handle must land in the registry");
    }

    #[test]
    fn resume_post_recap_id_path_creates_handle_and_uses_recap_hint() {
        // Recap path: the recap's resume_hint is the canonical
        // default. Without explicit overrides, the response mirrors
        // recap.resume_hint values.
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let recap = store_recap(&store, &sid);
        // The heuristic recap's resume_hint points at the last message
        // and tail-resumes (branch_on_resume = false).
        let expected_from = recap.resume_hint.as_ref().and_then(|h| h.from_message);

        let req = ResumeRequest {
            from_recap_id: Some(recap.id.clone()),
            from_subject_id: None,
            kind: None,
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_OK);
        assert_eq!(out.body["resumed_session_id"], sid);
        assert_eq!(out.body["branched"], false);
        // The from_message in the response must mirror the recap hint
        // when no override is supplied.
        match expected_from {
            Some(id) => assert_eq!(out.body["from_message"], id),
            None => assert!(out.body["from_message"].is_null()),
        }
    }

    #[test]
    fn resume_post_explicit_overrides_beat_recap_hint() {
        // Per the design: "from_message overrides recap.resume_hint.
        // from_message". Pinning that order so a client that
        // deliberately rewinds to mid-conversation isn't silently
        // pulled back to the tail.
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let recap = store_recap(&store, &sid);
        // Use the SECOND message id as cursor (overriding the recap
        // hint which points at the third / last).
        let detail = store.get_session_detail(&sid).unwrap().unwrap();
        let second_id = detail.messages[1].id;
        let req = ResumeRequest {
            from_recap_id: Some(recap.id.clone()),
            from_subject_id: None,
            kind: None,
            from_message: Some(second_id),
            seed_instruction: Some("override seed".to_string()),
            branch: Some(true),
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_OK);
        assert_eq!(out.body["from_message"], second_id);
        assert_eq!(out.body["seed_instruction"], "override seed");
        assert_eq!(out.body["branched"], true);
    }

    #[test]
    fn resume_post_branch_true_creates_fork() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid.clone()),
            kind: Some("session".to_string()),
            from_message: None,
            seed_instruction: None,
            branch: Some(true),
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_OK);
        let resumed = out.body["resumed_session_id"].as_str().unwrap().to_string();
        assert_ne!(resumed, sid, "branch must produce a new session id");
        assert!(
            resumed.starts_with("fork-"),
            "fork id must follow the daemon's fork- convention; got: {resumed}"
        );
        // The fork must exist and contain the same messages.
        let fork_detail = store
            .get_session_detail(&resumed)
            .unwrap()
            .expect("fork must exist");
        assert_eq!(fork_detail.messages.len(), 3);
    }

    #[test]
    fn resume_post_primed_count_uses_from_message_cursor() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let detail = store.get_session_detail(&sid).unwrap().unwrap();
        let cursor = detail.messages[1].id; // second of three
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid),
            kind: Some("session".to_string()),
            from_message: Some(cursor),
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let out = do_v1_resume_post(&store, &reg, &req);
        assert_eq!(out.status, HTTP_OK);
        // 2 messages have id <= cursor (first two).
        assert_eq!(out.body["primed_message_count"], 2);
    }

    // ── GET /v1/resume/:handle ───────────────────────────────────────────

    #[test]
    fn resume_get_returns_stored_record() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        let post = do_v1_resume_post(
            &store,
            &reg,
            &ResumeRequest {
                from_recap_id: None,
                from_subject_id: Some(sid),
                kind: Some("session".to_string()),
                from_message: None,
                seed_instruction: None,
                branch: None,
                client: None,
            },
        );
        let handle = post.body["handle"].as_str().unwrap().to_string();
        let get = do_v1_resume_get(&reg, &handle);
        assert_eq!(get.status, HTTP_OK);
        assert_eq!(get.body["handle"], handle);
        assert_eq!(get.body["ready"], true);
    }

    #[test]
    fn resume_get_missing_handle_returns_404() {
        let reg = ResumeRegistry::new();
        let out = do_v1_resume_get(&reg, "no-such-handle");
        assert_eq!(out.status, HTTP_NOT_FOUND);
        assert!(out.body["error"]
            .as_str()
            .is_some_and(|s| s.contains("not found")));
    }

    // ── Registry hygiene ─────────────────────────────────────────────────

    #[test]
    fn registry_distinct_handles_coexist() {
        let (store, sid, _dir) = fixture();
        let reg = ResumeRegistry::new();
        for _ in 0..3 {
            let _ = do_v1_resume_post(
                &store,
                &reg,
                &ResumeRequest {
                    from_recap_id: None,
                    from_subject_id: Some(sid.clone()),
                    kind: Some("session".to_string()),
                    from_message: None,
                    seed_instruction: None,
                    branch: None,
                    client: None,
                },
            );
        }
        assert_eq!(reg.len(), 3);
    }

    // ── J1.3b: kind=job resume helper tests ────────────────────────────

    use crate::job_manager::{
        AgentEventPayload, CreateJobReq, JobManager, JobStatus, JobsDb,
    };
    use std::sync::Arc;

    /// Build a JobManager with one terminal-state job and an auto-recap
    /// produced by J1.2's terminal-state hook. Returns the manager, the
    /// parent job_id, and the recap_id of the freshly persisted recap.
    async fn job_resume_fixture(
        task: &str,
    ) -> (Arc<JobManager>, String, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = JobsDb::open_with(&dir.path().join("jobs.db"), [42u8; 32]).unwrap();
        let mgr = Arc::new(JobManager::new_with(db));
        let sid = mgr
            .create(CreateJobReq {
                task: task.into(),
                provider: "mock".into(),
                approval: "auto".into(),
                workspace_root: "/tmp/ws".into(),
                priority: 5,
                webhook_url: None,
                tags: vec!["urgent".into()],
                quota_bucket: None,
            })
            .await
            .unwrap();
        mgr.mark_running(&sid).await.unwrap();
        mgr.publish_event(
            &sid,
            AgentEventPayload::complete("Next, run the test suite.".into()),
        )
        .await;
        mgr.mark_terminal(&sid, JobStatus::Complete, Some("ok".into()), None)
            .await
            .unwrap();
        // J1.2 auto-recap landed exactly one row.
        let recap = mgr
            .list_job_recaps_for_subject(&sid, 1)
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("auto-recap should exist after mark_terminal");
        (mgr, sid, recap.id, dir)
    }

    #[tokio::test]
    async fn resume_job_post_creates_new_job_linked_to_parent() {
        let (mgr, parent, recap_id, _dir) = job_resume_fixture("Refactor").await;
        let req = ResumeRequest {
            from_recap_id: Some(recap_id.clone()),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, HTTP_OK);
        let new_id = out.body["resumed_session_id"].as_str().unwrap();
        assert_ne!(new_id, parent, "resume must spawn a new job_id");
        assert!(out.body["branched"].as_bool().unwrap());

        // Verify the new job's parent_link columns point at the parent.
        let (pid, rid) = mgr.get_parent_link(new_id).await.unwrap();
        assert_eq!(pid.as_deref(), Some(parent.as_str()));
        assert_eq!(rid.as_deref(), Some(recap_id.as_str()));
    }

    #[tokio::test]
    async fn resume_job_post_404s_on_missing_recap() {
        let (mgr, _parent, _recap_id, _dir) = job_resume_fixture("Task").await;
        let req = ResumeRequest {
            from_recap_id: Some("no-such-recap".into()),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, HTTP_NOT_FOUND);
    }

    #[tokio::test]
    async fn resume_job_post_uses_seed_from_recap_when_unspecified() {
        let (mgr, _parent, recap_id, _dir) = job_resume_fixture("Refactor").await;
        let req = ResumeRequest {
            from_recap_id: Some(recap_id),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, HTTP_OK);
        let seed = out.body["seed_instruction"].as_str();
        assert!(
            seed.is_some_and(|s| s.to_lowercase().contains("test")),
            "expected seed pulled from recap.next_actions; got {seed:?}"
        );
    }

    #[tokio::test]
    async fn resume_job_post_override_beats_recap_seed() {
        let (mgr, _parent, recap_id, _dir) = job_resume_fixture("Task").await;
        let req = ResumeRequest {
            from_recap_id: Some(recap_id),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: Some("Custom override".into()),
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, HTTP_OK);
        assert_eq!(
            out.body["seed_instruction"].as_str(),
            Some("Custom override")
        );
    }

    #[tokio::test]
    async fn resume_job_post_409s_on_running_parent() {
        let dir = tempfile::tempdir().unwrap();
        let db = JobsDb::open_with(&dir.path().join("jobs.db"), [42u8; 32]).unwrap();
        let mgr = Arc::new(JobManager::new_with(db));
        let sid = mgr
            .create(CreateJobReq {
                task: "Running".into(),
                provider: "mock".into(),
                approval: "auto".into(),
                workspace_root: "/tmp/ws".into(),
                priority: 5,
                webhook_url: None,
                tags: vec![],
                quota_bucket: None,
            })
            .await
            .unwrap();
        mgr.mark_running(&sid).await.unwrap();
        // Don't mark terminal — parent stays running.
        let req = ResumeRequest {
            from_recap_id: None,
            from_subject_id: Some(sid),
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: Some("Try again".into()),
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, 409);
    }

    #[tokio::test]
    async fn resume_job_post_inherits_provider_and_tags() {
        let (mgr, _parent, recap_id, _dir) = job_resume_fixture("Inherit").await;
        let req = ResumeRequest {
            from_recap_id: Some(recap_id),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: Some("Continue".into()),
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        let new_id = out.body["resumed_session_id"].as_str().unwrap();
        let new_job = mgr.get(new_id).await.expect("new job exists");
        assert_eq!(new_job.provider, "mock");
        assert_eq!(new_job.tags, vec!["urgent"]);
    }

    #[tokio::test]
    async fn resume_job_post_400s_on_session_recap_mismatch() {
        // A recap id that doesn't exist in the jobs store should 404
        // (not 400), because the helper can't tell whether the id is
        // a session recap from a different store. The 400 path triggers
        // when the id resolves to a recap whose `kind != Job` — which
        // is impossible inside JobsDb (its insert rejects non-job kinds).
        // So we only assert 404 for unknown ids.
        let (mgr, _p, _r, _dir) = job_resume_fixture("Task").await;
        let req = ResumeRequest {
            from_recap_id: Some("session-recap-id".into()),
            from_subject_id: None,
            kind: Some("job".into()),
            from_message: None,
            seed_instruction: None,
            branch: None,
            client: None,
        };
        let registry = ResumeRegistry::default();
        let out = do_v1_resume_post_job(&mgr, &registry, &req).await;
        assert_eq!(out.status, HTTP_NOT_FOUND);
    }
}
