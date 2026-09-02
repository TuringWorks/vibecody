//! HTTP surface for the engagement lifecycle (`/engagements/*`).
//!
//! Mounted behind `require_auth` in [`crate::serve`], alongside the other
//! authenticated routes. Everything here is a thin shell over
//! [`crate::engagement`]: the store holds the rules, these handlers hold the
//! transport.
//!
//! SQLite is blocking, so every handler hops to `spawn_blocking` rather than
//! stalling a Tokio worker on a file lock — the daemon serves streaming chat on
//! the same runtime and a blocked worker there is felt immediately.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engagement::{
    render_handover_markdown, render_report_markdown, DeliverableStatus, EngagementStatus,
    EngagementStore, EvidenceKind, GateVerdict, Phase, GATE_TEMPLATE, TEMPLATE,
};
use crate::serve::ServeState;

type HttpError = (StatusCode, Json<serde_json::Value>);

fn err(status: StatusCode, msg: impl Into<String>) -> HttpError {
    (status, Json(json!({ "error": msg.into() })))
}

/// Run a store operation off the async runtime.
///
/// The store is opened per call. That is deliberate: an engagement mutation is
/// a human-paced action measured in minutes, so a connection pool would buy
/// nothing and a long-lived handle would hold a WAL lock across the daemon's
/// whole lifetime for no reason.
async fn with_store<T, F>(f: F) -> Result<T, HttpError>
where
    F: FnOnce(&EngagementStore) -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let store = EngagementStore::open_default()?;
        f(&store)
    })
    .await
    .map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("engagement task panicked or was cancelled: {e}"),
        )
    })?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))
}

/// Parse a `?phase=` filter. An unrecognised value is a 400, not a silent
/// "all phases" — a typo that quietly widens a query is how a client ends up
/// reading another phase's numbers.
fn parse_phase(raw: Option<&str>) -> Result<Option<Phase>, HttpError> {
    match raw {
        None => Ok(None),
        Some(s) if s.is_empty() || s == "all" => Ok(None),
        Some(s) => Phase::from_str(s).map(Some).ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                format!("unknown phase '{s}'; expected discover|prove|build|operate"),
            )
        }),
    }
}

// ── Requests ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateEngagementReq {
    pub name: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Deserialize)]
