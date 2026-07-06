//! `skilloptai` CLI (feature = "cli").
//!
//! Verbs:
//!   - `train <skill.md> --tasks <tasks.jsonl>` — train one skill against a
//!     static task set; writes `<out>` (`*.opt.md`, never overwriting a shipped
//!     skill) and an optional `--report` markdown.
//!   - `propose <skill.md> --tasks <tasks.jsonl>` — one-epoch dry-run: print the
//!     textual gradient and accept nothing.
//!   - `replay <run.db>` — replay a prior run deterministically (deferred).
//!
//! LLM access is provider-agnostic: the CLI ships an OpenAI-compatible chat
//! client (`--base-url`, `--model`, `--api-key`) that works against any
//! OpenAI-shaped endpoint (OpenAI, Groq, OpenRouter, local Ollama/LM Studio).
//! The VibeCody daemon wires the *real* provider-selected backend via the
//! bridge (Phase 3); this standalone client is the publishable fallback.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use skilllensai::llm::{LlmDescriptor, SkillLlm};
use skilllensai::metrics::eval::EvalTask;
use skilllensai::model::skill::Skill;
use skilloptai::env::{Env, StaticEnv};
use skilloptai::report::TrainingReport;
use skilloptai::trainer::{train, TrainConfig};

#[derive(Parser)]
#[command(
    name = "skilloptai",
    version,
    about = "Train agent-skill documents via textual-gradient optimization"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Train a skill against a static task set; write the trained `*.opt.md`.
    Train {
        /// Seed skill markdown file.
        skill: PathBuf,
        /// Tasks JSONL file (one EvalTask per line).
        #[arg(long)]
        tasks: PathBuf,
        /// Output trained skill path (e.g. `formal-verification.opt.md`).
        #[arg(short, long)]
        out: PathBuf,
        /// Optional report markdown path.
        #[arg(long)]
        report: Option<PathBuf>,
        #[command(flatten)]
        llm: LlmArgs,
        #[command(flatten)]
        cfg: TrainCfgArgs,
    },
    /// One-epoch dry-run: print the proposed edits and accept nothing.
    Propose {
        /// Seed skill markdown file.
        skill: PathBuf,
        /// Tasks JSONL file.
        #[arg(long)]
        tasks: PathBuf,
        #[command(flatten)]
        llm: LlmArgs,
        #[command(flatten)]
        cfg: TrainCfgArgs,
    },
    /// Replay a prior training run deterministically (deferred — Phase 5).
    Replay {
        /// Prior run DB.
        run_db: PathBuf,
    },
}

/// Shared LLM backend args (OpenAI-compatible).
#[derive(clap::Args, Debug)]
struct LlmArgs {
    /// OpenAI-compatible base URL (no `/chat/completions` suffix).
    #[arg(long, default_value = "https://api.openai.com/v1")]
    base_url: String,
    /// Model id (provider-agnostic — pick one your endpoint serves).
    #[arg(long, default_value = "gpt-4o-mini")]
    model: String,
    /// API key. Falls back to `OPENAI_API_KEY` when omitted.
    #[arg(long)]
    api_key: Option<String>,
}

