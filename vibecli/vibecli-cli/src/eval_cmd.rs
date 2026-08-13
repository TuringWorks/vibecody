//! `vibecli --eval …` — driving the evaluation harness from the CLI.
//!
//! Everything of substance lives in the `vibe-eval` crate; this module is the
//! command surface plus two things that cannot live there:
//!
//! * [`ProviderJudge`], which wires rubric grading to whichever provider the
//!   caller selected. The harness crate deliberately does not know how to
//!   construct a provider — a judge that silently defaulted to one vendor
//!   would violate the provider-agnostic rule and, worse, would make rubric
//!   scores incomparable across runs without saying so.
//! * Run persistence under `~/.vibecli/evals/`, so `report` and `gate` can
//!   talk about a run that finished yesterday.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole};
use vibe_eval::dataset::{self, Registry};
use vibe_eval::gate::{self, GatePolicy};
use vibe_eval::grade::{JudgeModel, JudgeScore};
use vibe_eval::harness::{CliConfig, CliHarness, DaemonConfig, DaemonHarness, ProbeHarness};
use vibe_eval::report::EvalReport;
use vibe_eval::runner::{Filter, RunConfig, Runner};
use vibe_eval::task::{Capability, Difficulty, Surface};

// ── Argument helpers ─────────────────────────────────────────────────────────

struct Args<'a>(&'a [String]);

impl<'a> Args<'a> {
    fn flag(&self, name: &str) -> Option<String> {
        let mut it = self.0.iter().peekable();
        while let Some(a) = it.next() {
            if a == name {
                return it.next().cloned();
            }
            if let Some(v) = a.strip_prefix(&format!("{}=", name)) {
                return Some(v.to_string());
            }
        }
        None
    }

    fn has(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }

    /// Comma-separated list form: `--capability code_repair,debugging`.
    fn list(&self, name: &str) -> Vec<String> {
        self.flag(name)
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn positionals(&self) -> Vec<&str> {
        // Values belonging to a flag are skipped, so `--suite safety list`
        // does not read `safety` as the subcommand.
        let mut out = Vec::new();
        let mut skip_next = false;
        for a in self.0 {
            if skip_next {
                skip_next = false;
                continue;
            }
            if a.starts_with("--") {
                skip_next = !a.contains('=') && takes_value(a);
                continue;
            }
            out.push(a.as_str());
        }
        out
    }
}

fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--suite"
            | "--capability"
            | "--surface"
            | "--difficulty"
            | "--tag"
            | "--task"
            | "--limit"
            | "--provider"
            | "--model"
            | "--concurrency"
            | "--timeout"
            | "--samples"
            | "--judge-provider"
            | "--judge-model"
            | "--baseline"
            | "--min-pass-rate"
            | "--min-coverage"
            | "--max-regressions"
            | "--out"
            | "--binary"
            | "--daemon-url"
            | "--format"
    )
}

// ── The judge ────────────────────────────────────────────────────────────────

/// Rubric grading backed by whichever provider the caller selected.
pub struct ProviderJudge {
    provider: Arc<dyn LLMProvider>,
    label: String,
}

impl ProviderJudge {
    pub fn new(provider: Arc<dyn LLMProvider>, label: String) -> Self {
        Self { provider, label }
    }
}

#[async_trait::async_trait]
impl JudgeModel for ProviderJudge {
    fn describe(&self) -> String {
        self.label.clone()
    }

