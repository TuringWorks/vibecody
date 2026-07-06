//! `skilllensai` CLI (feature = "cli").
//!
//! Phase 1 verbs (no LLM, no key):
//!   - `convert <input.jsonl>` — normalise raw runs to trajectory JSONL.
//!   - `report  <skills_dir>`  — portfolio table over `skills/*.md`.
//!
//! `extract` / `score` (LLM-backed) arrive in Phase 2.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use skilllensai::{report::SkillReport, Skill};

#[derive(Parser)]
#[command(
    name = "skilllensai",
    version,
    about = "Analyse & measure agent-skill utility"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Normalise raw agent runs (JSONL of RawRun) into trajectory JSONL.
    Convert {
        /// Input file of one RawRun JSON object per line.
        input: PathBuf,
        /// Write trajectories here instead of stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Portfolio report over a `skills/*.md` directory.
    Report {
        /// Directory containing `*.md` skill files.
        skills_dir: PathBuf,
        /// Optional file of observed intents (one per line) for trigger coverage.
        #[arg(long)]
        intents: Option<PathBuf>,
        /// Emit a markdown table instead of the aligned text table.
        #[arg(long)]
        markdown: bool,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Convert { input, out } => {
            let raw = std::fs::read_to_string(&input)?;
            let pool = skilllensai::convert::convert_jsonl(&raw)?;
            let jsonl = pool.to_jsonl();
            match out {
                Some(p) => {
                    std::fs::write(&p, jsonl)?;
                    eprintln!("wrote {} trajectories to {}", pool.len(), p.display());
                }
                None => println!("{jsonl}"),
            }
        }
        Cmd::Report {
            skills_dir,
            intents,
            markdown,
        } => {
            let intents_vec: Vec<String> = match intents {
                Some(p) => std::fs::read_to_string(&p)?
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect(),
                None => Vec::new(),
            };

            let mut skills = Vec::new();
            for entry in std::fs::read_dir(&skills_dir)? {
                let path = entry?.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    skills.push(Skill::from_file(&path)?);
                }
            }
            skills.sort_by(|a, b| a.name.cmp(&b.name));

            let reports: Vec<SkillReport> = skills
                .iter()
                .map(|s| SkillReport::measure_static(s, &intents_vec))
                .collect();

            if markdown {
                print!("{}", skilllensai::report::portfolio_markdown(&reports));
            } else {
                println!(
                    "{:<44} {:<22} {:>7} {:>9}",
                    "skill", "category", "tokens", "coverage"
                );
                for r in &reports {
                    let cov = if intents_vec.is_empty() {
                        "—".to_string()
                    } else {
                        format!("{:.2}", r.trigger_coverage)
                    };
                    println!(
                        "{:<44} {:<22} {:>7} {:>9}",
                        truncate(&r.skill, 44),
                        truncate(&r.category, 22),
                        r.token_cost,
                        cov
                    );
                }
            }
            eprintln!("{} skills", reports.len());
        }
    }
    Ok(())
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
