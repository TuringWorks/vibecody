//! Context Assembler — single entry point that builds the system context for
//! a turn out of the various memory subsystems (`ProjectMemory`,
//! `OpenMemory`, orchestration lessons, project profile, task-relevant
//! files).
//!
//! Phase 2 of the memory-as-infrastructure redesign. Initially this module
//! reproduces the existing injection sites in `main.rs` (REPL chat seed at
//! L4135 and agent task seed + OpenMemory layered context at L13575) under
//! a single, policy-driven budget. No behavior change yet — Phase 4 will
//! tune per-agent-type allocations on top of this scaffold.
//!
//! The assembler is deliberately pure: it takes a workspace path, a
//! `ContextPolicy`, a `ContextBudget`, and the small subset of config
//! toggles it cares about, and returns an ordered list of named
//! `ContextSection`s. Callers decide whether to emit them as one combined
//! system message or separately.

// Phase 2 only wires the chat site; the Agent variant, `total_chars`,
// `combined`, and `is_empty` are consumed by the lib tests and BDD harness
// and will be wired into the agent task site in Phase 3/4.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

// ── Public types ────────────────────────────────────────────────────────────

/// Which kind of session is being assembled. Different policies pull from
/// different retrievers.
#[derive(Debug, Clone)]
pub enum ContextPolicy {
    /// REPL chat — hierarchical project memory + orchestration rules.
    Chat,
    /// Agent task execution — project profile + task-relevant files +
    /// OpenMemory layered context + (when `job_id` is `Some`) the durable
    /// agent scratchpad, keyed off the task description and session id.
    Agent {
        task: String,
        /// Session / job id used to load this agent's scratchpad from the
        /// JobsDb (Phase 3). `None` disables the scratchpad section.
        job_id: Option<String>,
    },
}

/// The kind of agent a context is being assembled for. Each kind has its
/// own budget shape (see `ContextBudget::for_kind`) so a research agent
/// gets more memory and a coding agent gets more task-file budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    /// REPL chat — small retrieval, project memory + orchestration.
    Chat,
    /// Coding/editing agent — large task-file budget, moderate memory.
    CodingAgent,
    /// Research/analysis agent — large memory budget, smaller code.
    ResearchAgent,
    /// Background/async job resuming work — scratchpad-dominant, minimal
    /// fresh retrieval (the agent is recovering its own state, not
    /// re-reading the world).
    BackgroundJob,
}

/// Hard caps on assembled context size, expressed in characters. Phase 4
/// differentiates this per-agent-type via `ContextBudget::for_kind`.
/// `section_caps` overrides `max_section_chars` on a per-section-name
/// basis; any section not listed falls back to `max_section_chars`.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    pub max_total_chars: usize,
    pub max_section_chars: usize,
    pub section_caps: Vec<(&'static str, usize)>,
}

impl Default for ContextBudget {
    fn default() -> Self {
        // Generous enough that current behavior is never truncated.
        Self {
            max_total_chars: 1024 * 1024,
            max_section_chars: 256 * 1024,
            section_caps: Vec::new(),
        }
    }
}

/// Parse a wire-format agent kind string into the typed enum.
///
/// Used by both the Tauri panel-facing command (S1) and the HTTP
/// `/agent` payload negotiation (S3) so the canonical valid-kind list
/// lives in exactly one place. The `Err` value is a human-readable
/// message that callers can echo back to clients verbatim.
pub fn parse_agent_kind(s: &str) -> Result<AgentKind, String> {
    match s {
        "Chat" => Ok(AgentKind::Chat),
        "CodingAgent" => Ok(AgentKind::CodingAgent),
        "ResearchAgent" => Ok(AgentKind::ResearchAgent),
        "BackgroundJob" => Ok(AgentKind::BackgroundJob),
        other => Err(format!(
            "unknown kind {other:?}; expected Chat | CodingAgent | ResearchAgent | BackgroundJob"
        )),
    }
}

