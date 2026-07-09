//! # `skillforge_index` — the SkillLens / SkillOpt bridge into VibeCody
//!
//! The daemon-side adapter that wires the two standalone SkillForge crates
//! ([`skilllensai`] = analyse / measure, [`skilloptai`] = train / optimise)
//! into the 13-client product, following the same template as
//! [`crate::graph_index`] (kodegraph): **standalone crate → bridge module →
//! HTTP routes → clients.**
//!
//! ## What lives here
//!
//! - **Process-global catalog** — a [`skill_catalog::SkillCatalog`] over the
//!   shipped `skills/*.md` tree, plus a cache of [`SkillReport`]s. Built in
//!   the background on startup (parse-only — **no LLM, no key needed**), so
//!   the panel opens instantly. Surfaced in `/health` + the startup banner.
//! - **[`AiProviderLlm`]** — the STRICT provider-agnostic seam. Adapts the
//!   toolbar-selected `vibe_ai::AIProvider` onto the crate-local
//!   [`skilllensai::llm::SkillLlm`] trait. **Never hard-codes Anthropic**;
//!   the provider+model come from the request body (toolbar selection),
//!   never `config.toml`. See CLAUDE.md → Provider-Agnostic Panels.
//! - **[`RepoAgentEnv`]** — the day-one concrete `skilloptai::env::Env` that
//!   derives [`EvalTask`]s from the repo's own skill catalog (each skill's
//!   triggers → a `Contains`-graded task). Richer tasks drawn from real
//!   agent-job history (`ToolExit`/`LlmJudge` graders) are a follow-up.
//! - **Pure `*_value()` helpers** — every public helper returns
//!   `serde_json::Value` so **no SkillForge crate type leaks across the
//!   HTTP boundary** (same discipline as `graph_index`).
//!
//! ## Scope + known limitations
//!
//! - SkillForge is a dependency of `vibecli-cli` only. `vibe-ai` /
//!   `vibe-core` stay SkillForge-free.
//! - `POST /skillopt/train` spawns the run on a tokio task and returns a job
//!   id; `GET /skillopt/status/:job` reports `Running`/`Done`/`Failed`/
//!   `Cancelled`. `POST /v1/skillopt/train/stream` is the SSE variant — it
//!   emits `job` / per-epoch `epoch` / terminal `done` events. Both paths
//!   share the same `JOBS` map, and `cancel/:job` flips a live `CancelToken`
//!   (in `skilloptai::trainer`) so the run stops at the next epoch boundary.
//! - The catalog refresh is **on startup + explicit `/skilllens/refresh`**;
//!   there is no file-watcher loop driving `skills/*.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use serde_json::{json, Value};
use tokio::sync::Mutex;

use skilllensai::llm::{LlmDescriptor, SkillLlm};
use skilllensai::metrics::eval::{target_evolvability, EvalTask, Grader};
use skilllensai::model::{ExperiencePool, Skill as LensSkill};
use skilllensai::report::SkillReport;
use skilllensai::{
    convert as lens_convert,
    extract::{self as lens_extract, Extractor},
    VERSION as LENS_VERSION,
};
use skilloptai::env::{Env as OptEnv, StaticEnv};
use skilloptai::report::TrainingReport;
use skilloptai::trainer::{train_with_signals, CancelToken, EpochEvent, TrainConfig, TrainSignals};
use skilloptai::VERSION as OPT_VERSION;

use vibe_ai::provider::{AIProvider, Message, MessageRole};
use vibe_ai::{load_eval_records, SkillEvalRecord};

use crate::skill_catalog::SkillCatalog;

// ── versions (surfaced in /health) ──────────────────────────────────────────

/// Combined SkillForge toolchain version, surfaced in the `/health` block.
pub fn toolchain_version() -> String {
    format!("skilllensai {} · skilloptai {}", LENS_VERSION, OPT_VERSION)
}

// ── LLM adapter — the STRICT provider-agnostic seam ─────────────────────────

/// Adapter that implements the crate-local [`SkillLlm`] trait over a
/// daemon-constructed [`AIProvider`]. The provider+model come from the
/// toolbar selection (request body), never `config.toml` — no hard-coded
/// Anthropic.
pub struct AiProviderLlm {
    provider: Arc<dyn AIProvider>,
    model: String,
}

impl AiProviderLlm {
    pub fn new(provider: Arc<dyn AIProvider>, model: String) -> Self {
        Self { provider, model }
    }
}

#[async_trait::async_trait]
impl SkillLlm for AiProviderLlm {
    fn descriptor(&self) -> LlmDescriptor {
        LlmDescriptor {
            provider: self.provider.name().to_string(),
            model: self.model.clone(),
        }
    }

    async fn chat(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let messages = [
            Message {
                role: MessageRole::System,
                content: system.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user.to_string(),
            },
        ];
        self.provider
            .chat(&messages, None)
            .await
            .map_err(|e| anyhow::anyhow!("AiProviderLlm chat: {e:?}"))
    }
}

/// Build an [`AiProviderLlm`] from a toolbar-style `(provider, model)`
/// selection. Returns `None` when the provider isn't configured (no key in
/// `ProfileStore` and no env override) — callers surface a "select a
/// configured model" empty state rather than silently calling a default.
pub fn adapter_from_selection(provider: &str, model: &str) -> Option<AiProviderLlm> {
    let p = crate::serve::build_provider_override(provider, model)?;
    Some(AiProviderLlm::new(p, model.to_string()))
}

// ── process-global catalog + report cache ───────────────────────────────────

/// Lifecycle status of the SkillForge catalog, surfaced in `/health` + banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillForgeStatus {
    /// Background catalog parse in progress.
    Loading,
    /// Catalog loaded and queryable.
    Ready,
    /// No catalog (init not called / skills dir missing).
    Disabled,
}

impl SkillForgeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Disabled => "disabled",
        }
    }
}

/// A cached [`SkillReport`] keyed by skill name. `None` means "not yet
/// scored"; the panel shows the deterministic `trigger_coverage` from the
/// catalog even before an LLM score is computed.
type ReportCache = HashMap<String, SkillReport>;

struct SkillForgeState {
    catalog: SkillCatalog,
    reports: ReportCache,
    /// Promoted-skill override paths keyed by skill name, scanned from the
    /// per-workspace override dir (`<ws>/.vibecli/skills/*.opt.md`) on init +
    /// refresh. The shipped `skills/*.md` tree is never written to — promoted
    /// artifacts land here so the 710 shipped skills stay pristine. Surfaced
    /// as `promoted_override` on the skill-detail JSON.
    promoted_overrides: HashMap<String, PathBuf>,
}

static STATE: OnceLock<RwLock<SkillForgeState>> = OnceLock::new();
static STATUS: OnceLock<RwLock<SkillForgeStatus>> = OnceLock::new();