    async fn score(&self, rubric: &str, prompt: &str, answer: &str) -> Result<JudgeScore, String> {
        if answer.trim().is_empty() {
            // Nothing was produced, so there is nothing to judge. Returning a
            // zero here would be a judgement the model never made.
            return Err("the agent produced no final answer to judge".to_string());
        }
        let system = "You are grading an AI assistant's work against a rubric. \
             Reply with JSON only, in the form \
             {\"score\": <number between 0 and 1>, \"rationale\": \"<one or two sentences>\"}. \
             Do not wrap it in markdown fences. Be strict: award a high score only \
             when the rubric's criteria are actually met.";
        let user = format!(
            "## Rubric\n{rubric}\n\n## What the assistant was asked\n{prompt}\n\n\
             ## What the assistant produced\n{answer}\n\n\
             Return only the JSON object."
        );
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: system.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: user,
            },
        ];
        let raw = self
            .provider
            .chat(&messages, None)
            .await
            .map_err(|e| e.to_string())?;
        parse_judge_reply(&raw)
    }
}

/// Pull the score out of a judge reply.
///
/// Models wrap JSON in fences, prefix it with prose, or emit reasoning tags.
/// A parse failure is an error rather than a default score: a judge whose
/// output we cannot read has not graded anything.
fn parse_judge_reply(raw: &str) -> Result<JudgeScore, String> {
    let cleaned = raw
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string();
    let start = cleaned.find('{');
    let end = cleaned.rfind('}');
    let slice = match (start, end) {
        (Some(s), Some(e)) if e > s => &cleaned[s..=e],
        _ => {
            return Err(format!(
                "judge reply contained no JSON object: {}",
                tail(&cleaned)
            ))
        }
    };
    let value: serde_json::Value = serde_json::from_str(slice)
        .map_err(|e| format!("judge reply is not valid JSON ({}): {}", e, tail(slice)))?;
    let score = value
        .get("score")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("judge reply has no numeric `score`: {}", tail(slice)))?;
    if !(0.0..=1.0).contains(&score) {
        return Err(format!(
            "judge returned {} which is outside 0.0..=1.0",
            score
        ));
    }
    Ok(JudgeScore {
        score,
        rationale: value
            .get("rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("(no rationale given)")
            .to_string(),
    })
}

fn tail(s: &str) -> String {
    s.chars()
        .rev()
        .take(200)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

// ── Run storage ──────────────────────────────────────────────────────────────

fn runs_dir() -> Option<PathBuf> {
    vibe_eval::runs_dir()
}

fn save_run(report: &EvalReport) -> Result<PathBuf, String> {
    let dir = runs_dir()
        .ok_or("HOME is unset, so there is nowhere to archive the run")?
        .join(&report.run_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = report.to_json().map_err(|e| e.to_string())?;
    std::fs::write(dir.join("report.json"), &json).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("report.md"), report.to_markdown()).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn load_run(run_id: &str) -> Result<EvalReport, String> {
    let dir = runs_dir().ok_or("HOME is unset")?;
    let path = if run_id == "latest" {
        latest_run(&dir).ok_or_else(|| format!("no runs found under {}", dir.display()))?
    } else {
        dir.join(run_id)
    };
    let file = path.join("report.json");
    let text = std::fs::read_to_string(&file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("{} is not a valid report: {}", file.display(), e))
}

fn latest_run(dir: &Path) -> Option<PathBuf> {
    let mut runs: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.path().join("report.json").exists())
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e.path()))
        })
        .collect();
    runs.sort_by_key(|(t, _)| *t);
    runs.pop().map(|(_, p)| p)
}

/// The repository root, needed by `repo_root` conformance tasks and by the
/// default suites directory. Walks up from the cwd looking for the suites.
fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut cursor = Some(start);
    while let Some(dir) = cursor {
        if dir.join(vibe_eval::DEFAULT_SUITES_DIR).is_dir() {
            return Some(dir.to_path_buf());
        }
        cursor = dir.parent();
    }
    None
}

// ── Parsing filters ──────────────────────────────────────────────────────────

fn parse_capability(s: &str) -> Option<Capability> {
    Capability::ALL
        .iter()
        .copied()
        .find(|c| c.slug() == s.trim().to_lowercase())
}

fn parse_surface(s: &str) -> Option<Surface> {
    Surface::ALL
        .iter()
        .copied()
        .find(|x| x.slug() == s.trim().to_lowercase())
}

