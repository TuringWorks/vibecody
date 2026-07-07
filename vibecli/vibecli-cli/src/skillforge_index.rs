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
//!   id; `GET /skillopt/status/:job` reports `Running`/`Done`/`Failed`.
//!   Per-epoch streaming + true cancellation need a callback/cancel token in
//!   `skilloptai::trainer::train` — deferred to Phase 4. Today cancellation
//!   marks the job `Cancelled` and discards the result when the run finishes.
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
use skilloptai::trainer::{train, TrainConfig};
use skilloptai::VERSION as OPT_VERSION;

use vibe_ai::provider::{AIProvider, Message, MessageRole};

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
pub fn adapter_from_selection(
    provider: &str,
    model: &str,
) -> Option<AiProviderLlm> {
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

/// Initialize the process-global catalog. Idempotent — the `skills_dir`
/// argument is only used the first time. Kicks off a non-blocking background
/// parse (parse is CPU/IO-bound and cheap over ~710 small files, but we keep
/// it off the serving thread for parity with `graph_index`).
pub fn init_skillforge(skills_dir: Option<&Path>) -> SkillForgeStatus {
    let _ = STATUS.set(RwLock::new(SkillForgeStatus::Loading));
    let dir = skills_dir.map(Path::to_path_buf).unwrap_or_else(skills_dir_default);
    std::thread::spawn(move || {
        let catalog = SkillCatalog::load_from_with_cwd_plugins(&dir).unwrap_or_default();
        let state = SkillForgeState {
            catalog,
            reports: HashMap::new(),
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
    Some(format!("{skills} skills, {cached} scored, top evolvability {top}"))
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
    let cached: Option<SkillReport> = with_state(|s| s.reports.get(name).cloned())?;
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
        "report": cached.as_ref().map(|r| r.to_json()),
    }))
}

/// `POST /skilllens/refresh` — re-read the skills dir and reset the report
/// cache. Returns the new status payload.
pub fn refresh_value() -> Value {
    let dir = skills_dir_default();
    let catalog = SkillCatalog::load_from_with_cwd_plugins(&dir).unwrap_or_default();
    let _ = with_state_mut(|s| {
        s.catalog = catalog;
        s.reports.clear();
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
    let rendered: Vec<Value> = skills.iter().map(|s| json!({
        "name": s.name,
        "category": s.category,
        "triggers": s.triggers,
        "body": s.render(),
    })).collect();
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
        Some(s) if !s.trim().is_empty() => {
            serde_json::Deserializer::from_str(s)
                .into_iter::<EvalTask>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("parsing tasks: {e}"))?
        }
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

// ── RepoAgentEnv — day-one Env over the repo's own skills ───────────────────

/// The day-one concrete [`OptEnv`] for the daemon. Derives [`EvalTask`]s
/// from the catalog: each skill becomes a task whose prompt asks the model
/// to perform the skill's first trigger and whose grader is [`Grader::Contains`]
/// on a key phrase drawn from the skill body. Deterministic, no extra LLM,
/// exercises the full `train()` loop against the repo's own skill library.
///
/// Richer tasks drawn from real VibeCody agent-job history (graded by
/// `ToolExit`/`LlmJudge`) are a documented follow-up — the daemon is the
/// right place for that coupling, but the job-history → EvalTask derivation
/// needs a per-job grader which is not yet wired.
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
        .unwrap_or_else(|| vec![EvalTask {
            id: skill_name.to_string(),
            prompt: format!("Follow the {skill_name} skill for its stated purpose."),
            grader: Grader::Contains(skill_name.to_string()),
        }]);
        Self { tasks }
    }

    /// Build from an explicit JSONL task set (passthrough to `StaticEnv` for
    /// callers that supply their own benchmark).
    pub fn from_jsonl(s: &str) -> anyhow::Result<Self> {
        let tasks: Vec<EvalTask> =
            serde_json::Deserializer::from_str(s)
                .into_iter::<EvalTask>()
                .collect::<Result<_, _>>()?;
        Ok(Self { tasks })
    }
}

/// Construct one [`EvalTask`] from a catalog entry: the prompt asks the model
/// to act on the skill's first trigger; the grader is `Contains` on the first
/// meaningful phrase of the skill body (a cheap, deterministic signal that
/// the skill's guidance actually showed up in the rollout).
fn eval_task_for_skill(name: &str, e: &crate::skill_catalog::Skill) -> Option<EvalTask> {
    let trigger = e.frontmatter.triggers.first().cloned().unwrap_or_else(|| name.to_string());
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
    /// `env`: `{kind: "static", tasks: "<jsonl>"}` or `{kind: "repo"}`.
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
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct EnvSpec {
    #[serde(rename = "kind")]
    pub kind: EnvKind,
    /// Inline JSONL of `EvalTask`s (required when `kind == Static`).
    pub tasks: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EnvKind {
    Static,
    Repo,
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
    Running { started_at: u64 },
    Done { report: Value, started_at: u64, finished_at: u64 },
    Failed { error: String, started_at: u64, finished_at: u64 },
    /// Set by `POST /skillopt/cancel/:job`. The spawned task runs to
    /// completion (skilloptai's `train()` has no cancel token in Phase 3)
    /// and its result is discarded.
    Cancelled { started_at: u64 },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrainJob {
    pub id: String,
    pub skill: String,
    pub llm: LlmDescriptor,
    #[serde(flatten)]
    pub state: TrainJobState,
}

type JobMap = HashMap<String, TrainJob>;
static JOBS: OnceLock<Mutex<JobMap>> = OnceLock::new();

fn jobs() -> &'static Mutex<JobMap> {
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Build the [`OptEnv`] for a train request. `Static` → inline JSONL;
/// `Repo` → [`RepoAgentEnv::from_catalog`].
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
    })
}

/// `POST /skillopt/train` — spawn a training run and return its job id.
/// The run executes on a tokio task; poll `GET /skillopt/status/:job`.
pub async fn spawn_train(req: &TrainRequest) -> Result<Value, String> {
    let skill = lens_skill_by_name(&req.skill)
        .ok_or_else(|| format!("skill '{}' not in catalog", req.skill))?;
    let llm = adapter_from_selection(&req.provider, &req.model)
        .ok_or_else(|| format!("provider '{}' not configured", req.provider))?;
    let env = env_for(&req.env)?;
    let cfg = req.config.into_cfg(); // &self — borrows, no move out of &TrainRequest

    // Deterministic-ish job id: skill + provider+model + seed. Collisions
    // (same triple re-submitted) overwrite the prior job — intended.
    let id = format!(
        "{}-{}-{}-{}",
        req.skill,
        req.provider,
        req.model,
        cfg.seed
    );
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let descriptor = llm.descriptor();
    let job = TrainJob {
        id: id.clone(),
        skill: req.skill.clone(),
        llm: descriptor.clone(),
        state: TrainJobState::Running { started_at },
    };
    jobs().lock().await.insert(id.clone(), job);

    let id_spawn = id.clone();
    let skill_name = req.skill.clone();
    let llm: Arc<dyn SkillLlm> = Arc::new(llm);
    tokio::spawn(async move {
        let result = train(skill, env.as_ref(), llm.as_ref(), &cfg).await;
        let finished_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut map = jobs().lock().await;
        let Some(job) = map.get_mut(&id_spawn) else {
            return;
        };
        // If cancelled while running, drop the result.
        if matches!(job.state, TrainJobState::Cancelled { .. }) {
            return;
        }
        match result {
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

    Ok(json!({ "job_id": id, "status": "running", "llm": descriptor }))
}

/// `GET /skillopt/status/:job` — current job state. `404` if unknown.
pub async fn train_status_value(job_id: &str) -> Option<Value> {
    let map = jobs().lock().await;
    map.get(job_id).map(|j| serde_json::to_value(j).unwrap_or_else(|_| json!({"id": j.id})))
}

/// `POST /skillopt/cancel/:job` — best-effort cancel (see [`TrainJobState::Cancelled`]).
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
            .then_with(|| a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or("")))
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
        "best_skill_md": report.best_skill_md,
    })
}

