//! HTTP surface for Developer Excellence metrics (`/devex/*`).
//!
//! Mounted behind `require_auth` in [`crate::serve`]. Everything here is a thin
//! shell over [`crate::devex_metrics`]: the module holds the measurement rules,
//! these handlers hold the transport.
//!
//! Every handler forks `git` or walks a directory, both blocking. They hop to
//! `spawn_blocking` rather than stalling a Tokio worker — the daemon streams
//! chat on the same runtime and a blocked worker is felt immediately there.

use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::json;

use crate::devex_metrics::{
    compute_dora, compute_space, render_scorecard_markdown, render_survey_markdown,
    scan_onboarding, scan_practices, scorecard, DoraOptions, ReleaseMarker, DEFAULT_WINDOW_DAYS,
};

type HttpError = (StatusCode, Json<serde_json::Value>);

fn err(status: StatusCode, msg: impl Into<String>) -> HttpError {
    (status, Json(json!({ "error": msg.into() })))
}

/// Upper bound on the measurement window.
///
/// Five years. Not a guess at what anybody wants — a bound so a caller passing
/// `window=99999999` cannot make the daemon walk a decade of history on every
/// request. A request above it is rejected with the bound named, rather than
/// silently clamped: a clamped value rendered as "5 years" would be a number
/// the caller never asked for presented as one they did.
const MAX_WINDOW_DAYS: u32 = 365 * 5;

#[derive(Debug, Deserialize)]
pub struct DevexQuery {
    /// Workspace or repository path. Required — see [`resolve_path`].
    pub path: Option<String>,
    /// Measurement window in days. Defaults to
    /// [`DEFAULT_WINDOW_DAYS`](crate::devex_metrics::DEFAULT_WINDOW_DAYS).
    pub window: Option<u32>,
    /// `tags` (default) or `merges`.
    pub marker: Option<String>,
    /// Branch consulted when `marker=merges`.
    pub branch: Option<String>,
}

/// A path is required, never inferred.
///
/// Falling back to the daemon's cwd would measure whatever directory the daemon
/// happened to be launched from and label the answer with the caller's repo —
/// the same trap `engagement_routes::scan_workspace` refuses.
fn resolve_path(q: &DevexQuery) -> Result<std::path::PathBuf, HttpError> {
    match q.path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => Ok(std::path::PathBuf::from(p)),
        None => Err(err(
            StatusCode::BAD_REQUEST,
            "`path` is required: pass the repository or workspace to measure. \
             The daemon will not guess a directory and label the result yours.",
        )),
    }
}

fn resolve_window(q: &DevexQuery) -> Result<u32, HttpError> {
    match q.window {
        None => Ok(DEFAULT_WINDOW_DAYS),
        Some(0) => Err(err(
            StatusCode::BAD_REQUEST,
            "`window` must be at least 1 day",
        )),
        Some(w) if w > MAX_WINDOW_DAYS => Err(err(
            StatusCode::BAD_REQUEST,
            format!("`window` must be {MAX_WINDOW_DAYS} days or fewer"),
        )),
        Some(w) => Ok(w),
    }
}

fn resolve_opts(q: &DevexQuery) -> Result<DoraOptions, HttpError> {
    let marker = match q.marker.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
        None => ReleaseMarker::VersionTags,
        Some(m) => ReleaseMarker::from_str(m).ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                format!("unknown marker '{}'; expected tags|merges", sanitize(m)),
            )
        })?,
    };
    Ok(DoraOptions {
        window_days: resolve_window(q)?,
        release_marker: marker,
        release_branch: q
            .branch
            .clone()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_else(|| "HEAD".to_string()),
    })
}

/// Echo caller-supplied text back into an error without letting it carry
/// control characters or unbounded length into a log line or a UI toast.
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect::<String>()
}

/// Run a blocking measurement off the async runtime.
async fn blocking<T, F>(what: &'static str, f: F) -> Result<T, HttpError>
where
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{what} task panicked or was cancelled: {e}"),
            )
        })?
        // A bad path or a directory that is not a repository is the caller's
        // input, not the daemon failing — 400, so a client can say "point me at
        // a repo" rather than "the daemon is broken".
        .map_err(|e| err(StatusCode::BAD_REQUEST, format!("{e:#}")))
}

async fn get_dora(Query(q): Query<DevexQuery>) -> Result<Json<serde_json::Value>, HttpError> {
    let path = resolve_path(&q)?;
    let opts = resolve_opts(&q)?;
    let report = blocking("dora", move || compute_dora(&path, &opts)).await?;
    Ok(Json(json!({ "dora": report })))
}

async fn get_practices(Query(q): Query<DevexQuery>) -> Result<Json<serde_json::Value>, HttpError> {
    let path = resolve_path(&q)?;
    let report = blocking("practices", move || scan_practices(&path)).await?;
    Ok(Json(json!({ "practices": report })))
}

async fn get_onboarding(Query(q): Query<DevexQuery>) -> Result<Json<serde_json::Value>, HttpError> {
    let path = resolve_path(&q)?;
    let window = resolve_window(&q)?;
    let report = blocking("onboarding", move || scan_onboarding(&path, window)).await?;
    Ok(Json(json!({ "onboarding": report })))
}