fn parse_difficulty(s: &str) -> Option<Difficulty> {
    match s.trim().to_lowercase().as_str() {
        "easy" => Some(Difficulty::Easy),
        "medium" => Some(Difficulty::Medium),
        "hard" => Some(Difficulty::Hard),
        _ => None,
    }
}

/// Turn flags into a filter, reporting names that matched nothing.
///
/// A misspelled `--capability code_repare` silently selecting zero tasks and
/// printing a clean "0 passed" is exactly the kind of quiet success this
/// harness exists to avoid.
fn build_filter(args: &Args) -> Result<Filter, String> {
    let mut unknown = Vec::new();
    let capabilities = args
        .list("--capability")
        .into_iter()
        .filter_map(|s| {
            parse_capability(&s).or_else(|| {
                unknown.push(format!("capability `{}`", s));
                None
            })
        })
        .collect();
    let surfaces = args
        .list("--surface")
        .into_iter()
        .filter_map(|s| {
            parse_surface(&s).or_else(|| {
                unknown.push(format!("surface `{}`", s));
                None
            })
        })
        .collect();
    let difficulties = args
        .list("--difficulty")
        .into_iter()
        .filter_map(|s| {
            parse_difficulty(&s).or_else(|| {
                unknown.push(format!("difficulty `{}`", s));
                None
            })
        })
        .collect();

    if !unknown.is_empty() {
        return Err(format!(
            "unknown {}.\n  capabilities: {}\n  surfaces: {}\n  difficulties: easy, medium, hard",
            unknown.join(", "),
            Capability::ALL
                .iter()
                .map(|c| c.slug())
                .collect::<Vec<_>>()
                .join(", "),
            Surface::ALL
                .iter()
                .map(|s| s.slug())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok(Filter {
        suites: args.list("--suite"),
        capabilities,
        surfaces,
        difficulties,
        tags: args.list("--tag"),
        task_contains: args.flag("--task"),
        limit: args.flag("--limit").and_then(|v| v.parse().ok()),
    })
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// `vibecli --eval <subcommand> [flags]`
pub async fn run_eval_command(
    raw_args: &[String],
    default_provider: &str,
    default_model: Option<&str>,
) -> i32 {
    let args = Args(raw_args);
    let positionals = args.positionals();
    let subcommand = positionals.first().copied().unwrap_or("help");

    match subcommand {
        "list" => cmd_list(&args),
        "run" => cmd_run(&args, default_provider, default_model).await,
        "report" => cmd_report(&args, positionals.get(1).copied()),
        "gate" => cmd_gate(&args, positionals.get(1).copied()),
        "runs" => cmd_runs(),
        "datasets" => cmd_datasets(&args, &positionals).await,
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        other => {
            eprintln!("Unknown --eval subcommand `{}`.\n", other);
            print_help();
            2
        }
    }
}

fn print_help() {
    println!(
        r#"vibecli --eval — VibeCody's evaluation harness

  list                        Show the suites and tasks a run would execute
  run                         Run the suites and write a report
  report [<run-id>|latest]    Print a stored report as Markdown
  runs                        List archived runs
  gate [<run-id>|latest]      Compare against a baseline and exit non-zero on regression
  datasets list               Third-party datasets, and the ones not yet wired
  datasets fetch <id>         Download a dataset into ~/.vibecli/evals/datasets
  datasets import <id>        Convert a dataset into a suite file

Selection (all accept comma-separated lists):
  --suite <ids>               coding-core, code-repair, refactor-multifile,
                              agentic, work-tasks, safety, surfaces
  --capability <slugs>        code_repair, tool_use, work_task, safety, …
  --surface <slugs>           cli, daemon, vscode, mobile, watch, …
  --difficulty <levels>       easy, medium, hard
  --tag <tags>                offline, python, rubric, live, static, …
  --task <substring>          Match against <suite>/<task-id>
  --limit <n>                 Cap the number of tasks (applied last)

Execution:
  --provider <name>           Provider under test (defaults to the CLI's)
  --model <name>              Model under test
  --binary <path>             vibecli binary to evaluate (default: PATH)
  --daemon-url <url>          Daemon base URL for live probes
  --concurrency <n>           Parallel tasks (default 4)
  --samples <n>               Run each task n times (default 1). Agent runs are
                              not deterministic; repeats expose how unstable a
                              result is instead of hiding it behind one roll.
  --timeout <secs>            Per-task budget (default 600)
  --judge-provider <name>     Enable rubric grading with this provider
  --judge-model <name>        Model for rubric grading

Gating:
  --baseline <run-id>         Report to compare against
  --max-regressions <n>       Allowed pass→fail transitions (default 0)
  --min-pass-rate <0..1>      Absolute floor
  --min-coverage <0..1>       Minimum fraction of tasks that must be scored

Output:
  --out <dir>                 Also write report.json / report.md here
  --format json|md            Print format for `report` (default md)

Examples:
  vibecli --eval list --suite safety
  vibecli --eval run --tag offline --provider claude --model claude-opus-5
  vibecli --eval run --suite surfaces --daemon-url http://127.0.0.1:7878
  vibecli --eval gate latest --baseline run-1754000000
"#
    );
}

fn build_runner(args: &Args, provider: &str, model: Option<&str>) -> Runner {
    let binary = args
        .flag("--binary")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("vibecli"));
    let daemon_url = args
        .flag("--daemon-url")
        .unwrap_or_else(|| DaemonConfig::default().base_url);

    let mut runner = Runner::new()
        .with_harness(Arc::new(CliHarness::new(CliConfig {
            binary,
            provider: provider.to_string(),
            model: model.map(str::to_string),
            extra_args: Vec::new(),
            env: Default::default(),
        })))
        .with_harness(Arc::new(DaemonHarness::new(DaemonConfig {
            base_url: daemon_url,
            provider: Some(provider.to_string()),
            model: model.map(str::to_string),
            ..DaemonConfig::default()
        })));

    // Every remaining surface gets a probe harness. Conformance tasks need no
    // agent turn, and registering them explicitly is what keeps a surface's
    // tasks from being reported as "no harness registered" — a skip that
    // reads like a gap in the harness rather than a result.
    for surface in Surface::ALL {
        if !matches!(surface, Surface::Cli | Surface::Daemon) {
            runner = runner.with_harness(Arc::new(ProbeHarness::new(*surface)));
        }
    }
    runner
}

fn base_run_config(args: &Args, provider: &str, model: Option<&str>) -> Result<RunConfig, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let repo_root = find_repo_root(&cwd).ok_or_else(|| {
        format!(
            "could not find `{}` in {} or any parent directory. \
             Run this from a VibeCody checkout, or pass --suite-dir.",
            vibe_eval::DEFAULT_SUITES_DIR,
            cwd.display()
        )
    })?;

    Ok(RunConfig {
        suites_dir: repo_root.join(vibe_eval::DEFAULT_SUITES_DIR),
        repo_root: Some(repo_root),
        filter: build_filter(args)?,
        concurrency: args
            .flag("--concurrency")
            .and_then(|v| v.parse().ok())
            .unwrap_or(4),
        default_timeout: Duration::from_secs(600),
        // An explicit `--timeout` is a ceiling, not a fallback. Every shipped
        // suite sets `defaults.timeout_secs`, so treating the flag as a
        // fallback meant it was never consulted: a task ran its full 900s
        // while the operator had asked for 600 and was told nothing.
        timeout_cap: args
            .flag("--timeout")
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs),
        samples: args
            .flag("--samples")
            .and_then(|v| v.parse().ok())
            .filter(|n| *n >= 1)
            .unwrap_or(1),
        daemon_base_url: Some(
            args.flag("--daemon-url")
                .unwrap_or_else(|| DaemonConfig::default().base_url),
        ),
        judge: None,
        provider: provider.to_string(),
        model: model.unwrap_or_default().to_string(),
        run_id: String::new(),
    })
}

fn cmd_list(args: &Args) -> i32 {
    let (config, runner) = match base_run_config(args, "", None) {
        Ok(c) => {
            let r = build_runner(args, "", None);
            (c, r)
        }
        Err(e) => {
            eprintln!("❌ {}", e);
            return 2;
        }
    };
    let (work, load_errors) = runner.plan(&config);

    for e in &load_errors {
        eprintln!("⚠️  {}", e);
    }
    if work.is_empty() {
        println!("No tasks match this selection.");
        // Nothing to run is not success; a filter typo lands here.
        return 1;
    }

    let mut current_suite = String::new();
    for planned in &work {
        let (task_ref, surface) = (&planned.task, &planned.surface);
        if task_ref.suite != current_suite {
            current_suite = task_ref.suite.clone();
            println!("\n{}", current_suite);
        }
        println!(
            "  {:<34} {:<20} {:<8} {:<10} {}",
            task_ref.task.id,
            task_ref.task.capability.slug(),
            task_ref.task.difficulty.slug(),
            surface.slug(),
            task_ref.task.title
        );
    }
    println!("\n{} task/surface pairs selected.", work.len());
    0
}

async fn cmd_run(args: &Args<'_>, default_provider: &str, default_model: Option<&str>) -> i32 {
    let provider = args
        .flag("--provider")
        .unwrap_or_else(|| default_provider.to_string());
    let model = args
        .flag("--model")
        .or_else(|| default_model.map(str::to_string));

    let mut config = match base_run_config(args, &provider, model.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ {}", e);
            return 2;
        }
    };
    config.run_id = format!(
        "run-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );

    // Rubric grading is opt-in. Without it, judge-graded tasks are reported as
    // skipped — never as passes, and never quietly dropped from the suite.
    if let Some(judge_provider) = args.flag("--judge-provider") {
        let judge_model = args.flag("--judge-model");
        match crate::create_provider(&judge_provider, judge_model.clone()) {
            Ok(p) => {
                let label = format!(
                    "{}:{}",
                    judge_provider,
                    judge_model.as_deref().unwrap_or("default")
                );
                eprintln!("⚖️  Rubric grading enabled via {}", label);
                config.judge = Some(Arc::new(ProviderJudge::new(p, label)));
            }
            Err(e) => {
                eprintln!(
                    "❌ Cannot build the judge provider `{}`: {}",
                    judge_provider, e
                );
                return 2;
            }
        }
    }

    let runner = build_runner(args, &provider, model.as_deref());
    let (work, _) = runner.plan(&config);
    if work.is_empty() {
        eprintln!("❌ No tasks match this selection — nothing was run.");
        return 1;
    }
    eprintln!(
        "▶️  {} task/surface pairs · provider={} model={} · concurrency={}",
        work.len(),
        provider,
        model.as_deref().unwrap_or("(provider default)"),
        config.concurrency
    );

    let report = runner.run(&config).await;

    match save_run(&report) {
        Ok(dir) => eprintln!("📁 Run archived at {}", dir.display()),
        Err(e) => eprintln!("⚠️  Could not archive the run: {}", e),
    }
    if let Some(out) = args.flag("--out") {
        let dir = PathBuf::from(&out);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("⚠️  Cannot create {}: {}", dir.display(), e);
        } else {
            let _ = report
                .to_json()
                .map(|j| std::fs::write(dir.join("report.json"), j));
            let _ = std::fs::write(dir.join("report.md"), report.to_markdown());
            eprintln!("📄 Report written to {}", dir.display());
        }
    }

    println!("{}", report.to_markdown());

    let overall = report.overall();
    // A run that measured nothing is not a success, whatever its pass rate
    // renders as.
    if overall.scored() == 0 {
        eprintln!(
            "❌ No task produced a verdict ({} skipped, {} errored). \
             This says nothing about VibeCody — check the skip reasons above.",
            overall.skipped, overall.errored
        );
        return 1;
    }
    0
}

fn cmd_report(args: &Args, run_id: Option<&str>) -> i32 {
    let id = run_id.unwrap_or("latest");
    match load_run(id) {
        Err(e) => {
            eprintln!("❌ {}", e);
            2
        }
        Ok(report) => {
            let format = args.flag("--format").unwrap_or_else(|| "md".to_string());
            match format.as_str() {
                "json" => match report.to_json() {
                    Ok(j) => {
                        println!("{}", j);
                        0
                    }
                    Err(e) => {
                        eprintln!("❌ {}", e);
                        2
                    }
                },
                _ => {
                    println!("{}", report.to_markdown());
                    0
                }
            }
        }
    }
}

fn cmd_runs() -> i32 {
    let Some(dir) = runs_dir() else {
        eprintln!("❌ HOME is unset");
        return 2;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        println!("No runs yet. `vibecli --eval run` creates one.");
        return 0;
    };
    let mut rows: Vec<(String, String)> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().join("report.json").exists())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let report = load_run(&id).ok()?;
            let t = report.overall();
            let rate = match t.pass_rate() {
                Some(r) => format!("{:.0}%", r * 100.0),
                // Not 0% — see the report module.
                None => "n/a".to_string(),
            };
            Some((
                id,
                format!(
                    "{:>5}  {} passed / {} scored  ({} skipped, {} errored)  {} {}",
                    rate,
                    t.passed,
                    t.scored(),
                    t.skipped,
                    t.errored,
                    report.config.provider,
                    report.config.model
                ),
            ))
        })
        .collect();
    rows.sort();
    if rows.is_empty() {
        println!("No runs yet. `vibecli --eval run` creates one.");
        return 0;
    }
    for (id, summary) in rows {
        println!("{:<24} {}", id, summary);
    }
    0
}