/// Shared training-config args.
#[derive(clap::Args, Debug)]
struct TrainCfgArgs {
    #[arg(long, default_value_t = 8)]
    epochs: usize,
    #[arg(long, default_value_t = 4)]
    rollouts_per_epoch: usize,
    #[arg(long, default_value_t = 512)]
    textual_lr: usize,
    #[arg(long, default_value_t = 0.3)]
    val_split: f32,
    #[arg(long, default_value_t = 2000)]
    max_skill_tokens: usize,
    #[arg(long, default_value_t = 3)]
    patience: usize,
    #[arg(long, default_value_t = 2)]
    select_k: usize,
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

impl TrainCfgArgs {
    fn to_config(&self) -> TrainConfig {
        TrainConfig {
            epochs: self.epochs,
            rollouts_per_epoch: self.rollouts_per_epoch,
            textual_lr: self.textual_lr,
            val_split: self.val_split,
            max_skill_tokens: self.max_skill_tokens,
            patience: self.patience,
            select_k: self.select_k,
            seed: self.seed,
        }
    }
}

/// An OpenAI-compatible chat client implementing [`SkillLlm`]. Provider-agnostic
/// — works against any endpoint that speaks `/chat/completions`.
struct OpenAiCompatLlm {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiCompatLlm {
    fn build(args: &LlmArgs) -> Result<Self> {
        let api_key = args
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .context("no API key: pass --api-key or set OPENAI_API_KEY")?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self {
            client,
            base_url: args.base_url.trim_end_matches('/').to_string(),
            model: args.model.clone(),
            api_key,
        })
    }
}

#[async_trait]
impl SkillLlm for OpenAiCompatLlm {
    fn descriptor(&self) -> LlmDescriptor {
        LlmDescriptor {
            provider: "openai-compat".to_string(),
            model: self.model.clone(),
        }
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        let status = resp.status();
        let text = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {url}: {}", truncate(&text, 500));
        }
        let parsed: ChatCompletion = serde_json::from_str(&text)
            .with_context(|| format!("decoding chat completion: {}", truncate(&text, 200)))?;
        let content = parsed
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .ok_or_else(|| anyhow::anyhow!("no choices in response"))?;
        Ok(content.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletion {
    choices: Vec<Choice>,
}
#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    message: Message,
}
#[derive(Debug, Deserialize, Serialize)]
struct Message {
    content: String,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn load_skill(path: &std::path::Path) -> Result<Skill> {
    Skill::from_file(path).with_context(|| format!("reading skill {}", path.display()))
}

fn load_env(tasks: &std::path::Path) -> Result<StaticEnv> {
    StaticEnv::from_jsonl_path(tasks).with_context(|| format!("reading tasks {}", tasks.display()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Train {
            skill,
            tasks,
            out,
            report,
            llm,
            cfg,
        } => {
            let seed = load_skill(&skill)?;
            let env = load_env(&tasks)?;
            let llm = OpenAiCompatLlm::build(&llm)?;
            let config = cfg.to_config();

            eprintln!(
                "training {} against {} tasks ({} train / {} val), {} epochs, target {}/{}, seed {}",
                seed.name,
                env.tasks().len(),
                ((env.tasks().len() as f32) * (1.0 - config.val_split)).round() as usize,
                ((env.tasks().len() as f32) * config.val_split).round() as usize,
                config.epochs,
                llm.descriptor().provider,
                llm.descriptor().model,
                config.seed,
            );

            let report_obj: TrainingReport = train(seed, &env, &llm, &config).await?;
            report_obj
                .write_best_skill(&out)
                .with_context(|| format!("writing {}", out.display()))?;
            eprintln!(
                "wrote {} · accepted {}/{} · best val {:.3} · spent {} tokens",
                out.display(),
                report_obj.accepted,
                report_obj.rejected,
                report_obj.best_val_score,
                report_obj.spent_tokens,
            );
            if let Some(rp) = report {
                std::fs::write(&rp, report_obj.to_markdown())
                    .with_context(|| format!("writing {}", rp.display()))?;
                eprintln!("wrote {} (report)", rp.display());
            }
        }
        Cmd::Propose {
            skill,
            tasks,
            llm,
            cfg,
        } => {
            use skilloptai::buffer::RejectedEditBuffer;
            use skilloptai::propose::propose;
            use skilloptai::rollout::{rollout, select_failures};

            let seed = load_skill(&skill)?;
            let env = load_env(&tasks)?;
            let llm = OpenAiCompatLlm::build(&llm)?;
            let mut config = cfg.to_config();
            config.epochs = 1; // one-epoch dry-run

            let (train_tasks, _val) = env.split(config.val_split, config.seed);
            let sample: Vec<EvalTask> = train_tasks
                .iter()
                .take(config.rollouts_per_epoch)
                .cloned()
                .collect();
            let roll = rollout(&seed, &sample, &llm).await?;
            let targets = select_failures(&roll.trajectories, config.select_k);
            let rejected = RejectedEditBuffer::new();
            let prop = propose(&seed, &targets, &rejected, config.textual_lr, &llm).await?;

            println!("=== proposed edits (dry-run, nothing accepted) ===");
            if prop.edits.is_empty() {
                println!("  (none — propose returned no usable edits)");
            }
            for (i, e) in prop.edits.iter().enumerate() {
                println!("  {i}: {} (cost {})", e.label(), e.char_cost());
            }
            if prop.dropped_as_rejected > 0 {
                println!(
                    "  ({} edit(s) dropped as already-rejected)",
                    prop.dropped_as_rejected
                );
            }
            println!("=== raw model response ===");
            println!("{}", prop.raw_response);
        }
        Cmd::Replay { run_db: _ } => {
            eprintln!("skilloptai replay: not yet implemented (Phase 5). See notes/skillforge/05.");
            std::process::exit(2);
        }
    }
    Ok(())
}