/// Resolve the bundled-skills directory the same way `mcp_server` does:
/// `VIBECLI_SKILLS_DIR` → `CARGO_MANIFEST_DIR/skills` →
/// `<exe>/../share/vibecli/skills`. Duplicated here (rather than importing
/// `mcp_server::skills_dir_default`, which is private) so the bridge stays
/// self-contained and testable in isolation.
fn skills_dir_default() -> PathBuf {
    if let Ok(p) = std::env::var("VIBECLI_SKILLS_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills");
    if manifest_dir.is_dir() {
        return manifest_dir;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("../share/vibecli/skills");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    manifest_dir
}

/// Resolve the per-workspace promoted-skill override dir — where promoted
/// `*.opt.md` artifacts land so the shipped `skills/*.md` tree stays pristine.
///
/// - `Some(ws)` → `<ws>/.vibecli/skills` (explicit workspace, e.g. from a
///   future request param or a test tempdir).
/// - `None` → resolve from the daemon's cwd: use it only when a workspace
///   store already exists there (`<cwd>/.vibecli/workspace.db`), so promote
///   never creates a stray `.vibecli/` tree in an arbitrary scratch dir.
///   Otherwise fall back to the global per-user `~/.vibecli/skills`.
pub fn promote_dir_for(workspace: Option<&Path>) -> PathBuf {
    if let Some(ws) = workspace {
        return ws.join(".vibecli").join("skills");
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join(".vibecli").join("workspace.db").exists() {
            return cwd.join(".vibecli").join("skills");
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".vibecli").join("skills"))
        .unwrap_or_else(|| PathBuf::from(".vibecli").join("skills"))
}

/// Scan `dir` for `*.opt.md` promoted-skill overrides, keyed by skill name
/// (the stem minus `.opt`). Returns an empty map when `dir` is missing — no
/// overrides yet is the common case. Pure (no STATE) so it's unit-testable.
fn scan_promoted_overrides_in(dir: &Path) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        // `file_stem("foo.opt.md")` == `"foo.opt"` → strip `.opt` → `"foo"`.
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(name) = stem.strip_suffix(".opt") else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        out.insert(name.to_string(), p);
    }
    out
}

/// Scan the resolved override dir for promoted skills (init + refresh path).
fn scan_promoted_overrides() -> HashMap<String, PathBuf> {
    scan_promoted_overrides_in(&promote_dir_for(None))
}

/// Initialize the process-global catalog. Idempotent — the `skills_dir`
/// argument is only used the first time. Kicks off a non-blocking background
/// parse (parse is CPU/IO-bound and cheap over ~710 small files, but we keep
/// it off the serving thread for parity with `graph_index`).
pub fn init_skillforge(skills_dir: Option<&Path>) -> SkillForgeStatus {
    let _ = STATUS.set(RwLock::new(SkillForgeStatus::Loading));
    let dir = skills_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(skills_dir_default);
    std::thread::spawn(move || {
        let catalog = SkillCatalog::load_from_with_cwd_plugins(&dir).unwrap_or_default();
        let state = SkillForgeState {
            catalog,
            reports: HashMap::new(),
            promoted_overrides: scan_promoted_overrides(),
        };
        let _ = STATE.set(RwLock::new(state));
        if let Some(s) = STATUS.get() {
            *s.write().unwrap() = SkillForgeStatus::Ready;
        }
    });
    SkillForgeStatus::Loading
}

/// Current lifecycle status (or `Disabled` if `init_skillforge` never ran).
pub fn current_status() -> SkillForgeStatus {
    STATUS
        .get()
        .map(|s| *s.read().unwrap())
        .unwrap_or(SkillForgeStatus::Disabled)
}

/// `/health` + banner payload: `{status, skills, cached_reports, toolchain}`.
pub fn status_value() -> Value {
    let status = current_status();
    let (skills, cached) = with_state(|s| (s.catalog.len(), s.reports.len())).unwrap_or((0, 0));
    json!({
        "status": status.as_str(),
        "skills": skills,
        "cached_reports": cached,
        "toolchain": toolchain_version(),
    })
}

/// Compact one-line "skill health" summary for the agent system prompt
/// (G3). Returns `None` when no skills have been scored yet, so the
/// prompt is **not** bloated for users who never ran SkillLens — the
/// line only appears once `cached_reports > 0`. This auto-gate replaces
/// the config-flag opt-in sketched in note 07: it satisfies the same
/// rationale ("don't bloat the prompt for users who haven't scored
/// skills") with no `Config` plumbing, and mirrors how kodegraph's
/// `graph_summary` is always-on when a graph exists.
///
/// Format: `N skills, M scored, top evolvability X.XX`.
pub fn render_health_line() -> Option<String> {
    let (skills, cached) = with_state(|s| (s.catalog.len(), s.reports.len())).unwrap_or((0, 0));
    if cached == 0 {
        return None;
    }
    let top_evo = with_state(|s| {
        s.reports
            .values()
            .filter_map(|r| r.target_evolvability)
            .fold(None, |acc, v| match acc {
                None => Some(v),
                Some(m) => Some(m.max(v)),
            })
    })
    .flatten();
    let top = match top_evo {
        Some(v) => format!("{v:.2}"),
        None => "—".to_string(),
    };
    Some(format!(
        "{skills} skills, {cached} scored, top evolvability {top}"
    ))
}

fn with_state<R>(f: impl FnOnce(&SkillForgeState) -> R) -> Option<R> {
    STATE.get().map(|g| f(&g.read().unwrap()))
}

fn with_state_mut<R>(f: impl FnOnce(&mut SkillForgeState) -> R) -> Option<R> {
    STATE.get().map(|g| f(&mut g.write().unwrap()))
}

/// Load a [`LensSkill`] by name from the catalog (parses the on-disk file).
fn lens_skill_by_name(name: &str) -> Option<LensSkill> {
    // `with_state` wraps in `Option<R>`; `catalog.get` returns `Option`, so we
    // get `Option<Option<PathBuf>>` — flatten with `??`.
    let path = with_state(|s| s.catalog.get(name).map(|e| e.path.clone()))??;
    LensSkill::from_file(&path).ok() // parse error → None (caller surfaces "not found")
}

// ── catalog helpers (pure → Value) ──────────────────────────────────────────

/// `GET /skilllens/skills` — catalog list with deterministic scores. No LLM.
pub fn list_skills_value() -> Value {
    let rows: Vec<Value> = with_state(|s| {
        s.catalog
            .all()
            .iter()
            .map(|skill| {
                let cached = s.reports.get(&skill.name);
                json!({
                    "name": skill.name,
                    "category": skill.frontmatter.category.clone().unwrap_or_default(),
                    "summary": skill.summary(),
                    "source": match &skill.source {
                        crate::skill_catalog::SkillSource::Builtin => "builtin",
                        crate::skill_catalog::SkillSource::Plugin(p) => p.as_str(),
                    },
                    "trigger_coverage": cached.map(|r| r.trigger_coverage),
                    "extraction_efficacy": cached.and_then(|r| r.extraction_efficacy),
                    "target_evolvability": cached.and_then(|r| r.target_evolvability),
                    "has_promoted_override": s.promoted_overrides.contains_key(&skill.name),
                })
            })
            .collect()
    })
    .unwrap_or_default();
    json!({ "skills": rows })
}

/// `GET /skilllens/skills/:name` — one skill + its body + cached report.
pub fn get_skill_value(name: &str) -> Option<Value> {
    let entry = with_state(|s| s.catalog.get(name).map(|e| e.clone()))??;
    let body = std::fs::read_to_string(&entry.path).unwrap_or_default();
    let (cached, promoted_override): (Option<SkillReport>, Option<String>) = with_state(|s| {
        (
            s.reports.get(name).cloned(),
            s.promoted_overrides
                .get(name)
                .map(|p| p.display().to_string()),
        )
    })?;
    Some(json!({
        "name": entry.name,
        "category": entry.frontmatter.category.clone().unwrap_or_default(),
        "triggers": entry.frontmatter.triggers,
        "tools_allowed": entry.frontmatter.tools_allowed,
        "source": match &entry.source {
            crate::skill_catalog::SkillSource::Builtin => "builtin",
            crate::skill_catalog::SkillSource::Plugin(p) => p.as_str(),
        },
        "body": body,
        "promoted_override": promoted_override,
        "report": cached.as_ref().map(|r| r.to_json()),
    }))
}

/// `POST /skilllens/refresh` — re-read the skills dir and reset the report
/// cache. Returns the new status payload.
pub fn refresh_value() -> Value {
    let dir = skills_dir_default();
    let catalog = SkillCatalog::load_from_with_cwd_plugins(&dir).unwrap_or_default();
    let overrides = scan_promoted_overrides();
    let _ = with_state_mut(|s| {
        s.catalog = catalog;
        s.reports.clear();
        s.promoted_overrides = overrides;
    });
    status_value()
}