fn cmd_gate(args: &Args, run_id: Option<&str>) -> i32 {
    let current = match load_run(run_id.unwrap_or("latest")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ {}", e);
            return 2;
        }
    };
    let baseline = match args.flag("--baseline") {
        None => None,
        Some(id) => match load_run(&id) {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("❌ baseline: {}", e);
                return 2;
            }
        },
    };
    let comparison = baseline.as_ref().map(|b| gate::compare(b, &current));

    let policy = GatePolicy {
        max_regressions: args
            .flag("--max-regressions")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        max_coverage_losses: args
            .flag("--max-coverage-losses")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        min_pass_rate: args.flag("--min-pass-rate").and_then(|v| v.parse().ok()),
        min_coverage: args.flag("--min-coverage").and_then(|v| v.parse().ok()),
        fail_on_empty_run: !args.has("--allow-empty"),
    };

    if let Some(cmp) = &comparison {
        println!("{}", gate::comparison_markdown(cmp));
    }
    let outcome = gate::evaluate(&current, comparison.as_ref(), &policy);
    for note in &outcome.notes {
        println!("· {}", note);
    }
    if outcome.passed {
        println!("\n✅ Gate passed.");
    } else {
        println!("\n❌ Gate failed:");
        for v in &outcome.violations {
            println!("  • {}", v);
        }
    }
    outcome.exit_code()
}

