/*!
 * BDD tests for the Context Assembler (Phase 2 of the memory-as-infrastructure
 * redesign). Exercises the policy-driven entry point that replaces the three
 * ad-hoc injection sites in `main.rs`.
 *
 * Run with: cargo test --test context_assembler_bdd
 */
use cucumber::{World, given, then, when};
use std::path::PathBuf;
use tempfile::TempDir;
use vibecli_cli::context_assembler::{
    AgentKind, AssembledContext, ContextBudget, ContextPolicy, MemoryToggles,
    assemble_context,
};

#[derive(Default, World)]
pub struct AssemblerWorld {
    tmp: Option<TempDir>,
    jobs_tmp: Option<TempDir>,
    ctx: Option<AssembledContext>,
}

impl std::fmt::Debug for AssemblerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssemblerWorld")
            .field("tmp", &self.tmp.as_ref().map(|t| t.path().to_owned()))
            .field("jobs_tmp", &self.jobs_tmp.as_ref().map(|t| t.path().to_owned()))
            .field("ctx_sections", &self.ctx.as_ref().map(|c| {
                c.sections
                    .iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
            }))
            .finish()
    }
}

impl AssemblerWorld {
    fn workspace(&mut self) -> PathBuf {
        if self.tmp.is_none() {
            self.tmp = Some(TempDir::new().expect("tempdir"));
        }
        self.tmp.as_ref().unwrap().path().to_owned()
    }
    fn jobs_db_path(&mut self) -> PathBuf {
        if self.jobs_tmp.is_none() {
            self.jobs_tmp = Some(TempDir::new().expect("jobs tempdir"));
        }
        self.jobs_tmp.as_ref().unwrap().path().join("jobs.db")
    }
    fn ctx(&self) -> &AssembledContext {
        self.ctx.as_ref().expect("assembler has not been run yet")
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r"^a fresh workspace$")]
fn given_workspace(w: &mut AssemblerWorld) {
    let _ = w.workspace();
}

#[given(regex = r#"^the workspace contains a "([^"]+)" file with body "([^"]+)"$"#)]
fn given_file_with_body(w: &mut AssemblerWorld, name: String, body: String) {
    let workspace = w.workspace();
    std::fs::write(workspace.join(&name), body).expect("write file");
}

#[given(regex = r#"^the workspace contains a "([^"]+)" file of size (\d+)$"#)]
fn given_file_of_size(w: &mut AssemblerWorld, name: String, size: usize) {
    let workspace = w.workspace();
    let body = "x".repeat(size);
    std::fs::write(workspace.join(&name), body).expect("write file");
}

#[given(
    regex = r#"^job "([^"]+)" has scratchpad entry "([^"]+)" with value "([^"]+)"$"#
)]
fn given_scratchpad_entry(
    w: &mut AssemblerWorld,
    session_id: String,
    key: String,
    value: String,
) {
    let path = w.jobs_db_path();
    let db = vibecli_cli::job_manager::JobsDb::open(&path).expect("open jobs db");
    db.scratchpad_set(&session_id, &key, &value).expect("set entry");
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the assembler runs with policy "chat"$"#)]
fn when_run_chat(w: &mut AssemblerWorld) {
    let workspace = w.workspace();
    w.ctx = Some(assemble_context(
        &workspace,
        &ContextPolicy::Chat,
        &ContextBudget::default(),
        &MemoryToggles::default(),
    ));
}

#[when(regex = r#"^the assembler runs with policy "chat" and total budget (\d+)$"#)]
fn when_run_chat_with_budget(w: &mut AssemblerWorld, total: usize) {
    let workspace = w.workspace();
    let budget = ContextBudget {
        max_total_chars: total,
        max_section_chars: 10_000,
        section_caps: Vec::new(),
    };
    w.ctx = Some(assemble_context(
        &workspace,
        &ContextPolicy::Chat,
        &budget,
        &MemoryToggles::default(),
    ));
}

#[when(
    regex = r#"^the assembler runs with policy "agent" and task "([^"]+)" and OpenMemory disabled$"#
)]
fn when_run_agent_disabled(w: &mut AssemblerWorld, task: String) {
    let workspace = w.workspace();
    let toggles = MemoryToggles {
        openmemory_enabled: false,
        openmemory_auto_inject: false,
        jobs_db_path: None,
    };
    w.ctx = Some(assemble_context(
        &workspace,
        &ContextPolicy::Agent {
            task,
            job_id: None,
        },
        &ContextBudget::default(),
        &toggles,
    ));
}