// ── convert (raw runs → ExperiencePool) — no LLM ────────────────────────────

/// `POST /skilllens/convert {runs}` — normalise raw agent runs into an
/// [`ExperiencePool`] (JSONL). Pure; no LLM. The client persists the pool.
pub fn convert_value(runs_jsonl: &str) -> Result<Value, String> {
    let pool = lens_convert::convert_jsonl(runs_jsonl).map_err(|e| e.to_string())?;
    Ok(json!({ "pool": pool.to_jsonl(), "trajectories": pool.len() }))
}

// ── extract (pool → candidate skills) — LLM ─────────────────────────────────

/// `POST /skilllens/extract {pool, method, provider, model}` — distil a
/// candidate skill set from the experience pool. `method` is `"sequential"`
/// or `"parallel"` (default parallel).
pub async fn extract_value(
    pool_jsonl: &str,
    method: &str,
    provider: &str,
    model: &str,
) -> Result<Value, String> {
    let pool = ExperiencePool::from_jsonl(pool_jsonl).map_err(|e| e.to_string())?;
    let llm = adapter_from_selection(provider, model)
        .ok_or_else(|| format!("provider '{provider}' not configured (no key in ProfileStore)"))?;
    let skills: Vec<LensSkill> = match method {
        "sequential" => {
            let ex = lens_extract::sequential::SequentialExtractor::default();
            ex.extract(&pool, &llm).await.map_err(|e| e.to_string())?
        }
        _ => {
            let ex = lens_extract::parallel::ParallelExtractor::default();
            ex.extract(&pool, &llm).await.map_err(|e| e.to_string())?
        }
    };
    let rendered: Vec<Value> = skills
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "category": s.category,
                "triggers": s.triggers,
                "body": s.render(),
            })
        })
        .collect();
    Ok(json!({ "candidates": rendered, "method": method, "llm": llm.descriptor().model }))
}

// ── score (skill → SkillReport) — LLM ───────────────────────────────────────

/// `POST /skilllens/score {skill, tasks?, provider, model}` — measure a skill.
/// `target_evolvability` is computed against the supplied `tasks` (JSONL of
/// [`EvalTask`]); if omitted, the catalog-derived [`RepoAgentEnv`] tasks for
/// that skill are used. `extraction_efficacy` is `None` unless a pool is
/// supplied (Phase 3 measures evolvability + coverage).
pub async fn score_value(
    skill_name: &str,
    tasks_jsonl: Option<&str>,
    provider: &str,
    model: &str,
) -> Result<Value, String> {
    let skill = lens_skill_by_name(skill_name)
        .ok_or_else(|| format!("skill '{skill_name}' not in catalog"))?;
    let llm = adapter_from_selection(provider, model)
        .ok_or_else(|| format!("provider '{provider}' not configured (no key in ProfileStore)"))?;

    let tasks: Vec<EvalTask> = match tasks_jsonl {
        Some(s) if !s.trim().is_empty() => serde_json::Deserializer::from_str(s)
            .into_iter::<EvalTask>()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("parsing tasks: {e}"))?,
        _ => RepoAgentEnv::from_catalog_for(&skill_name).tasks(),
    };

    // Static (no-LLM) portion — always available.
    let mut report = SkillReport::measure_static(&skill, &skill.triggers);
    // LLM portion — target evolvability over the supplied/derived tasks.
    let ev = target_evolvability(&skill, &tasks, &llm)
        .await
        .map_err(|e| e.to_string())?;
    report.target_evolvability = Some(ev);

    // Cache it so the catalog list surfaces the score without re-running.
    let _ = with_state_mut(|s| {
        s.reports.insert(skill_name.to_string(), report.clone());
    });

    Ok(json!({
        "skill": skill_name,
        "report": report.to_json(),
        "llm": llm.descriptor(),
        "tasks": tasks.len(),
    }))
}

// ── RepoAgentEnv — Env over the repo's own skills / agent-job history ───────

/// The concrete [`OptEnv`] for the daemon. Two task sources:
///
/// - **Catalog** ([`RepoAgentEnv::from_catalog`] / [`from_catalog_for`]):
///   each skill becomes a task whose prompt asks the model to perform the
///   skill's first trigger and whose grader is [`Grader::Contains`] on a key
///   phrase drawn from the skill body. Deterministic, no extra LLM, exercises
///   the full `train()` loop against the repo's own skill library.
/// - **History** ([`RepoAgentEnv::from_history`]): one [`EvalTask`] per real
///   agent run, derived from the per-session [`SkillEvalRecord`] written at
///   the end of every agent run (see `vibe_ai::trace::TraceWriter::save_eval_record`).
///   The prompt is the session's first user message; the grader is either
///   [`Grader::LlmJudge`] (default — scores each rollout against the session's
///   reference final answer; one extra LLM call per task per epoch) or
///   [`Grader::Contains`] (free, weak — checks for a phrase from the reference
///   answer). Selected via `env.grader` on the train request.
///
/// [`from_catalog_for`]: RepoAgentEnv::from_catalog_for
pub struct RepoAgentEnv {
    tasks: Vec<EvalTask>,
}

