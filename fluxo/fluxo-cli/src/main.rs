//! `fluxo` — a thin command-line client over the `fluxo-server` HTTP API.
//!
//! ```text
//! fluxo register def.json            # register a workflow definition
//! fluxo run order --input '{"a":1}'  # start a run (add --wait to tail to completion)
//! fluxo get <workflow_id>            # show a run
//! fluxo tail <workflow_id>           # stream the run's timeline (SSE) until terminal
//! fluxo ls                           # list registered definitions
//! fluxo runs --status RUNNING        # list runs
//! fluxo signal <workflow_id> <ref>   # complete a WAIT/HUMAN task
//! ```
//!
//! The server URL comes from `--url` or `$FLUXO_URL` (default `http://127.0.0.1:8080`).

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fluxo", version, about = "Fluxo workflow CLI")]
struct Cli {
    /// Base URL of the fluxo-server.
    #[arg(long, env = "FLUXO_URL", default_value = "http://127.0.0.1:8080", global = true)]
    url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register a workflow definition from a JSON file.
    Register {
        /// Path to a Conductor-compatible workflow JSON file.
        file: PathBuf,
    },
    /// Start a run of a registered workflow.
    Run {
        /// Workflow name.
        name: String,
        /// Pin a specific version (default: latest).
        #[arg(long)]
        version: Option<u32>,
        /// Inline JSON input.
        #[arg(long, conflicts_with = "input_file")]
        input: Option<String>,
        /// Read JSON input from a file.
        #[arg(long)]
        input_file: Option<PathBuf>,
        /// Correlation id to attach.
        #[arg(long)]
        correlation_id: Option<String>,
        /// Tail the run to completion after starting it.
        #[arg(long)]
        wait: bool,
    },
    /// Show a run by id.
    Get {
        /// Workflow run id.
        workflow_id: String,
    },
    /// Stream a run's timeline (SSE) until it reaches a terminal state.
    Tail {
        /// Workflow run id.
        workflow_id: String,
    },
    /// List registered workflow definitions.
    Ls,
    /// List runs, optionally filtered by status.
    Runs {
        /// Filter: RUNNING, COMPLETED, FAILED, TERMINATED, PAUSED, TIMED_OUT.
        #[arg(long)]
        status: Option<String>,
    },
    /// Complete a WAIT/HUMAN task, resuming the run.
    Signal {
        /// Workflow run id.
        workflow_id: String,
        /// Task reference name to signal.
        reference_name: String,
        /// Inline JSON output to attach.
        #[arg(long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base = cli.url.trim_end_matches('/').to_string();

    match cli.command {
        Command::Register { file } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let def: Value = serde_json::from_str(&text).context("workflow file is not valid JSON")?;
            let body = post(&client, &format!("{base}/workflow"), def).await?;
            print_json(&body.unwrap_or(Value::Null));
        }
        Command::Run { name, version, input, input_file, correlation_id, wait } => {
            let input_value = load_input(input, input_file)?;
            let mut req = json!({ "input": input_value });
            if let Some(v) = version {
                req["version"] = json!(v);
            }
            if let Some(c) = correlation_id {
                req["correlationId"] = json!(c);
            }
            let body = post(&client, &format!("{base}/workflow/{name}/execute"), req).await?;
            let body = body.unwrap_or(Value::Null);
            print_json(&body);
            if wait {
                if let Some(id) = body.get("workflowId").and_then(Value::as_str) {
                    eprintln!("--- tailing {id} ---");
                    tail(&client, &base, id).await?;
                }
            }
        }
        Command::Get { workflow_id } => {
            let body = get(&client, &format!("{base}/workflow/run/{workflow_id}")).await?;
            print_json(&body);
        }
        Command::Tail { workflow_id } => {
            tail(&client, &base, &workflow_id).await?;
        }
        Command::Ls => {
            let body = get(&client, &format!("{base}/workflow")).await?;
            print_json(&body);
        }
        Command::Runs { status } => {
            let url = match status {
                Some(s) => format!("{base}/runs?status={s}"),
                None => format!("{base}/runs"),
            };
            let body = get(&client, &url).await?;
            print_json(&body);
        }
        Command::Signal { workflow_id, reference_name, output } => {
            let output_value = match output {
                Some(s) => serde_json::from_str(&s).context("--output is not valid JSON")?,
                None => json!({}),
            };
            post(
                &client,
                &format!("{base}/workflow/run/{workflow_id}/signal"),
                json!({ "referenceName": reference_name, "output": output_value }),
            )
            .await?;
            eprintln!("signalled {reference_name}");
        }
    }
    Ok(())
}

fn load_input(inline: Option<String>, file: Option<PathBuf>) -> Result<Value> {
    match (inline, file) {
        (Some(s), _) => serde_json::from_str(&s).context("--input is not valid JSON"),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str(&text).context("--input-file is not valid JSON")
        }
        (None, None) => Ok(json!({})),
    }
}

async fn get(client: &reqwest::Client, url: &str) -> Result<Value> {
    let response = client.get(url).send().await.context("request failed")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("{} {}: {}", status.as_u16(), url, text);
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

async fn post(client: &reqwest::Client, url: &str, body: Value) -> Result<Option<Value>> {
    let response = client.post(url).json(&body).send().await.context("request failed")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("{} {}: {}", status.as_u16(), url, text);
    }
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::from_str(&text).unwrap_or(Value::Null)))
    }
}

/// Stream the SSE timeline, printing each snapshot, until the run is terminal.
async fn tail(client: &reqwest::Client, base: &str, workflow_id: &str) -> Result<()> {
    let url = format!("{base}/workflow/run/{workflow_id}/stream");
    let response = client.get(&url).send().await.context("stream request failed")?;
    if !response.status().is_success() {
        bail!("{} {}", response.status().as_u16(), url);
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut data_lines: Vec<String> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("stream error")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline) = buffer.find('\n') {
            let line = buffer[..newline].trim_end_matches('\r').to_string();
            buffer.drain(..=newline);

            if line.is_empty() {
                if data_lines.is_empty() {
                    continue;
                }
                let data = std::mem::take(&mut data_lines).join("\n");
                let terminal = print_event(&data);
                if terminal {
                    return Ok(());
                }
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }
    }
    Ok(())
}

/// Print one SSE data payload; return true if it reports a terminal status.
fn print_event(data: &str) -> bool {
    match serde_json::from_str::<Value>(data) {
        Ok(value) => {
            print_json(&value);
            value
                .get("status")
                .and_then(Value::as_str)
                .map(is_terminal)
                .unwrap_or(false)
        }
        Err(_) => {
            println!("{data}");
            false
        }
    }
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "COMPLETED" | "FAILED" | "TERMINATED" | "TIMED_OUT")
}

fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!("{value}"),
    }
}