#[when(
    regex = r#"^the assembler runs with policy "agent" for task "([^"]+)" and job "([^"]+)"$"#
)]
fn when_run_agent_with_job(w: &mut AssemblerWorld, task: String, job_id: String) {
    let workspace = w.workspace();
    let db_path = w.jobs_db_path();
    let toggles = MemoryToggles {
        openmemory_enabled: false,
        openmemory_auto_inject: false,
        jobs_db_path: Some(db_path),
    };
    w.ctx = Some(assemble_context(
        &workspace,
        &ContextPolicy::Agent {
            task,
            job_id: Some(job_id),
        },
        &ContextBudget::default(),
        &toggles,
    ));
}

#[when(
    regex = r#"^the assembler runs with policy "agent" for task "([^"]+)" and job "([^"]+)" under total budget (\d+)$"#
)]
fn when_run_agent_with_job_and_budget(
    w: &mut AssemblerWorld,
    task: String,
    job_id: String,
    total: usize,
) {
    let workspace = w.workspace();
    let db_path = w.jobs_db_path();
    let toggles = MemoryToggles {
        openmemory_enabled: false,
        openmemory_auto_inject: false,
        jobs_db_path: Some(db_path),
    };
    let budget = ContextBudget {
        max_total_chars: total,
        max_section_chars: 10_000,
        section_caps: Vec::new(),
    };
    w.ctx = Some(assemble_context(
        &workspace,
        &ContextPolicy::Agent {
            task,
            job_id: Some(job_id),
        },
        &budget,
        &toggles,
    ));
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(expr = "the assembled context includes a section named {string}")]
fn then_includes_section(w: &mut AssemblerWorld, name: String) {
    let ctx = w.ctx();
    assert!(
        ctx.sections.iter().any(|s| s.name == name.as_str()),
        "expected section {:?}; got {:?}",
        name,
        ctx.sections.iter().map(|s| s.name).collect::<Vec<_>>()
    );
}

#[then(expr = "the assembled context omits a section named {string}")]
fn then_omits_section(w: &mut AssemblerWorld, name: String) {
    let ctx = w.ctx();
    assert!(
        !ctx.sections.iter().any(|s| s.name == name.as_str()),
        "section {:?} should have been omitted; got {:?}",
        name,
        ctx.sections.iter().map(|s| s.name).collect::<Vec<_>>()
    );
}

#[then(expr = "the section {string} contains {string}")]
fn then_section_contains(w: &mut AssemblerWorld, name: String, needle: String) {
    let ctx = w.ctx();
    let section = ctx
        .get(name.as_str())
        .unwrap_or_else(|| panic!("missing section {:?}", name));
    assert!(
        section.contains(&needle),
        "section {:?} did not contain {:?}; full content: {}",
        name,
        needle,
        section
    );
}

#[then(expr = "the assembled total chars are at most {int}")]
fn then_total_at_most(w: &mut AssemblerWorld, n: usize) {
    let ctx = w.ctx();
    assert!(
        ctx.total_chars <= n,
        "total {} > {}",
        ctx.total_chars,
        n
    );
}

#[then(expr = "the combined context contains {string}")]
fn then_combined_contains(w: &mut AssemblerWorld, needle: String) {
    let ctx = w.ctx();
    let combined = ctx.combined().expect("combined output");
    assert!(
        combined.contains(&needle),
        "combined did not contain {:?}; combined: {}",
        needle,
        combined
    );
}

// ── Per-kind budget assertions (Phase 4) ─────────────────────────────────────

fn kind_from(name: &str) -> AgentKind {
    match name {
        "chat" => AgentKind::Chat,
        "coding" => AgentKind::CodingAgent,
        "research" => AgentKind::ResearchAgent,
        "background" => AgentKind::BackgroundJob,
        other => panic!("unknown agent kind in feature: {other:?}"),
    }
}

#[then(expr = "the budget for kind {string} caps {string} higher than {string}")]
fn then_budget_kind_cap_higher(
    _w: &mut AssemblerWorld,
    kind: String,
    bigger: String,
    smaller: String,
) {
    let b = ContextBudget::for_kind(kind_from(&kind));
    let big = b
        .cap_for(bigger.as_str())
        .unwrap_or_else(|| panic!("no cap for {bigger:?}"));
    let small = b
        .cap_for(smaller.as_str())
        .unwrap_or_else(|| panic!("no cap for {smaller:?}"));
    assert!(
        big > small,
        "kind {kind:?}: expected {bigger} ({big}) > {smaller} ({small})"
    );
}

#[then(expr = "the budget for kind {string} has total at most {int}")]
fn then_budget_kind_total_at_most(
    _w: &mut AssemblerWorld,
    kind: String,
    max: usize,
) {
    let b = ContextBudget::for_kind(kind_from(&kind));
    assert!(
        b.max_total_chars <= max,
        "kind {kind:?} total {} > {max}",
        b.max_total_chars
    );
}

fn main() {
    futures::executor::block_on(AssemblerWorld::run(
        "tests/features/context_assembler.feature",
    ));
}