impl RepoAgentEnv {
    /// Build tasks for the **whole** catalog (used when training a fresh
    /// skill against the repo's full task surface).
    pub fn from_catalog() -> Self {
        let tasks = with_state(|s| {
            s.catalog
                .all()
                .iter()
                .filter_map(|e| eval_task_for_skill(&e.name, e))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        Self { tasks }
    }

    /// Build tasks for a single named skill (used by `score` when no explicit
    /// task set is supplied). Falls back to a single synthetic task if the
    /// skill isn't found in the catalog.
    pub fn from_catalog_for(skill_name: &str) -> Self {
        let tasks = with_state(|s| {
            s.catalog
                .get(skill_name)
                .into_iter()
                .filter_map(|e| eval_task_for_skill(skill_name, e))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![EvalTask {
                id: skill_name.to_string(),
                prompt: format!("Follow the {skill_name} skill for its stated purpose."),
                grader: Grader::Contains(skill_name.to_string()),
            }]
        });
        Self { tasks }
    }

    /// Build from an explicit JSONL task set (passthrough to `StaticEnv` for
    /// callers that supply their own benchmark).
    pub fn from_jsonl(s: &str) -> anyhow::Result<Self> {
        let tasks: Vec<EvalTask> = serde_json::Deserializer::from_str(s)
            .into_iter::<EvalTask>()
            .collect::<Result<_, _>>()?;
        Ok(Self { tasks })
    }

    /// Build tasks from real agent-job history — one [`EvalTask`] per
    /// [`SkillEvalRecord`] (a prior agent run). The record's `prompt` (the
    /// session's first user message) becomes the task prompt; the grader is
    /// chosen by `grader` ([`HistoryGrader::LlmJudge`] scores against the
    /// reference final answer; [`HistoryGrader::Contains`] checks for a phrase
    /// from it). Records with an empty `prompt` or `final_answer` (errored /
    /// truncated runs) are skipped. Returns `Err` if no usable records remain.
    pub fn from_history(
        records: Vec<SkillEvalRecord>,
        grader: HistoryGrader,
    ) -> Result<Self, String> {
        let mut tasks = Vec::with_capacity(records.len());
        for r in records {
            if r.prompt.trim().is_empty() || r.final_answer.trim().is_empty() {
                continue;
            }
            let g = match grader {
                HistoryGrader::LlmJudge => Grader::LlmJudge(rubric_for(&r)),
                HistoryGrader::Contains => Grader::Contains(phrase_from(&r.final_answer)),
            };
            tasks.push(EvalTask {
                id: r.session_id.clone(),
                prompt: r.prompt,
                grader: g,
            });
        }
        if tasks.is_empty() {
            return Err(
                "no usable skill-eval records (all had an empty prompt or final answer)"
                    .to_string(),
            );
        }
        Ok(Self { tasks })
    }
}

/// How to grade `History`-env tasks derived from [`SkillEvalRecord`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryGrader {
    /// [`Grader::LlmJudge`] — scores each rollout against the session's
    /// reference final answer. Meaningful, but adds one LLM call per task
    /// per epoch (eval cost ≈ doubles).
    LlmJudge,
    /// [`Grader::Contains`] — free, weak: checks for a phrase drawn from the
    /// reference answer. Use for cheap smoke training.
    Contains,
}

/// Parse the `env.grader` string from a train request into a [`HistoryGrader`].
/// `None` (omitted) and `"llm_judge"` → [`HistoryGrader::LlmJudge`] (default);
/// `"contains"` → [`HistoryGrader::Contains`]; anything else → `Err`.
fn parse_history_grader(s: Option<&str>) -> Result<HistoryGrader, String> {
    match s.map(|x| x.to_ascii_lowercase()).as_deref() {
        None | Some("llm_judge") => Ok(HistoryGrader::LlmJudge),
        Some("contains") => Ok(HistoryGrader::Contains),
        Some(other) => Err(format!(
            "env.grader must be \"llm_judge\" or \"contains\", got {other:?}"
        )),
    }
}

/// Resolve the trace dir to scan for `<sess>-eval.json` records. `override_dir`
/// (the `env.tasks` field, repurposed for `History`) wins if it exists; else
/// fall back to `~/.vibecli/traces/` (where the CLI's `TraceWriter` writes).
fn history_trace_dir(override_dir: Option<&str>) -> PathBuf {
    if let Some(p) = override_dir {
        let path = PathBuf::from(p);
        if path.exists() {
            return path;
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".vibecli").join("traces"))
        .unwrap_or_else(|| PathBuf::from(".vibecli").join("traces"))
}

/// Build the [`Grader::LlmJudge`] rubric for one [`SkillEvalRecord`]: instructs
/// the judge to score the rollout 0.0–1.0 against the session's reference final
/// answer, with the tool-success rate + completion status as hints.
fn rubric_for(r: &SkillEvalRecord) -> String {
    let completed = if r.completed { "completed" } else { "partial" };
    let answer = truncate_chars(&r.final_answer, 1200);
    let rate_pct = r.tool_success_rate * 100.0;
    format!(
        "Score the response 0.0-1.0 on whether it accomplishes the user's task. \
         Reference: a prior agent run {completed} this task with the final answer below \
         (tool success rate {rate_pct:.0}%, {steps} steps).\n\n\
         --- Reference final answer ---\n{answer}\n--- End ---\n\n\
         Respond with a single number in [0, 1].",
        steps = r.steps,
    )
}

/// Extract a cheap `Contains` phrase from a reference answer: the first
/// non-empty, non-`#`-heading line, truncated to 80 chars. Falls back to the
/// first 80 chars of the answer. Intentionally weak — use [`HistoryGrader::LlmJudge`]
/// for meaningful grading.
fn phrase_from(answer: &str) -> String {
    let line = answer
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or(answer.trim());
    truncate_chars(line, 80)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    format!("{}…", &s[..end])
}

/// Construct one [`EvalTask`] from a catalog entry: the prompt asks the model
/// to act on the skill's first trigger; the grader is `Contains` on the first
/// meaningful phrase of the skill body (a cheap, deterministic signal that
/// the skill's guidance actually showed up in the rollout).
fn eval_task_for_skill(name: &str, e: &crate::skill_catalog::Skill) -> Option<EvalTask> {
    let trigger = e
        .frontmatter
        .triggers
        .first()
        .cloned()
        .unwrap_or_else(|| name.to_string());
    let phrase = e
        .body
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or(name)
        .to_string();
    Some(EvalTask {
        id: name.to_string(),
        prompt: format!("Use the {name} skill to: {trigger}."),
        grader: Grader::Contains(phrase),
    })
}

impl OptEnv for RepoAgentEnv {
    fn tasks(&self) -> Vec<EvalTask> {
        self.tasks.clone()
    }
}

// ── train (skill → best_skill.md) — LLM, async job ──────────────────────────

/// `POST /skillopt/train` request body.
#[derive(Debug, serde::Deserialize)]
pub struct TrainRequest {
    pub skill: String,
    /// `env`: `{kind: "static", tasks: "<jsonl>"}` or `{kind: "repo"}` or
    /// `{kind: "history", grader: "llm_judge"|"contains"}` (tasks optionally
    /// overrides the trace dir to scan).
    #[serde(default = "default_env_kind")]
    pub env: EnvSpec,
    /// Overrides for `TrainConfig` (all optional; defaults from `TrainConfig`).
    #[serde(default)]
    pub config: TrainConfigOverride,
    pub provider: String,
    pub model: String,
}

fn default_env_kind() -> EnvSpec {
    EnvSpec {
        kind: EnvKind::Repo,
        tasks: None,
        grader: None,
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct EnvSpec {
    #[serde(rename = "kind")]
    pub kind: EnvKind,
    /// Inline JSONL of `EvalTask`s (required when `kind == Static`). For
    /// `kind == History`, an optional override of the trace dir to scan for
    /// `<sess>-eval.json` records (defaults to `~/.vibecli/traces/`).
    pub tasks: Option<String>,
    /// History-only: grader for derived tasks — `"llm_judge"` (default,
    /// meaningful — scores each rollout against the session's reference
    /// answer; adds one LLM call per task per epoch) or `"contains"` (free,
    /// weak). Ignored for `Static`/`Repo`.
    #[serde(default)]
    pub grader: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EnvKind {
    Static,
    Repo,
    /// Derive `EvalTask`s from real agent-job history (`<sess>-eval.json`
    /// records written at the end of every agent run).
    History,
}

/// A `TrainConfig` subset that the panel may override. Unspecified fields
/// take `TrainConfig::default()`.
#[derive(Debug, Default, serde::Deserialize)]
pub struct TrainConfigOverride {
    #[serde(default)]
    pub epochs: Option<usize>,
    #[serde(default)]
    pub rollouts_per_epoch: Option<usize>,
    #[serde(default)]
    pub textual_lr: Option<usize>,
    #[serde(default)]
    pub val_split: Option<f32>,
    #[serde(default)]
    pub max_skill_tokens: Option<usize>,
    #[serde(default)]
    pub patience: Option<usize>,
    #[serde(default)]
    pub select_k: Option<usize>,
    #[serde(default)]
    pub seed: Option<u64>,
}

impl TrainConfigOverride {
    fn into_cfg(&self) -> TrainConfig {
        let mut c = TrainConfig::default();
        if let Some(v) = self.epochs {
            c.epochs = v;
        }
        if let Some(v) = self.rollouts_per_epoch {
            c.rollouts_per_epoch = v;
        }
        if let Some(v) = self.textual_lr {
            c.textual_lr = v;
        }
        if let Some(v) = self.val_split {
            c.val_split = v;
        }
        if let Some(v) = self.max_skill_tokens {
            c.max_skill_tokens = v;
        }
        if let Some(v) = self.patience {
            c.patience = v;
        }
        if let Some(v) = self.select_k {
            c.select_k = v;
        }
        if let Some(v) = self.seed {
            c.seed = v;
        }
        c
    }
}

/// Lifecycle of a training job. Surfaced via `GET /skillopt/status/:job`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase", tag = "state")]
pub enum TrainJobState {
    Running {
        started_at: u64,
    },
    Done {
        report: Value,
        started_at: u64,
        finished_at: u64,
    },
    Failed {
        error: String,
        started_at: u64,
        finished_at: u64,
    },
    /// Set by `POST /skillopt/cancel/:job`. `cancel_train_value` flips the
    /// job's [`CancelToken`] so the spawned task observes cancellation at
    /// the next epoch boundary and stops; its (partial) result is discarded.
    Cancelled {
        started_at: u64,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrainJob {
    pub id: String,
    pub skill: String,
    pub llm: LlmDescriptor,
    #[serde(flatten)]
    pub state: TrainJobState,
    /// Live cancellation handle — observed between epochs by the spawned
    /// training task. Not part of the HTTP surface (skipped over the wire).
    #[serde(skip)]
    pub cancel: CancelToken,
}

type JobMap = HashMap<String, TrainJob>;
static JOBS: OnceLock<Mutex<JobMap>> = OnceLock::new();

fn jobs() -> &'static Mutex<JobMap> {
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Build the [`OptEnv`] for a train request. `Static` → inline JSONL;
/// `Repo` → [`RepoAgentEnv::from_catalog`]; `History` → [`RepoAgentEnv::from_history`]
/// over the `<sess>-eval.json` records in the trace dir.
fn env_for(spec: &EnvSpec) -> Result<Box<dyn OptEnv>, String> {
    Ok(match spec.kind {
        EnvKind::Static => {
            let s = spec
                .tasks
                .as_deref()
                .ok_or("env.tasks (JSONL) required when env.kind=static")?;
            Box::new(StaticEnv::from_jsonl(s).map_err(|e| e.to_string())?)
        }
        EnvKind::Repo => Box::new(RepoAgentEnv::from_catalog()),
        EnvKind::History => {
            let grader = parse_history_grader(spec.grader.as_deref())?;
            let dir = history_trace_dir(spec.tasks.as_deref());
            let records = load_eval_records(&dir);
            if records.is_empty() {
                return Err(format!(
                    "no skill-eval records found in {} — run an agent first (the daemon writes \
                     <session>-eval.json at the end of every run)",
                    dir.display()
                ));
            }
            Box::new(RepoAgentEnv::from_history(records, grader)?)
        }
    })
}

/// `POST /skillopt/train` — spawn a training run and return its job id.
/// The run executes on a tokio task; poll `GET /skillopt/status/:job` or
/// stream `POST /v1/skillopt/train/stream` for live per-epoch events.
pub async fn spawn_train(req: &TrainRequest) -> Result<Value, String> {
    let prep = prepare_run(req)?;
    Ok(launch_run(prep, None).await)
}

/// `POST /v1/skillopt/train/stream` — like [`spawn_train`] but returns a
/// live [`EpochEvent`] stream alongside the job id. The caller (the SSE
/// handler in `serve.rs`) drains the receiver and forwards each event;
/// the run registers in the same `JOBS` map, so `cancel/:job` and
/// `status/:job` work identically to the non-streaming path.
pub async fn spawn_train_streaming(
    req: &TrainRequest,
) -> Result<(Value, tokio::sync::mpsc::Receiver<EpochEvent>), String> {
    let prep = prepare_run(req)?;
    let (tx, rx) = tokio::sync::mpsc::channel::<EpochEvent>(64);
    let json = launch_run(prep, Some(tx)).await;
    Ok((json, rx))
}

/// Everything needed to launch a run, derived from a [`TrainRequest`] with
/// no IO side effects (no `JOBS` mutation) so the streaming + non-streaming
/// entry points share one validation + construction path.
struct RunPrep {
    id: String,
    started_at: u64,
    skill_name: String,
    skill: LensSkill,
    env: Box<dyn OptEnv>,
    llm: Arc<dyn SkillLlm>,
    cfg: TrainConfig,
    descriptor: LlmDescriptor,
}

fn prepare_run(req: &TrainRequest) -> Result<RunPrep, String> {
    let skill = lens_skill_by_name(&req.skill)
        .ok_or_else(|| format!("skill '{}' not in catalog", req.skill))?;
    let llm = adapter_from_selection(&req.provider, &req.model)
        .ok_or_else(|| format!("provider '{}' not configured", req.provider))?;
    let env = env_for(&req.env)?;
    let cfg = req.config.into_cfg();

    // Deterministic-ish job id: skill + provider+model + seed. Collisions
    // (same triple re-submitted) overwrite the prior job — intended.
    let id = format!("{}-{}-{}-{}", req.skill, req.provider, req.model, cfg.seed);
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let descriptor = llm.descriptor();

    Ok(RunPrep {
        id,
        started_at,
        skill_name: req.skill.clone(),
        skill,
        env,
        llm: Arc::new(llm),
        cfg,
        descriptor,
    })
}

/// Register the job as `Running` and spawn the training task. `progress`
/// is forwarded to `train_with_signals` so the streaming path receives
/// one [`EpochEvent`] per epoch; `None` for the non-streaming path. Returns
/// the `job_id` JSON the HTTP boundary hands back to the client.
async fn launch_run(
    prep: RunPrep,
    progress: Option<tokio::sync::mpsc::Sender<EpochEvent>>,
) -> Value {
    let RunPrep {
        id,
        started_at,
        skill_name,
        skill,
        env,
        llm,
        cfg,
        descriptor,
    } = prep;
    let cancel = CancelToken::new();
    let job = TrainJob {
        id: id.clone(),
        skill: skill_name.clone(),
        llm: descriptor.clone(),
        state: TrainJobState::Running { started_at },
        cancel: cancel.clone(),
    };
    jobs().lock().await.insert(id.clone(), job);

    let id_spawn = id.clone();
    let cancel_spawn = cancel.clone();
    tokio::spawn(async move {
        let signals = TrainSignals {
            cancel: Some(&cancel_spawn),
            progress: progress.as_ref(),
        };
        let result = train_with_signals(skill, env.as_ref(), llm.as_ref(), &cfg, signals).await;
        let finished_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut map = jobs().lock().await;
        let Some(job) = map.get_mut(&id_spawn) else {
            return;
        };
        // Late external cancel (state already Cancelled) — discard result.
        if matches!(job.state, TrainJobState::Cancelled { .. }) {
            return;
        }
        match result {
            Ok(report) if report.cancelled => {
                // Train observed the cancel token mid-run.
                job.state = TrainJobState::Cancelled { started_at };
            }
            Ok(report) => {
                job.state = TrainJobState::Done {
                    report: report_to_value(&report),
                    started_at,
                    finished_at,
                };
            }
            Err(e) => {
                job.state = TrainJobState::Failed {
                    error: e.to_string(),
                    started_at,
                    finished_at,
                };
            }
        }
        let _ = skill_name; // provenance kept for future per-skill job pages
    });

    json!({ "job_id": id, "status": "running", "llm": descriptor })
}

/// `GET /skillopt/status/:job` — current job state. `404` if unknown.
pub async fn train_status_value(job_id: &str) -> Option<Value> {
    let map = jobs().lock().await;
    map.get(job_id)
        .map(|j| serde_json::to_value(j).unwrap_or_else(|_| json!({"id": j.id})))
}

/// `POST /skillopt/cancel/:job` — request cancellation of a running job.
/// Flips the job's [`CancelToken`] (observed at the next epoch boundary by
/// the spawned task) and marks state `Cancelled`. Best-effort: a job that
/// has already finished is reported `not running`.
pub async fn cancel_train_value(job_id: &str) -> Value {
    let mut map = jobs().lock().await;
    match map.get_mut(job_id) {
        Some(job) => {
            if matches!(job.state, TrainJobState::Running { .. }) {
                let started_at = if let TrainJobState::Running { started_at } = job.state {
                    started_at
                } else {
                    0
                };
                // Signal the running task to stop at the next epoch boundary.
                job.cancel.cancel();
                job.state = TrainJobState::Cancelled { started_at };
                json!({ "job_id": job_id, "cancelled": true })
            } else {
                json!({ "job_id": job_id, "cancelled": false, "reason": "not running" })
            }
        }
        None => json!({ "job_id": job_id, "error": "no such job" }),
    }
}

/// Compact list of all known train jobs for the TUI SkillForge screen's
/// train-status pane (G2). No HTTP route — this is read directly by the
/// in-process TUI component (the daemon's own client), mirroring how the
/// Goals screen reads `SessionStore` directly. Each row is
/// `{id, skill, state, llm}` with a short human-readable `state` label.
pub async fn list_jobs_value() -> Value {
    let map = jobs().lock().await;
    let mut rows: Vec<Value> = map
        .values()
        .map(|j| {
            let state = match &j.state {
                TrainJobState::Running { .. } => "running",
                TrainJobState::Done { .. } => "done",
                TrainJobState::Failed { .. } => "failed",
                TrainJobState::Cancelled { .. } => "cancelled",
            };
            json!({
                "id": j.id,
                "skill": j.skill,
                "state": state,
                "llm": format!("{}/{}", j.llm.provider, j.llm.model),
            })
        })
        .collect();
    // Deterministic order: newest-feeling by id desc isn't meaningful
    // without timestamps on every variant, so sort by skill then id.
    rows.sort_by(|a, b| {
        a["skill"]
            .as_str()
            .unwrap_or("")
            .cmp(b["skill"].as_str().unwrap_or(""))
            .then_with(|| {
                a["id"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["id"].as_str().unwrap_or(""))
            })
    });
    json!(rows)
}

/// Render a [`TrainingReport`] as JSON for the HTTP boundary (no crate type
/// leaks). Includes the deployable `best_skill_md` artifact.
pub fn report_to_value(report: &TrainingReport) -> Value {
    json!({
        "skill_name": report.skill_name,
        "epochs_run": report.epochs_run,
        "best_val_score": report.best_val_score,
        "val_curve": report.val_curve,
        "accepted": report.accepted,
        "rejected": report.rejected,
        "final_tokens": report.final_tokens,
        "spent_tokens": report.spent_tokens,
        "early_stopped": report.early_stopped,
        "cancelled": report.cancelled,
        "best_skill_md": report.best_skill_md,
    })
}

// ── promote (write *.opt.md to the override dir) — explicit, audited ──────────

/// Write a promoted skill body to `<dir>/<skill>.opt.md`, creating `dir` if
/// missing. Pure (no STATE) so the path + creation logic is unit-testable
/// with a tempdir. Used by [`promote_value`].
fn write_promoted_override_in(
    skill_name: &str,
    content: &str,
    dir: &Path,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let opt_path = dir.join(format!("{skill_name}.opt.md"));
    std::fs::write(&opt_path, content).map_err(|e| e.to_string())?;
    Ok(opt_path)
}

/// `POST /skillopt/promote {skill, content}` — write `<skill>.opt.md` to the
/// per-workspace override dir (`<ws>/.vibecli/skills/`). **Never** overwrites
/// the shipped `skills/*.md` tree — promoted artifacts land in the override
/// dir so the 710 shipped skills stay pristine. The panel requires a separate
/// human action to swap a promoted artifact into the live loader (ties into
/// the patent-audit rule about surfacing AI output). Returns the written path.
pub fn promote_value(skill_name: &str, content: &str) -> Result<Value, String> {
    // Precondition: the skill must exist in the catalog (don't write an
    // override for an unknown name).
    let _shipped_path = with_state(|s| s.catalog.get(skill_name).map(|e| e.path.clone()))
        .ok_or_else(|| "skillforge catalog not initialized".to_string())?
        .ok_or_else(|| format!("skill '{skill_name}' not in catalog"))?;
    let dir = promote_dir_for(None);
    let opt_path = write_promoted_override_in(skill_name, content, &dir)?;
    // Update the override cache so the panel sees it without a refresh.
    let _ = with_state_mut(|s| {
        s.promoted_overrides
            .insert(skill_name.to_string(), opt_path.clone());
    });
    Ok(json!({
        "skill": skill_name,
        "written": opt_path.display().to_string(),
        "dir": dir.display().to_string(),
        "note": "promoted to the per-workspace override dir (<ws>/.vibecli/skills/*.opt.md) — the shipped skills/*.md tree is untouched; swap into the live loader deliberately.",
    }))
}

// ── HTTP request types + `do_v1_*` wrappers (called by serve.rs handlers) ──
//
// Pure wrappers returning `(StatusCode, Value)` so the axum handlers in
// `serve.rs` stay trivial one-liners (same split as `graph_index`). No
// SkillForge crate type crosses the HTTP boundary — only `serde_json::Value`.

#[derive(Debug, serde::Deserialize)]
pub struct ConvertRequest {
    /// Raw agent runs as JSONL (`RawRun` per line).
    pub runs: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExtractRequest {
    /// Experience pool as JSONL (`Trajectory` per line).
    pub pool: String,
    /// `"sequential"` or `"parallel"` (default parallel).
    #[serde(default)]
    pub method: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScoreRequest {
    pub skill: String,
    /// Optional JSONL of `EvalTask`s. If omitted, catalog-derived tasks are used.
    #[serde(default)]
    pub tasks: Option<String>,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct PromoteRequest {
    pub skill: String,
    /// The trained skill markdown (the `best_skill_md` artifact).
    pub content: String,
}

use axum::http::StatusCode;

fn ok(v: Value) -> (StatusCode, Value) {
    (StatusCode::OK, v)
}
fn bad(msg: impl Into<String>) -> (StatusCode, Value) {
    (StatusCode::BAD_REQUEST, json!({ "error": msg.into() }))
}

pub(crate) fn do_v1_skilllens_skills() -> (StatusCode, Value) {
    ok(list_skills_value())
}

pub(crate) fn do_v1_skilllens_skill(name: &str) -> (StatusCode, Value) {
    match get_skill_value(name) {
        Some(v) => ok(v),
        None => (
            StatusCode::NOT_FOUND,
            json!({ "error": format!("skill '{name}' not found") }),
        ),
    }
}

pub(crate) fn do_v1_skilllens_refresh() -> (StatusCode, Value) {
    ok(refresh_value())
}

pub(crate) fn do_v1_skilllens_convert(req: &ConvertRequest) -> (StatusCode, Value) {
    match convert_value(&req.runs) {
        Ok(v) => ok(v),
        Err(e) => bad(e),
    }
}

pub(crate) async fn do_v1_skilllens_extract(req: &ExtractRequest) -> (StatusCode, Value) {
    let method = if req.method.is_empty() {
        "parallel"
    } else {
        req.method.as_str()
    };
    match extract_value(&req.pool, method, &req.provider, &req.model).await {
        Ok(v) => ok(v),
        Err(e) => bad(e),
    }
}

pub(crate) async fn do_v1_skilllens_score(req: &ScoreRequest) -> (StatusCode, Value) {
    match score_value(&req.skill, req.tasks.as_deref(), &req.provider, &req.model).await {
        Ok(v) => ok(v),
        Err(e) => bad(e),
    }
}

pub(crate) async fn do_v1_skillopt_train(req: &TrainRequest) -> (StatusCode, Value) {
    match spawn_train(req).await {
        Ok(v) => ok(v),
        Err(e) => bad(e),
    }
}

pub(crate) async fn do_v1_skillopt_status(job: &str) -> (StatusCode, Value) {
    match train_status_value(job).await {
        Some(v) => ok(v),
        None => (
            StatusCode::NOT_FOUND,
            json!({ "error": format!("no job '{job}'") }),
        ),
    }
}

pub(crate) async fn do_v1_skillopt_cancel(job: &str) -> (StatusCode, Value) {
    ok(cancel_train_value(job).await)
}

pub(crate) fn do_v1_skillopt_promote(req: &PromoteRequest) -> (StatusCode, Value) {
    match promote_value(&req.skill, &req.content) {
        Ok(v) => ok(v),
        Err(e) => bad(e),
    }
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use skilllensai::llm::NullLlm;

    #[test]
    fn status_disabled_before_init() {
        // Before init, status is Disabled (or whatever the OnceLock holds).
        // We can't assert globally across tests, so just check the str form.
        let s = SkillForgeStatus::Disabled;
        assert_eq!(s.as_str(), "disabled");
        assert_eq!(SkillForgeStatus::Ready.as_str(), "ready");
        assert_eq!(SkillForgeStatus::Loading.as_str(), "loading");
    }

    /// G3 — `render_health_line` must auto-gate to `None` when no skills
    /// have been scored, so the agent system prompt is not bloated for
    /// users who never ran SkillLens. When the index hasn't been
    /// initialised (or has zero cached reports), `with_state` returns
    /// `None` → `(skills, cached) = (0, 0)` → `None`. This is the
    /// no-bloat contract the G3 deviation from note 07 relies on.
    #[test]
    fn render_health_line_is_none_when_nothing_scored() {
        // No init in this test → STATE is unset → with_state returns None
        // → cached == 0 → the auto-gate returns None.
        assert!(render_health_line().is_none());
    }

    #[test]
    fn repo_env_from_catalog_for_unknown_skill_yields_synthetic_task() {
        // No catalog initialized in this test → from_catalog_for falls back.
        let env = RepoAgentEnv::from_catalog_for("does-not-exist");
        let tasks = env.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "does-not-exist");
    }

    #[tokio::test]
    async fn convert_value_roundtrips_empty_pool() {
        // An empty JSONL → empty pool (0 trajectories), no LLM.
        let v = convert_value("").unwrap();
        assert_eq!(v["trajectories"], 0);
    }

    #[tokio::test]
    async fn convert_value_rejects_garbage() {
        assert!(convert_value("not json").is_err());
    }

    #[test]
    fn train_config_override_applies_overrides() {
        let o = TrainConfigOverride {
            epochs: Some(2),
            seed: Some(7),
            ..Default::default()
        };
        let c = o.into_cfg();
        assert_eq!(c.epochs, 2);
        assert_eq!(c.seed, 7);
        // untouched fields keep defaults
        let d = TrainConfig::default();
        assert_eq!(c.patience, d.patience);
        assert_eq!(c.val_split, d.val_split);
    }

    #[tokio::test]
    async fn spawn_train_fails_for_unknown_skill() {
        let req = TrainRequest {
            skill: "no-such-skill".into(),
            env: default_env_kind(),
            config: TrainConfigOverride::default(),
            provider: "ollama".into(),
            model: "qwen3".into(),
        };
        assert!(spawn_train(&req).await.is_err());
    }

    #[tokio::test]
    async fn spawn_train_fails_for_unconfigured_provider() {
        // Use a skill name that may exist; the provider check fires first
        // only if the skill resolves. Force the provider error by picking a
        // provider nobody has configured.
        let req = TrainRequest {
            skill: "any".into(),
            env: default_env_kind(),
            config: TrainConfigOverride::default(),
            provider: "definitely-not-a-provider".into(),
            model: "x".into(),
        };
        // Either the skill lookup fails or the provider lookup fails — both
        // are "Err". Asserting Err (not the specific message) is robust.
        assert!(spawn_train(&req).await.is_err());
    }

    #[tokio::test]
    async fn cancel_unknown_job_is_reported() {
        let v = cancel_train_value("no-such-job").await;
        assert!(v.get("error").is_some());
    }

    /// `cancel_train_value` flips the job's `CancelToken` (so the running
    /// task observes cancellation at the next epoch boundary) and marks the
    /// state `Cancelled`. Insert a synthetic `Running` job to exercise the
    /// path without a configured LLM provider.
    #[tokio::test]
    async fn cancel_flips_running_jobs_token() {
        let id = "test-cancel-token-flip";
        let token = CancelToken::new();
        let job = TrainJob {
            id: id.into(),
            skill: "demo".into(),
            llm: LlmDescriptor {
                provider: "mock".into(),
                model: "mock-1".into(),
            },
            state: TrainJobState::Running { started_at: 0 },
            cancel: token.clone(),
        };
        jobs().lock().await.insert(id.into(), job);

        assert!(!token.is_cancelled(), "token fresh");
        let v = cancel_train_value(id).await;
        assert_eq!(v["cancelled"], true);
        assert!(
            token.is_cancelled(),
            "cancel_train_value must flip the live token"
        );

        // Second cancel on the now-Cancelled job is a no-op (not running).
        let v2 = cancel_train_value(id).await;
        assert_eq!(v2["cancelled"], false);
        assert!(v2["reason"].as_str().unwrap_or("").contains("not running"));

        // Cleanup so it doesn't leak across tests sharing the process map.
        jobs().lock().await.remove(id);
    }

    /// A `Done` job is not cancellable — `cancel_train_value` reports
    /// `not running` and leaves the state + token untouched.
    #[tokio::test]
    async fn cancel_is_noop_for_done_job() {
        let id = "test-cancel-done-noop";
        let token = CancelToken::new();
        let job = TrainJob {
            id: id.into(),
            skill: "demo".into(),
            llm: LlmDescriptor {
                provider: "mock".into(),
                model: "mock-1".into(),
            },
            state: TrainJobState::Done {
                report: json!({ "skill_name": "demo" }),
                started_at: 0,
                finished_at: 1,
            },
            cancel: token.clone(),
        };
        jobs().lock().await.insert(id.into(), job);

        let v = cancel_train_value(id).await;
        assert_eq!(v["cancelled"], false);
        assert!(!token.is_cancelled(), "done job must not flip token");
        jobs().lock().await.remove(id);
    }

    // ── promoted-skill override dir ─────────────────────────────────────────

    #[test]
    fn promote_dir_for_explicit_workspace_is_vibecli_skills() {
        let ws = std::path::PathBuf::from("/tmp/test-ws");
        let dir = promote_dir_for(Some(&ws));
        assert_eq!(
            dir,
            std::path::PathBuf::from("/tmp/test-ws/.vibecli/skills")
        );
    }

    #[test]
    fn write_promoted_override_creates_dir_and_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("override"); // does not exist yet
        let path = write_promoted_override_in("rust-tests", "trained body", &dir).unwrap();
        assert!(dir.is_dir(), "override dir is created on demand");
        assert_eq!(path.file_name().unwrap(), "rust-tests.opt.md");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "trained body");
    }

    #[test]
    fn scan_promoted_overrides_keys_by_stem_and_ignores_non_opt() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rust-tests.opt.md"), "a").unwrap();
        std::fs::write(dir.join("formal-verification.opt.md"), "b").unwrap();
        // A plain shipped-style skill — not an override, must be skipped.
        std::fs::write(dir.join("plain-skill.md"), "c").unwrap();
        // A README — not markdown-with-.opt.
        std::fs::write(dir.join("README.txt"), "d").unwrap();

        let map = scan_promoted_overrides_in(&dir);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("rust-tests"));
        assert!(map.contains_key("formal-verification"));
        assert!(
            !map.contains_key("plain-skill"),
            "plain .md is not an override"
        );
    }

    #[test]
    fn scan_promoted_overrides_missing_dir_is_empty() {
        let map = scan_promoted_overrides_in(std::path::Path::new("/nonexistent-vibe-test-123"));
        assert!(
            map.is_empty(),
            "missing override dir → empty map, not an error"
        );
    }

    #[test]
    fn null_llm_descriptor_and_error_path() {
        let n = NullLlm;
        // descriptor is synchronous — confirm the trait is in scope.
        assert_eq!(n.descriptor().provider, "null");
    }

    #[test]
    fn report_to_value_shape() {
        let report = TrainingReport {
            skill_name: "demo".into(),
            epochs_run: 3,
            best_val_score: 0.66,
            val_curve: vec![0.5, 0.6, 0.66],
            accepted: 2,
            rejected: 1,
            final_tokens: 1200,
            spent_tokens: 9000,
            best_skill_md: "---\n---\n# demo".into(),
            early_stopped: false,
            cancelled: false,
        };
        let v = report_to_value(&report);
        assert_eq!(v["skill_name"], "demo");
        assert_eq!(v["accepted"], 2);
        // f32 → JSON f64 rounding: compare with tolerance, not exact eq.
        let last = v["val_curve"][2].as_f64().unwrap();
        assert!((last - 0.66).abs() < 1e-5);
    }

    // ── History env (real agent-job history → EvalTask) ─────────────────────

    fn eval_record(prompt: &str, answer: &str, completed: bool) -> SkillEvalRecord {
        SkillEvalRecord {
            timestamp: 1700000000,
            session_id: "1700000000".to_string(),
            prompt: prompt.to_string(),
            final_answer: answer.to_string(),
            tool_success_rate: 0.75,
            steps: 4,
            completed,
        }
    }

    #[test]
    fn from_history_llm_judge_builds_rubric_per_record() {
        let records = vec![
            SkillEvalRecord {
                session_id: "sess-a".into(),
                prompt: "Refactor the auth module".into(),
                final_answer: "Extracted a `TokenValidator` and added tests".into(),
                timestamp: 1,
                tool_success_rate: 1.0,
                steps: 5,
                completed: true,
            },
            SkillEvalRecord {
                session_id: "sess-b".into(),
                prompt: "Fix the flaky CI test".into(),
                final_answer: "Added a retry guard around the network call".into(),
                timestamp: 2,
                tool_success_rate: 0.5,
                steps: 3,
                completed: false,
            },
        ];
        let env = RepoAgentEnv::from_history(records, HistoryGrader::LlmJudge).unwrap();
        let tasks = env.tasks();
        assert_eq!(tasks.len(), 2, "one task per record");
        assert_eq!(tasks[0].id, "sess-a");
        assert_eq!(tasks[0].prompt, "Refactor the auth module");
        match &tasks[0].grader {
            Grader::LlmJudge(rubric) => {
                assert!(
                    rubric.contains("Reference final answer"),
                    "rubric cites the reference"
                );
                assert!(
                    rubric.contains("TokenValidator"),
                    "rubric embeds the reference answer"
                );
                assert!(rubric.contains("100%"), "rubric includes tool success rate");
            }
            other => panic!("expected LlmJudge, got {other:?}"),
        }
        // Partial run → rubric flags partial.
        match &tasks[1].grader {
            Grader::LlmJudge(rubric) => assert!(rubric.contains("partial")),
            _ => panic!("expected LlmJudge for second task"),
        }
    }

    #[test]
    fn from_history_contains_extracts_phrase() {
        let env = RepoAgentEnv::from_history(
            vec![eval_record(
                "Summarise the meeting",
                "## Notes\nAction items: file ticket, ping Alice.\n",
                true,
            )],
            HistoryGrader::Contains,
        )
        .unwrap();
        let tasks = env.tasks();
        assert_eq!(tasks.len(), 1);
        match &tasks[0].grader {
            Grader::Contains(phrase) => {
                // First non-empty, non-`#` line is the action-items line.
                assert!(phrase.contains("Action items"), "phrase = {phrase}");
                assert!(phrase.chars().count() <= 80, "phrase is truncated");
            }
            other => panic!("expected Contains, got {other:?}"),
        }
    }

    #[test]
    fn from_history_skips_records_with_empty_prompt_or_answer() {
        let records = vec![
            SkillEvalRecord {
                session_id: "empty-prompt".into(),
                prompt: "   ".into(),
                final_answer: "has answer".into(),
                timestamp: 1,
                tool_success_rate: 1.0,
                steps: 1,
                completed: true,
            },
            SkillEvalRecord {
                session_id: "empty-answer".into(),
                prompt: "has prompt".into(),
                final_answer: String::new(),
                timestamp: 2,
                tool_success_rate: 1.0,
                steps: 1,
                completed: true,
            },
            eval_record("kept", "kept answer", true),
        ];
        let env = RepoAgentEnv::from_history(records, HistoryGrader::Contains).unwrap();
        let tasks = env.tasks();
        assert_eq!(tasks.len(), 1, "only the complete record survives");
        assert_eq!(tasks[0].id, "1700000000");
    }

    #[test]
    fn from_history_all_unusable_is_err() {
        let err = RepoAgentEnv::from_history(
            vec![SkillEvalRecord {
                session_id: "s".into(),
                prompt: String::new(),
                final_answer: String::new(),
                timestamp: 0,
                tool_success_rate: 0.0,
                steps: 0,
                completed: false,
            }],
            HistoryGrader::LlmJudge,
        )
        .err()
        .expect("expected an error when all records are unusable");
        assert!(err.contains("no usable skill-eval records"), "err = {err}");
    }

    #[test]
    fn parse_history_grader_defaults_and_rejects() {
        assert_eq!(parse_history_grader(None).unwrap(), HistoryGrader::LlmJudge);
        assert_eq!(
            parse_history_grader(Some("llm_judge")).unwrap(),
            HistoryGrader::LlmJudge
        );
        assert_eq!(
            parse_history_grader(Some("contains")).unwrap(),
            HistoryGrader::Contains
        );
        // Case-insensitive.
        assert_eq!(
            parse_history_grader(Some("Contains")).unwrap(),
            HistoryGrader::Contains
        );
        assert!(parse_history_grader(Some("bogus")).is_err());
    }

    #[test]
    fn history_trace_dir_uses_override_when_it_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = history_trace_dir(Some(tmp.path().to_str().unwrap()));
        assert_eq!(dir, tmp.path());
    }

    #[test]
    fn history_trace_dir_falls_back_to_home_when_override_missing() {
        // Override path that does not exist → fall back to ~/.vibecli/traces.
        let dir = history_trace_dir(Some("/nonexistent-vibe-history-test-9999"));
        assert!(
            dir.ends_with(std::path::Path::new(".vibecli/traces")),
            "expected ~/.vibecli/traces fallback, got {}",
            dir.display()
        );
    }

    #[test]
    fn env_for_history_reads_eval_records_from_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let rec = SkillEvalRecord {
            session_id: "1800000000".into(),
            prompt: "Write a migration".into(),
            final_answer: "Created `migrate_002.sql` and ran it".into(),
            timestamp: 1800000000,
            tool_success_rate: 1.0,
            steps: 2,
            completed: true,
        };
        std::fs::write(
            tmp.path().join("1800000000-eval.json"),
            serde_json::to_string_pretty(&rec).unwrap(),
        )
        .unwrap();

        let spec = EnvSpec {
            kind: EnvKind::History,
            tasks: Some(tmp.path().to_str().unwrap().to_string()),
            grader: Some("contains".into()),
        };
        let env = env_for(&spec).unwrap();
        let tasks = env.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].prompt, "Write a migration");
        assert!(matches!(tasks[0].grader, Grader::Contains(_)));
    }

    #[test]
    fn env_for_history_no_records_is_err() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = EnvSpec {
            kind: EnvKind::History,
            tasks: Some(tmp.path().to_str().unwrap().to_string()),
            grader: None,
        };
        let err = env_for(&spec)
            .err()
            .expect("expected an error for no records");
        assert!(err.contains("no skill-eval records"), "err = {err}");
    }
}