async fn cmd_datasets(args: &Args<'_>, positionals: &[&str]) -> i32 {
    match positionals.get(1).copied().unwrap_or("list") {
        "list" => {
            println!("Wired datasets (importable):\n");
            for m in Registry::manifests() {
                println!("  {:<20} {}", m.id, m.name);
                println!("    licence : {}", m.license);
                println!("    source  : {}", m.homepage);
                println!(
                    "    maps to : {} / {}\n",
                    m.capability.slug(),
                    m.difficulty.slug()
                );
            }
            println!("Known gaps (not wired, and why):\n");
            for g in Registry::gaps() {
                println!("  {:<20} {}", g.id, g.name);
                println!("    measures    : {}", g.measures);
                println!("    blocked on  : {}\n", g.blocked_on);
            }
            println!("{}\n", Registry::comparability_note());
            0
        }
        "fetch" => {
            let Some(id) = positionals.get(2) else {
                eprintln!("Usage: vibecli --eval datasets fetch <id>");
                return 2;
            };
            let Some(manifest) = Registry::find(id) else {
                eprintln!("❌ Unknown dataset `{}`. Try `--eval datasets list`.", id);
                return 2;
            };
            eprintln!(
                "⚖️  {} is licensed {}. {}",
                manifest.name, manifest.license, manifest.license_note
            );
            match dataset::fetch(&manifest).await {
                Err(e) => {
                    eprintln!("❌ {}", e);
                    1
                }
                Ok(f) => {
                    println!("Downloaded {} bytes to {}", f.bytes, f.path.display());
                    println!("sha256: {}", f.sha256);
                    if manifest.sha256.is_none() {
                        // Without a pin, a benchmark can change under you and
                        // every historical comparison silently stops meaning
                        // the same thing.
                        println!(
                            "\nThis dataset has no pinned digest. Add\n    sha256: {}\n\
                             to its manifest to make future runs comparable.",
                            f.sha256
                        );
                    }
                    0
                }
            }
        }
        "import" => {
            let Some(id) = positionals.get(2) else {
                eprintln!("Usage: vibecli --eval datasets import <id> [--limit N]");
                return 2;
            };
            let Some(manifest) = Registry::find(id) else {
                eprintln!("❌ Unknown dataset `{}`.", id);
                return 2;
            };
            let fetched = match dataset::fetch(&manifest).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("❌ {}", e);
                    return 1;
                }
            };
            let text = match std::fs::read_to_string(&fetched.path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "❌ Cannot read {}: {}. If the download is compressed, \
                         decompress it in place first.",
                        fetched.path.display(),
                        e
                    );
                    return 1;
                }
            };
            let rows = match dataset::parse_rows(&text, manifest.format) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("❌ {}", e);
                    return 1;
                }
            };
            let limit = args.flag("--limit").and_then(|v| v.parse().ok());
            let (tasks, errors) = dataset::import(&manifest, &rows, limit);
            for e in errors.iter().take(5) {
                eprintln!("⚠️  {}", e);
            }
            if errors.len() > 5 {
                eprintln!("⚠️  …and {} more row(s) skipped", errors.len() - 5);
            }
            if tasks.is_empty() {
                eprintln!("❌ No importable rows in {}.", fetched.path.display());
                return 1;
            }
            let yaml = match dataset::to_suite_yaml(&manifest, &tasks) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!("❌ {}", e);
                    return 1;
                }
            };
            let cwd = std::env::current_dir().unwrap_or_default();
            let Some(root) = find_repo_root(&cwd) else {
                eprintln!("❌ Not inside a VibeCody checkout.");
                return 2;
            };
            let out = dataset::imported_suite_path(
                &root.join(vibe_eval::DEFAULT_SUITES_DIR),
                &manifest.id,
            );
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&out, yaml) {
                Err(e) => {
                    eprintln!("❌ Cannot write {}: {}", out.display(), e);
                    1
                }
                Ok(()) => {
                    println!(
                        "Imported {} task(s) from {} to {}",
                        tasks.len(),
                        manifest.name,
                        out.display()
                    );
                    println!("\n{}", Registry::comparability_note());
                    0
                }
            }
        }
        other => {
            eprintln!(
                "Unknown datasets subcommand `{}`. Available: list, fetch, import",
                other
            );
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flags_parse_in_both_forms() {
        let raw = args(&["run", "--suite", "safety", "--limit=5"]);
        let a = Args(&raw);
        assert_eq!(a.flag("--suite").as_deref(), Some("safety"));
        assert_eq!(a.flag("--limit").as_deref(), Some("5"));
    }

    #[test]
    fn positionals_skip_flag_values() {
        // Without this, `--eval --suite safety list` would read `safety` as
        // the subcommand and run nothing.
        let raw = args(&["--suite", "safety", "list"]);
        let a = Args(&raw);
        assert_eq!(a.positionals(), vec!["list"]);
    }

    #[test]
    fn comma_lists_split() {
        let raw = args(&["run", "--capability", "code_repair, debugging"]);
        let a = Args(&raw);
        assert_eq!(a.list("--capability"), vec!["code_repair", "debugging"]);
    }

    #[test]
    fn a_misspelled_capability_is_an_error_not_an_empty_selection() {
        // Selecting zero tasks and printing a clean result is the quiet
        // failure this harness is supposed to make impossible.
        let raw = args(&["run", "--capability", "code_repare"]);
        let err = build_filter(&Args(&raw)).expect_err("should reject");
        assert!(err.contains("code_repare"), "{}", err);
        assert!(err.contains("code_repair"), "should list the valid names");
    }

    #[test]
    fn valid_filters_parse() {
        let raw = args(&[
            "run",
            "--capability",
            "code_repair",
            "--surface",
            "cli,daemon",
            "--difficulty",
            "hard",
            "--tag",
            "offline",
            "--limit",
            "3",
        ]);
        let f = build_filter(&Args(&raw)).expect("parse");
        assert_eq!(f.capabilities, vec![Capability::CodeRepair]);
        assert_eq!(f.surfaces, vec![Surface::Cli, Surface::Daemon]);
        assert_eq!(f.difficulties, vec![Difficulty::Hard]);
        assert_eq!(f.limit, Some(3));
    }

    #[test]
    fn judge_reply_parses_through_fences_and_prose() {
        let reply = "Here is my assessment:\n```json\n\
                     {\"score\": 0.85, \"rationale\": \"Covers the blockers.\"}\n```";
        let parsed = parse_judge_reply(reply).expect("parse");
        assert_eq!(parsed.score, 0.85);
        assert!(parsed.rationale.contains("blockers"));
    }

    #[test]
    fn an_unreadable_judge_reply_errors_rather_than_scoring_zero() {
        // A zero here would be indistinguishable from the judge saying the
        // work was bad.
        assert!(parse_judge_reply("I think it was pretty good, honestly.").is_err());
        assert!(parse_judge_reply("{\"rationale\": \"no score field\"}").is_err());
        assert!(parse_judge_reply("").is_err());
    }

    #[test]
    fn a_judge_score_outside_the_range_is_rejected() {
        assert!(parse_judge_reply("{\"score\": 8.5}").is_err());
        assert!(parse_judge_reply("{\"score\": -1}").is_err());
        assert!(parse_judge_reply("{\"score\": 1.0}").is_ok());
    }

    #[test]
    fn surface_and_capability_slugs_parse_back() {
        for c in Capability::ALL {
            assert_eq!(parse_capability(c.slug()), Some(*c));
        }
        for s in Surface::ALL {
            assert_eq!(parse_surface(s.slug()), Some(*s));
        }
    }

    #[test]
    fn repo_root_is_found_by_walking_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::create_dir_all(dir.path().join(vibe_eval::DEFAULT_SUITES_DIR)).expect("mkdir");
        assert_eq!(
            find_repo_root(&nested).as_deref(),
            Some(dir.path()),
            "should walk up to the checkout root"
        );
    }

    #[test]
    fn repo_root_is_absent_outside_a_checkout() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(find_repo_root(dir.path()).is_none());
    }
}