/// Section names emitted by `assemble_context` for the various
/// `ContextPolicy` flavors. Single source of truth so the
/// `/v1/capabilities` advertisement and clients that consume it can
/// agree on which keys may appear in the assembled response.
pub const KNOWN_SECTION_NAMES: &[&str] = &[
    "project_memory",
    "orchestration",
    "project_profile",
    "task_files",
    "open_memory",
    "agent_scratchpad",
];

impl ContextBudget {
    /// Look up the effective cap for a named section, falling back to
    /// `max_section_chars` when no override is present.
    pub fn cap_for(&self, name: &str) -> Option<usize> {
        self.section_caps
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .or(Some(self.max_section_chars))
    }

    /// Budget tuned for the given agent kind. These numbers are a first
    /// cut — good enough to differentiate behavior across kinds but not
    /// yet tuned against a real token estimator; Phase 4+ will calibrate.
    pub fn for_kind(kind: AgentKind) -> Self {
        match kind {
            AgentKind::Chat => Self {
                max_total_chars: 32_000,
                max_section_chars: 16_000,
                section_caps: vec![
                    ("project_memory", 16_000),
                    ("orchestration", 8_000),
                ],
            },
            AgentKind::CodingAgent => Self {
                max_total_chars: 128_000,
                max_section_chars: 32_000,
                section_caps: vec![
                    ("agent_scratchpad", 16_000),
                    ("project_profile", 8_000),
                    // Code dominates — coding agents spend most of their
                    // budget on the files they're editing.
                    ("task_files", 96_000),
                    ("open_memory", 16_000),
                ],
            },
            AgentKind::ResearchAgent => Self {
                max_total_chars: 128_000,
                max_section_chars: 64_000,
                section_caps: vec![
                    ("agent_scratchpad", 16_000),
                    ("project_profile", 8_000),
                    ("task_files", 16_000),
                    // Memory dominates — research agents reason across
                    // history more than live code.
                    ("open_memory", 64_000),
                ],
            },
            AgentKind::BackgroundJob => Self {
                max_total_chars: 48_000,
                max_section_chars: 40_000,
                section_caps: vec![
                    // Scratchpad dominates — the job is resuming its
                    // own durable state, not re-indexing the world.
                    ("agent_scratchpad", 40_000),
                    ("project_profile", 4_000),
                    ("task_files", 4_000),
                ],
            },
        }
    }
}

/// One named slice of system context. Sections are kept distinct so callers
/// can either emit them as a single combined system message or as separate
/// messages, mirroring the existing injection-site behavior.
#[derive(Debug, Clone)]
pub struct ContextSection {
    pub name: &'static str,
    pub content: String,
    pub priority: u32,
    pub truncated: bool,
}

/// Output of the assembler — ordered sections plus accounting.
#[derive(Debug, Default, Clone)]
pub struct AssembledContext {
    pub sections: Vec<ContextSection>,
    pub total_chars: usize,
}

impl AssembledContext {
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Combine all sections into a single string suitable for one system
    /// message. Returns `None` when no sections were produced.
    pub fn combined(&self) -> Option<String> {
        if self.sections.is_empty() {
            return None;
        }
        let mut out =
            String::with_capacity(self.total_chars + self.sections.len() * 8);
        for (i, s) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n---\n\n");
            }
            out.push_str(&s.content);
        }
        Some(out)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.content.as_str())
    }
}

/// Subset of `config::MemoryConfig` that the assembler actually reads. Kept
/// separate so unit tests don't need to construct the full Config struct.
#[derive(Debug, Clone)]
pub struct MemoryToggles {
    pub openmemory_enabled: bool,
    pub openmemory_auto_inject: bool,
    /// Scratchpad source for `ContextPolicy::Agent { job_id: Some(_), .. }`.
    /// `None` disables the scratchpad section entirely (no DB access). In
    /// production callers pass `Some(job_manager::default_db_path())`;
    /// tests pass a tempdir path so unit tests never touch a user DB.
    pub jobs_db_path: Option<PathBuf>,
}