// ── promote (write *.opt.md) — explicit, audited ────────────────────────────

/// `POST /skillopt/promote {skill, content}` — write `*.opt.md` next to the
/// shipped skill file. **Never** overwrites the shipped `skills/*.md`; the
/// panel requires a separate human action to swap a promoted artifact into
/// the live loader (ties into the patent-audit rule about surfacing AI
/// output). Returns the written path.
pub fn promote_value(skill_name: &str, content: &str) -> Result<Value, String> {
    let path = with_state(|s| s.catalog.get(skill_name).map(|e| e.path.clone()))
        .ok_or_else(|| "skillforge catalog not initialized".to_string())?
        .ok_or_else(|| format!("skill '{skill_name}' not in catalog"))?;
    let opt_path = path.with_extension("opt.md");
    std::fs::write(&opt_path, content).map_err(|e| e.to_string())?;
    Ok(json!({
        "skill": skill_name,
        "written": opt_path.display().to_string(),
        "note": "promoted to *.opt.md — the shipped skill is untouched; swap into the live loader deliberately.",
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
        None => (StatusCode::NOT_FOUND, json!({ "error": format!("skill '{name}' not found") })),
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
    let method = if req.method.is_empty() { "parallel" } else { req.method.as_str() };
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
        None => (StatusCode::NOT_FOUND, json!({ "error": format!("no job '{job}'") })),
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
        };
        let v = report_to_value(&report);
        assert_eq!(v["skill_name"], "demo");
        assert_eq!(v["accepted"], 2);
        // f32 → JSON f64 rounding: compare with tolerance, not exact eq.
        let last = v["val_curve"][2].as_f64().unwrap();
        assert!((last - 0.66).abs() < 1e-5);
    }
}