pub struct PatchEngagementReq {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhaseQuery {
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchDeliverableReq {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddDeliverableReq {
    pub phase: String,
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tool_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddEvidenceReq {
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub reference: String,
}

fn default_kind() -> String {
    "note".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AddGateReq {
    pub phase: String,
    pub title: String,
    #[serde(default)]
    pub criterion: String,
    #[serde(default)]
    pub measurement: String,
}

#[derive(Debug, Deserialize)]
pub struct JudgeGateReq {
    pub verdict: String,
    /// What was actually observed. Omitted stays `null` in the store — a
    /// verdict with no observation is meant to look like one.
    #[serde(default)]
    pub observed: Option<String>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub decided_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdvanceReq {
    /// Exit the phase with blockers outstanding. The blockers are still
    /// returned and the override is recorded as an override.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct TemplateRow {
    pub phase: &'static str,
    pub phase_title: &'static str,
    pub cadence: Option<&'static str>,
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub tool_hint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct GateTemplateRow {
    pub phase: &'static str,
    pub title: &'static str,
    pub criterion: &'static str,
    pub measurement: &'static str,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /engagements/template` — the engagement model itself.
///
/// Static, so it answers before any engagement exists. The UI uses it to show
/// what the four phases promise, and `tool_hint` is what turns each promise
/// into a link to the panel that produces it.
async fn get_template() -> Json<serde_json::Value> {
    let deliverables: Vec<TemplateRow> = TEMPLATE
        .iter()
        .map(|t| TemplateRow {
            phase: t.phase.as_str(),
            phase_title: t.phase.title(),
            cadence: t.phase.cadence(),
            key: t.key,
            title: t.title,
            description: t.description,
            tool_hint: t.tool_hint,
        })
        .collect();
    let gates: Vec<GateTemplateRow> = GATE_TEMPLATE
        .iter()
        .map(|g| GateTemplateRow {
            phase: g.phase.as_str(),
            title: g.title,
            criterion: g.criterion,
            measurement: g.measurement,
        })
        .collect();
    let phases: Vec<serde_json::Value> = Phase::ALL
        .iter()
        .map(|p| {
            json!({
                "phase": p.as_str(),
                "title": p.title(),
                // `null` for Discover, which publishes no duration. The UI
                // renders that as a dash, never as an invented range.
                "cadence": p.cadence(),
                "purpose": p.purpose(),
                "index": p.index(),
            })
        })
        .collect();
    Json(json!({ "phases": phases, "deliverables": deliverables, "gates": gates }))
}

async fn list_engagements() -> Result<Json<serde_json::Value>, HttpError> {
    let items = with_store(|s| s.list()).await?;
    Ok(Json(json!({ "engagements": items })))
}

async fn create_engagement(
    Json(req): Json<CreateEngagementReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    if req.name.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name must not be empty"));
    }
    let e = with_store(move |s| {
        s.create(
            req.name.trim(),
            req.client.trim(),
            req.workspace_path.as_deref(),
            req.summary.trim(),
        )
    })
    .await?;
    Ok(Json(json!({ "engagement": e })))
}

/// `GET /engagements/{id}` — the engagement plus its full readiness report.
async fn get_engagement(Path(id): Path<String>) -> Result<Json<serde_json::Value>, HttpError> {
    let id2 = id.clone();
    let report = with_store(move |s| s.report(&id2)).await.map_err(|_| {
        err(
            StatusCode::NOT_FOUND,
            format!("no engagement '{}'", sanitize(&id)),
        )
    })?;
    Ok(Json(json!({ "report": report })))
}

async fn patch_engagement(
    Path(id): Path<String>,
    Json(req): Json<PatchEngagementReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    // Validate before touching the store so a bad phase is a 400 rather than a
    // silent no-op the caller reads as success.
    let phase = match req.phase.as_deref() {
        None => None,
        Some(p) => Some(Phase::from_str(p).ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                format!("unknown phase '{}'", sanitize(p)),
            )
        })?),
    };
    let status = req.status.as_deref().map(EngagementStatus::from_str);
    let id2 = id.clone();
    let e = with_store(move |s| {
        if let Some(st) = status {
            s.set_status(&id2, st)?;
        }
        if let Some(p) = phase {
            s.set_phase(&id2, p)?;
        }
        s.get(&id2)
    })
    .await?;
    match e {
        Some(e) => Ok(Json(json!({ "engagement": e }))),
        None => Err(err(
            StatusCode::NOT_FOUND,
            format!("no engagement '{}'", sanitize(&id)),
        )),
    }
}

async fn delete_engagement(Path(id): Path<String>) -> Result<Json<serde_json::Value>, HttpError> {
    let removed = with_store(move |s| s.delete(&id)).await?;
    Ok(Json(json!({ "deleted": removed })))
}

/// `POST /engagements/{id}/seed` — top up an engagement with template rows it
/// is missing, without disturbing anything already accepted.
async fn seed_engagement(Path(id): Path<String>) -> Result<Json<serde_json::Value>, HttpError> {
    let added = with_store(move |s| s.seed_template(&id)).await?;
    Ok(Json(json!({ "added": added })))
}

async fn list_deliverables(
    Path(id): Path<String>,
    Query(q): Query<PhaseQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let phase = parse_phase(q.phase.as_deref())?;
    let items = with_store(move |s| s.deliverables(&id, phase)).await?;
    Ok(Json(json!({ "deliverables": items })))
}

async fn add_deliverable(
    Path(id): Path<String>,
    Json(req): Json<AddDeliverableReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let phase = Phase::from_str(&req.phase).ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            format!("unknown phase '{}'", sanitize(&req.phase)),
        )
    })?;
    if req.key.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "key must not be empty"));
    }
    let d = with_store(move |s| {
        s.add_deliverable(
            &id,
            phase,
            req.key.trim(),
            req.title.trim(),
            req.description.trim(),
            req.tool_hint.as_deref(),
        )
    })
    .await?;
    Ok(Json(json!({ "deliverable": d })))
}