impl Default for MemoryToggles {
    fn default() -> Self {
        Self {
            openmemory_enabled: true,
            openmemory_auto_inject: true,
            jobs_db_path: None,
        }
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────

/// THE Phase 2 entry point. Build a single `AssembledContext` for `policy`
/// against `workspace`, respecting `budget`. Eventually replaces the ad-hoc
/// injection in `main.rs:4135` (chat) and `main.rs:13575` (agent).
pub fn assemble_context(
    workspace: &Path,
    policy: &ContextPolicy,
    budget: &ContextBudget,
    toggles: &MemoryToggles,
) -> AssembledContext {
    let raw = match policy {
        ContextPolicy::Chat => collect_chat_sections(workspace),
        ContextPolicy::Agent { task, job_id } => {
            collect_agent_sections(workspace, task, job_id.as_deref(), toggles)
        }
    };
    apply_budget(raw, budget)
}

// ── Per-policy collectors ───────────────────────────────────────────────────

fn collect_chat_sections(workspace: &Path) -> Vec<ContextSection> {
    let mut out = Vec::new();

    // 1) Hierarchical project memory (VIBECLI.md / AGENTS.md / CLAUDE.md).
    let memory = crate::memory::ProjectMemory::load(workspace);
    if let Some(content) = memory.combined() {
        out.push(ContextSection {
            name: "project_memory",
            content,
            priority: 0,
            truncated: false,
        });
    }

    // 2) Orchestration rules + saved lessons + active task.
    let lessons_store =
        crate::workflow_orchestration::LessonsStore::for_workspace(workspace);
    let todo_store =
        crate::workflow_orchestration::TodoStore::for_workspace(workspace);
    let lessons = lessons_store.load();
    let current_task = todo_store.load();
    let prompt = crate::workflow_orchestration::orchestration_system_prompt(
        &lessons,
        current_task.as_ref(),
    );
    if !prompt.is_empty() {
        out.push(ContextSection {
            name: "orchestration",
            content: prompt,
            priority: 1,
            truncated: false,
        });
    }

    out
}

fn collect_agent_sections(
    workspace: &Path,
    task: &str,
    job_id: Option<&str>,
    toggles: &MemoryToggles,
) -> Vec<ContextSection> {
    let mut out = Vec::new();

    // 1) Agent scratchpad — priority 0 (highest). The agent's own durable
    //    working state (plans, cursors, hypotheses). Must survive every
    //    other retriever's budget. Populated only when the policy supplies
    //    a job_id AND the toggles expose a scratchpad source.
    if let (Some(sid), Some(db_path)) = (job_id, toggles.jobs_db_path.as_deref())
    {
        if let Some(rendered) = render_scratchpad(db_path, sid) {
            out.push(ContextSection {
                name: "agent_scratchpad",
                content: rendered,
                priority: 0,
                truncated: false,
            });
        }
    }

    // 2) Project profile — always-on understanding of repo shape.
    let profile = crate::project_init::get_or_scan_profile(workspace);
    let summary = profile.to_system_prompt_context();
    if !summary.is_empty() {
        out.push(ContextSection {
            name: "project_profile",
            content: summary,
            priority: 1,
            truncated: false,
        });
    }

    // 3) Task-relevant files (preview, max 5 files / 80 lines each).
    let relevant =
        crate::project_init::extract_relevant_files_for_task(workspace, task);
    let mut files_block = String::new();
    let mut included = 0usize;
    for rel_path in relevant.iter() {
        if included >= 5 {
            break;
        }
        let full_path = workspace.join(rel_path);
        if !full_path.is_file() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let preview: String =
                content.lines().take(80).collect::<Vec<_>>().join("\n");
            if files_block.is_empty() {
                files_block.push_str("=== Task-Relevant Files ===\n\n");
            }
            files_block
                .push_str(&format!("--- {} ---\n{}\n\n", rel_path, preview));
            included += 1;
        }
    }
    if !files_block.is_empty() {
        out.push(ContextSection {
            name: "task_files",
            content: files_block,
            priority: 2,
            truncated: false,
        });
    }

    // 4) OpenMemory layered context (config-gated).
    if toggles.openmemory_enabled && toggles.openmemory_auto_inject {
        let store = crate::open_memory::project_scoped_store(workspace);
        let ctx = store.get_layered_context_default(task);
        if !ctx.is_empty() {
            out.push(ContextSection {
                name: "open_memory",
                content: ctx,
                priority: 3,
                truncated: false,
            });
        }
    }

    out
}

/// Render the agent scratchpad for a session as a human-readable block,
/// or return `None` when there are no entries (or the DB cannot be opened,
/// which is expected on first run before any job has been scheduled).
fn render_scratchpad(db_path: &Path, session_id: &str) -> Option<String> {
    let db = crate::job_manager::JobsDb::open(db_path).ok()?;
    let entries = db.scratchpad_list(session_id).ok()?;
    if entries.is_empty() {
        return None;
    }
    let mut block = String::new();
    block.push_str("=== Agent Scratchpad ===\n");
    block.push_str(
        "(durable working state from prior turns of this job)\n\n",
    );
    for e in entries.iter() {
        block.push_str(&format!("[{}]\n{}\n\n", e.key, e.value));
    }
    Some(block)
}

// ── Budget enforcement ──────────────────────────────────────────────────────

/// Apply per-section and total-budget caps. Sections are processed in
/// priority order (lower = higher priority); once the total budget is
/// exhausted, further sections are dropped. Sections that exceed the
/// per-section cap are truncated with a marker.
fn apply_budget(
    mut sections: Vec<ContextSection>,
    budget: &ContextBudget,
) -> AssembledContext {
    sections.sort_by_key(|s| s.priority);
    let mut out: Vec<ContextSection> = Vec::with_capacity(sections.len());
    let mut total = 0usize;
    for mut section in sections {
        // Per-section cap (override if configured, else default).
        let section_cap = budget
            .cap_for(section.name)
            .unwrap_or(budget.max_section_chars);
        if section.content.len() > section_cap {
            let mut cut = section_cap;
            while cut > 0 && !section.content.is_char_boundary(cut) {
                cut -= 1;
            }
            section.content.truncate(cut);
            section
                .content
                .push_str("\n\n[…truncated to fit section budget…]");
            section.truncated = true;
        }
        // Total cap.
        if total + section.content.len() > budget.max_total_chars {
            let remaining = budget.max_total_chars.saturating_sub(total);
            if remaining == 0 {
                break;
            }
            let mut cut = remaining;
            while cut > 0 && !section.content.is_char_boundary(cut) {
                cut -= 1;
            }
            section.content.truncate(cut);
            section
                .content
                .push_str("\n\n[…truncated to fit total budget…]");
            section.truncated = true;
        }
        total += section.content.len();
        out.push(section);
    }
    AssembledContext {
        sections: out,
        total_chars: total,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(path: &std::path::Path, content: &str) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    // ── S3: parse_agent_kind contract ────────────────────────────────────

    #[test]
    fn parse_agent_kind_accepts_all_four_variants() {
        // Pinning the wire-format spelling. Clients that hit
        // `/v1/capabilities` get this exact list back; any drift here
        // would silently break mobile/watch/IDE callers that hard-coded
        // the strings against an earlier daemon version.
        assert_eq!(parse_agent_kind("Chat").unwrap(), AgentKind::Chat);
        assert_eq!(parse_agent_kind("CodingAgent").unwrap(), AgentKind::CodingAgent);
        assert_eq!(parse_agent_kind("ResearchAgent").unwrap(), AgentKind::ResearchAgent);
        assert_eq!(parse_agent_kind("BackgroundJob").unwrap(), AgentKind::BackgroundJob);
    }

    #[test]
    fn parse_agent_kind_rejects_unknown_with_helpful_message() {
        // The error must (a) echo the bad input back so the client can
        // log it without re-encoding, and (b) list the valid alternatives
        // so a human reading a log can spot the typo immediately.
        let err = parse_agent_kind("Sycophant").unwrap_err();
        assert!(
            err.contains("Sycophant"),
            "error must echo bad input; got: {err}"
        );
        assert!(
            err.contains("Chat") && err.contains("BackgroundJob"),
            "error must list valid kinds; got: {err}"
        );
    }

    #[test]
    fn parse_agent_kind_is_case_sensitive() {
        // Pinning case sensitivity so a client passing lowercase or
        // SHOUTY_SNAKE_CASE gets a clear error instead of silently
        // falling back to a default kind.
        assert!(parse_agent_kind("chat").is_err());
        assert!(parse_agent_kind("CHAT").is_err());
        assert!(parse_agent_kind("coding_agent").is_err());
    }

    #[test]
    fn known_section_names_lists_every_assembler_emission() {
        // Every section name produced by `assemble_context` (across
        // both Chat and Agent policies) must appear in the public
        // KNOWN_SECTION_NAMES list. If a future slice adds a new
        // section but forgets to update this constant, /v1/capabilities
        // advertises a stale shape and clients break.
        let known: std::collections::HashSet<&str> =
            KNOWN_SECTION_NAMES.iter().copied().collect();
        for expected in
            ["project_memory", "orchestration", "project_profile", "task_files", "open_memory", "agent_scratchpad"]
        {
            assert!(
                known.contains(expected),
                "KNOWN_SECTION_NAMES is missing {expected:?}"
            );
        }
    }

    #[test]
    fn assemble_chat_always_includes_orchestration_section() {
        let tmp = TempDir::new().unwrap();
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Chat,
            &ContextBudget::default(),
            &MemoryToggles::default(),
        );
        // orchestration_system_prompt always returns the core rules even with
        // no lessons and no active task — the section must always appear.
        assert!(
            ctx.sections.iter().any(|s| s.name == "orchestration"),
            "expected orchestration section, got: {:?}",
            ctx.sections.iter().map(|s| s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn assemble_chat_includes_project_memory_when_present() {
        let tmp = TempDir::new().unwrap();
        write(
            &tmp.path().join("VIBECLI.md"),
            "# Project rules\nbe terse.\n",
        );
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Chat,
            &ContextBudget::default(),
            &MemoryToggles::default(),
        );
        let mem = ctx.get("project_memory").expect("project_memory section");
        assert!(mem.contains("be terse"), "got: {}", mem);
    }

    #[test]
    fn budget_truncation_caps_section_size_with_marker() {
        let tmp = TempDir::new().unwrap();
        let big = "x".repeat(10_000);
        write(&tmp.path().join("VIBECLI.md"), &big);
        let budget = ContextBudget {
            max_total_chars: 100_000,
            max_section_chars: 1_000,
            section_caps: Vec::new(),
        };
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Chat,
            &budget,
            &MemoryToggles::default(),
        );
        let section = ctx
            .sections
            .iter()
            .find(|s| s.name == "project_memory")
            .expect("project_memory");
        assert!(section.truncated, "section should be flagged truncated");
        assert!(
            section.content.contains("truncated"),
            "expected truncation marker, got: {}",
            section.content
        );
        // Cap (1000) + marker (~50). Real content + user-tier files won't
        // push past a few hundred chars more than that.
        assert!(
            section.content.len() <= 1_500,
            "len={}",
            section.content.len()
        );
    }

    #[test]
    fn budget_total_cap_drops_low_priority_sections() {
        let tmp = TempDir::new().unwrap();
        let big = "x".repeat(5_000);
        write(&tmp.path().join("VIBECLI.md"), &big);
        let budget = ContextBudget {
            max_total_chars: 500,
            max_section_chars: 10_000,
            section_caps: Vec::new(),
        };
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Chat,
            &budget,
            &MemoryToggles::default(),
        );
        // priority-0 project_memory consumes the whole budget; orchestration
        // (priority 1) gets dropped.
        assert!(
            ctx.total_chars <= 600,
            "total {} should be near 500-char budget",
            ctx.total_chars
        );
        assert!(
            !ctx.sections.iter().any(|s| s.name == "orchestration"),
            "orchestration must be dropped when budget is exhausted; got: {:?}",
            ctx.sections.iter().map(|s| s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn combined_concatenates_sections_with_separator() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("VIBECLI.md"), "ALPHA-MARKER");
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Chat,
            &ContextBudget::default(),
            &MemoryToggles::default(),
        );
        let combined = ctx.combined().expect("combined");
        assert!(combined.contains("ALPHA-MARKER"));
        if ctx.sections.len() > 1 {
            assert!(
                combined.contains("---"),
                "expected separator between sections"
            );
        }
    }

    #[test]
    fn agent_policy_with_openmemory_disabled_omits_open_memory_section() {
        let tmp = TempDir::new().unwrap();
        let toggles = MemoryToggles {
            openmemory_enabled: false,
            openmemory_auto_inject: false,
            jobs_db_path: None,
        };
        let ctx = assemble_context(
            tmp.path(),
            &ContextPolicy::Agent {
                task: "do a thing".into(),
                job_id: None,
            },
            &ContextBudget::default(),
            &toggles,
        );
        assert!(
            !ctx.sections.iter().any(|s| s.name == "open_memory"),
            "open_memory must not appear when OpenMemory toggles are off"
        );
    }

    #[test]
    fn empty_context_combined_returns_none() {
        let ctx = AssembledContext::default();
        assert!(ctx.combined().is_none());
        assert!(ctx.is_empty());
    }

    // ── Retrieval policies per agent kind (Phase 4) ──────────────────

    #[test]
    fn budget_for_chat_is_small_and_covers_project_memory_and_orchestration() {
        let b = ContextBudget::for_kind(AgentKind::Chat);
        // Chat is lightweight — small total, no code retrievers.
        assert!(b.max_total_chars <= 64_000, "chat total too large: {}", b.max_total_chars);
        assert!(b.cap_for("project_memory").is_some());
        assert!(b.cap_for("orchestration").is_some());
    }

    #[test]
    fn budget_for_coding_agent_favors_task_files() {
        let b = ContextBudget::for_kind(AgentKind::CodingAgent);
        let task_files = b.cap_for("task_files").expect("task_files cap");
        let open_memory = b.cap_for("open_memory").expect("open_memory cap");
        assert!(
            task_files > open_memory,
            "CodingAgent should prefer task_files ({}) over open_memory ({})",
            task_files,
            open_memory
        );
    }

    #[test]
    fn budget_for_research_agent_favors_open_memory() {
        let b = ContextBudget::for_kind(AgentKind::ResearchAgent);
        let task_files = b.cap_for("task_files").expect("task_files cap");
        let open_memory = b.cap_for("open_memory").expect("open_memory cap");
        assert!(
            open_memory > task_files,
            "ResearchAgent should prefer open_memory ({}) over task_files ({})",
            open_memory,
            task_files
        );
    }

    #[test]
    fn budget_for_background_job_favors_scratchpad() {
        let b = ContextBudget::for_kind(AgentKind::BackgroundJob);
        let scratchpad = b.cap_for("agent_scratchpad").expect("scratchpad cap");
        let task_files = b.cap_for("task_files").unwrap_or(0);
        assert!(
            scratchpad >= task_files * 2,
            "BackgroundJob scratchpad ({}) should dominate task_files ({})",
            scratchpad,
            task_files
        );
    }

    #[test]
    fn apply_budget_honors_per_section_cap_over_default() {
        // Default per-section cap is 10_000; we override task_files to 500.
        let budget = ContextBudget {
            max_total_chars: 1_000_000,
            max_section_chars: 10_000,
            section_caps: vec![("task_files", 500)],
        };
        let long = "x".repeat(5_000);
        let sections = vec![ContextSection {
            name: "task_files",
            content: long,
            priority: 0,
            truncated: false,
        }];
        let out = apply_budget(sections, &budget);
        let s = out.sections.first().expect("section");
        assert!(s.truncated, "section should be truncated");
        assert!(s.content.len() <= 700, "content={}", s.content.len());
    }

    // ── Scratchpad integration (Phase 3) ──────────────────────────────

    fn jobs_db_with(entries: &[(&str, &str, &str)]) -> (std::path::PathBuf, tempfile::TempDir) {
        // Open with the default key-derivation path so writes and reads
        // (via `render_scratchpad`) share the same encryption key.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("jobs.db");
        let db = crate::job_manager::JobsDb::open(&path).unwrap();
        for (sid, key, value) in entries {
            db.scratchpad_set(sid, key, value).unwrap();
        }
        (path, tmp)
    }

    #[test]
    fn agent_policy_includes_scratchpad_section_when_job_has_entries() {
        let workspace = TempDir::new().unwrap();
        let (db_path, _db_tmp) = jobs_db_with(&[
            ("job-42", "plan", "1. audit\n2. refactor\n3. verify"),
            ("job-42", "cursor", "main.rs:4135"),
        ]);
        let toggles = MemoryToggles {
            openmemory_enabled: false,
            openmemory_auto_inject: false,
            jobs_db_path: Some(db_path),
        };
        let ctx = assemble_context(
            workspace.path(),
            &ContextPolicy::Agent {
                task: "finish the assembler".into(),
                job_id: Some("job-42".into()),
            },
            &ContextBudget::default(),
            &toggles,
        );
        let sp = ctx
            .get("agent_scratchpad")
            .expect("agent_scratchpad section");
        assert!(sp.contains("plan"), "got: {}", sp);
        assert!(sp.contains("1. audit"), "got: {}", sp);
        assert!(sp.contains("main.rs:4135"), "got: {}", sp);
    }

    #[test]
    fn agent_policy_without_job_id_omits_scratchpad() {
        let workspace = TempDir::new().unwrap();
        let (db_path, _db_tmp) = jobs_db_with(&[("other-job", "k", "v")]);
        let toggles = MemoryToggles {
            openmemory_enabled: false,
            openmemory_auto_inject: false,
            jobs_db_path: Some(db_path),
        };
        let ctx = assemble_context(
            workspace.path(),
            &ContextPolicy::Agent {
                task: "no job id".into(),
                job_id: None,
            },
            &ContextBudget::default(),
            &toggles,
        );
        assert!(
            !ctx.sections.iter().any(|s| s.name == "agent_scratchpad"),
            "scratchpad must be omitted when no job_id"
        );
    }

    #[test]
    fn agent_policy_with_empty_scratchpad_omits_section() {
        let workspace = TempDir::new().unwrap();
        let (db_path, _db_tmp) = jobs_db_with(&[]);
        let toggles = MemoryToggles {
            openmemory_enabled: false,
            openmemory_auto_inject: false,
            jobs_db_path: Some(db_path),
        };
        let ctx = assemble_context(
            workspace.path(),
            &ContextPolicy::Agent {
                task: "fresh job".into(),
                job_id: Some("job-empty".into()),
            },
            &ContextBudget::default(),
            &toggles,
        );
        assert!(
            !ctx.sections.iter().any(|s| s.name == "agent_scratchpad"),
            "scratchpad should not appear for a job with zero entries"
        );
    }

    #[test]
    fn agent_scratchpad_has_highest_priority() {
        // Scratchpad is the agent's own durable state — must never be
        // dropped when other retrievers' budgets are tight.
        let workspace = TempDir::new().unwrap();
        let (db_path, _db_tmp) =
            jobs_db_with(&[("job-p", "plan", "PLAN-MARKER")]);
        let toggles = MemoryToggles {
            openmemory_enabled: false,
            openmemory_auto_inject: false,
            jobs_db_path: Some(db_path),
        };
        let budget = ContextBudget {
            max_total_chars: 200,
            max_section_chars: 10_000,
            section_caps: Vec::new(),
        };
        let ctx = assemble_context(
            workspace.path(),
            &ContextPolicy::Agent {
                task: "tight budget".into(),
                job_id: Some("job-p".into()),
            },
            &budget,
            &toggles,
        );
        // scratchpad present, project_profile dropped (lower priority).
        assert!(
            ctx.sections.iter().any(|s| s.name == "agent_scratchpad"),
            "scratchpad must survive a tight budget; got {:?}",
            ctx.sections.iter().map(|s| s.name).collect::<Vec<_>>()
        );
        assert!(
            ctx.get("agent_scratchpad")
                .unwrap()
                .contains("PLAN-MARKER")
        );
    }
}