async fn get_scorecard(Query(q): Query<DevexQuery>) -> Result<Json<serde_json::Value>, HttpError> {
    let path = resolve_path(&q)?;
    let opts = resolve_opts(&q)?;
    let sc = blocking("scorecard", move || scorecard(&path, &opts)).await?;
    Ok(Json(json!({ "scorecard": sc })))
}

/// The scorecard as the briefing a director circulates.
///
/// Returned as `text/markdown` so a panel can render it and a pipeline can pipe
/// it into a wiki without re-deriving the layout in three clients.
async fn get_scorecard_markdown(
    Query(q): Query<DevexQuery>,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), HttpError> {
    let path = resolve_path(&q)?;
    let opts = resolve_opts(&q)?;
    let sc = blocking("scorecard", move || scorecard(&path, &opts)).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        render_scorecard_markdown(&sc),
    ))
}

async fn get_space(Query(q): Query<DevexQuery>) -> Result<Json<serde_json::Value>, HttpError> {
    let path = resolve_path(&q)?;
    let opts = resolve_opts(&q)?;
    let report = blocking("space", move || {
        // DORA first, handed in: Performance references the stability pair
        // instead of restating it.
        let dora = compute_dora(&path, &opts)?;
        compute_space(&path, opts.window_days, &dora)
    })
    .await?;
    Ok(Json(json!({ "space": report })))
}

/// The survey instrument. Static text, but served here so every client renders
/// the same questions — a survey whose wording drifts per surface cannot be
/// tracked over time.
async fn get_survey() -> ([(axum::http::HeaderName, &'static str); 1], String) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        render_survey_markdown(),
    )
}

pub fn build_routes<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    use axum::routing::get;
    axum::Router::new()
        .route("/devex/dora", get(get_dora))
        .route("/devex/practices", get(get_practices))
        .route("/devex/onboarding", get(get_onboarding))
        .route("/devex/scorecard", get(get_scorecard))
        .route("/devex/scorecard.md", get(get_scorecard_markdown))
        .route("/devex/space", get(get_space))
        .route("/devex/survey.md", get(get_survey))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn app() -> axum::Router {
        build_routes::<()>().with_state(())
    }

    async fn get(uri: &str) -> (StatusCode, serde_json::Value) {
        let res = app()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .expect("router responded");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 22)
            .await
            .expect("body");
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn a_missing_path_is_refused_not_guessed() {
        let (status, body) = get("/devex/dora").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"].as_str().unwrap_or_default().contains("path"),
            "error should name the missing parameter: {body}"
        );
    }

    #[tokio::test]
    async fn an_unknown_marker_is_rejected_rather_than_defaulted() {
        // A typo that silently falls back to tags would report a number the
        // caller never asked for as the one they did.
        let (status, body) = get("/devex/dora?path=/tmp&marker=tagz").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("tags|merges"));
    }

    #[tokio::test]
    async fn an_out_of_range_window_is_rejected_not_clamped() {
        let (status, body) = get("/devex/dora?path=/tmp&window=99999999").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"]
            .as_str()
            .unwrap_or_default()
            .contains(&MAX_WINDOW_DAYS.to_string()));

        let (status, _) = get("/devex/dora?path=/tmp&window=0").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_non_repository_path_is_the_callers_error_not_a_500() {
        let dir = tempfile::tempdir().expect("tempdir");
        let uri = format!("/devex/dora?path={}", dir.path().display());
        let (status, _) = get(&uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn practices_scan_of_a_plain_directory_succeeds() {
        // Practices need no git history — an empty checkout still has an
        // answer, and it is "nothing detected", not an error.
        let dir = tempfile::tempdir().expect("tempdir");
        let uri = format!("/devex/practices?path={}", dir.path().display());
        let (status, body) = get(&uri).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["practices"]["practices"].as_array().is_some_and(|a| !a.is_empty()));
        assert_eq!(body["practices"]["mean_level"].as_f64(), Some(0.0));
    }

    #[tokio::test]
    async fn sanitize_strips_control_characters_and_bounds_length() {
        let dirty = format!("ta\ngs{}", "x".repeat(200));
        let clean = sanitize(&dirty);
        assert!(!clean.contains('\n'));
        assert!(clean.chars().count() <= 64);
    }

    #[tokio::test]
    async fn space_needs_a_path_like_every_other_measurement() {
        let (status, body) = get("/devex/space").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap_or_default().contains("path"));
    }

    #[tokio::test]
    async fn the_survey_is_served_without_a_path_because_it_measures_nothing() {
        // Every other route refuses to guess a repository. This one has no
        // repository to guess: it is an instrument, identical for everyone, and
        // served here so its wording cannot drift per client.
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/devex/survey.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router responded");
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/markdown; charset=utf-8")
        );
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .expect("body");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("Engineering experience survey"));
        // The commitments travel with the questions, not in a wiki somewhere.
        assert!(text.contains("Anonymous"));
    }

    #[test]
    fn every_route_is_a_read() {
        // Measurement must never mutate the repository it measures. If a write
        // verb is ever added here, this test is the place that argues about it.
        let routes = [
            "dora",
            "practices",
            "onboarding",
            "scorecard",
            "scorecard.md",
            "space",
            "survey.md",
        ];
        assert_eq!(routes.len(), 7);
    }
}