async fn patch_deliverable(
    Path((_id, did)): Path<(String, String)>,
    Json(req): Json<PatchDeliverableReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let status = req.status.as_deref().map(DeliverableStatus::from_str);
    let did2 = did.clone();
    with_store(move |s| {
        s.update_deliverable(&did2, status, req.owner.as_deref(), req.notes.as_deref())
    })
    .await?;
    Ok(Json(json!({ "updated": did })))
}

async fn list_evidence(
    Path((_id, did)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let items = with_store(move |s| s.evidence(&did)).await?;
    Ok(Json(json!({ "evidence": items })))
}

async fn add_evidence(
    Path((_id, did)): Path<(String, String)>,
    Json(req): Json<AddEvidenceReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let kind = EvidenceKind::from_str(&req.kind);
    let e = with_store(move |s| s.add_evidence(&did, kind, &req.label, &req.reference)).await?;
    Ok(Json(json!({ "evidence": e })))
}

async fn delete_evidence(
    Path((_id, eid)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let removed = with_store(move |s| s.delete_evidence(&eid)).await?;
    Ok(Json(json!({ "deleted": removed })))
}

async fn list_gates(
    Path(id): Path<String>,
    Query(q): Query<PhaseQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let phase = parse_phase(q.phase.as_deref())?;
    let items = with_store(move |s| s.gates(&id, phase)).await?;
    Ok(Json(json!({ "gates": items })))
}

async fn add_gate(
    Path(id): Path<String>,
    Json(req): Json<AddGateReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let phase = Phase::from_str(&req.phase).ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            format!("unknown phase '{}'", sanitize(&req.phase)),
        )
    })?;
    if req.title.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "gate title must not be empty"));
    }
    // A criterion with no measurement procedure is settled by whoever argues
    // hardest at the review, which is the failure this whole subsystem exists
    // to prevent. Refuse it at the edge.
    if req.measurement.trim().is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a gate needs a measurement procedure: state how it will be judged, \
             agreed before the phase runs",
        ));
    }
    let g = with_store(move |s| {
        s.add_gate(
            &id,
            phase,
            req.title.trim(),
            req.criterion.trim(),
            req.measurement.trim(),
        )
    })
    .await?;
    Ok(Json(json!({ "gate": g })))
}

async fn judge_gate(
    Path((_id, gid)): Path<(String, String)>,
    Json(req): Json<JudgeGateReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    // `GateVerdict::from_str` falls back to `not_measured`. That is the right
    // default for a database column and the wrong one for an API: a caller
    // that typed "passed" would see their gate silently reset. Validate here.
    let verdict = match req.verdict.as_str() {
        "not_measured" => GateVerdict::NotMeasured,
        "pending" => GateVerdict::Pending,
        "pass" => GateVerdict::Pass,
        "fail" => GateVerdict::Fail,
        "waived" => GateVerdict::Waived,
        other => {
            return Err(err(
                StatusCode::BAD_REQUEST,
                format!(
                    "unknown verdict '{}'; expected not_measured|pending|pass|fail|waived",
                    sanitize(other)
                ),
            ))
        }
    };
    // A pass with nothing observed is an assertion, not a measurement.
    if verdict == GateVerdict::Pass
        && req
            .observed
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a passing gate must record what was observed",
        ));
    }
    let gid2 = gid.clone();
    with_store(move |s| {
        s.judge_gate(
            &gid2,
            verdict,
            req.observed.as_deref(),
            &req.rationale,
            req.decided_by.as_deref(),
        )?;
        s.gate(&gid2)
    })
    .await?
    .map(|g| Json(json!({ "gate": g })))
    .ok_or_else(|| {
        err(
            StatusCode::NOT_FOUND,
            format!("no gate '{}'", sanitize(&gid)),
        )
    })
}

async fn delete_gate(
    Path((_id, gid)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let removed = with_store(move |s| s.delete_gate(&gid)).await?;
    Ok(Json(json!({ "deleted": removed })))
}

async fn advance_phase(
    Path(id): Path<String>,
    Json(req): Json<AdvanceReq>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let outcome = with_store(move |s| s.advance_phase(&id, req.force)).await?;
    Ok(Json(json!({ "outcome": outcome })))
}

async fn report_markdown(Path(id): Path<String>) -> Result<impl IntoResponse, HttpError> {
    let md = with_store(move |s| render_report_markdown(s, &id)).await?;
    Ok(markdown(md))
}

async fn handover_markdown(Path(id): Path<String>) -> Result<impl IntoResponse, HttpError> {
    let md = with_store(move |s| render_handover_markdown(s, &id)).await?;
    Ok(markdown(md))
}

fn markdown(body: String) -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        body,
    )
}

/// Strip control characters before echoing caller input into an error body.
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>()
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn build_routes() -> axum::Router<ServeState> {
    use axum::routing::{delete, get, patch, post};
    axum::Router::new()
        .route("/engagements/template", get(get_template))
        .route(
            "/engagements",
            get(list_engagements).post(create_engagement),
        )
        .route(
            "/engagements/{id}",
            get(get_engagement)
                .patch(patch_engagement)
                .delete(delete_engagement),
        )
        .route("/engagements/{id}/seed", post(seed_engagement))
        .route(
            "/engagements/{id}/deliverables",
            get(list_deliverables).post(add_deliverable),
        )
        .route(
            "/engagements/{id}/deliverables/{did}",
            patch(patch_deliverable),
        )
        .route(
            "/engagements/{id}/deliverables/{did}/evidence",
            get(list_evidence).post(add_evidence),
        )
        .route("/engagements/{id}/evidence/{eid}", delete(delete_evidence))
        .route("/engagements/{id}/gates", get(list_gates).post(add_gate))
        .route("/engagements/{id}/gates/{gid}", delete(delete_gate))
        .route("/engagements/{id}/gates/{gid}/judge", post(judge_gate))
        .route("/engagements/{id}/advance", post(advance_phase))
        .route("/engagements/{id}/report.md", get(report_markdown))
        .route("/engagements/{id}/handover.md", get(handover_markdown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_phase_filter_is_rejected_not_widened() {
        assert!(parse_phase(Some("discovery")).is_err());
        assert_eq!(parse_phase(Some("discover")).ok().flatten(), Some(Phase::Discover));
        assert_eq!(parse_phase(None).ok().flatten(), None);
        assert_eq!(parse_phase(Some("all")).ok().flatten(), None);
    }

    #[test]
    fn sanitize_strips_control_characters() {
        assert_eq!(sanitize("ok\u{0}\n\u{1b}[31m"), "ok[31m");
        assert_eq!(sanitize(&"x".repeat(500)).len(), 120);
    }

    #[test]
    fn every_route_path_is_under_the_engagements_prefix() {
        // The router is mounted inside the authed group; a path that escaped
        // the prefix would still be authenticated, but it would collide with
        // an unrelated namespace.
        let src = include_str!("engagement_routes.rs");
        for line in src.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix(".route(\"") {
                let path = rest.split('"').next().unwrap_or_default();
                assert!(
                    path.starts_with("/engagements"),
                    "route '{path}' is outside the /engagements namespace"
                );
            }
        }
    }
}
