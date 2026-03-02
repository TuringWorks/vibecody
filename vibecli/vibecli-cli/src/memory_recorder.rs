//! Auto memory recording: after a completed agent session, ask the LLM to
//! summarize key learnings and append 1–3 bullet points to `~/.vibecli/memory.md`.
//!
//! Feature is opt-in: `[memory] auto_record = true` in `~/.vibecli/config.toml`.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use vibe_ai::provider::{AIProvider, Message, MessageRole};

/// Append 1-3 learning bullet points from the session to `~/.vibecli/memory.md`.
///
/// * `provider` — the LLM used during the session (reused for summarisation)
/// * `task`     — the original user task description
/// * `steps`    — number of tool-use steps executed
/// * `summary`  — the agent's final `AgentEvent::Complete` summary text
pub async fn record_session(
    provider: Arc<dyn AIProvider>,
    task: &str,
    steps: usize,
    summary: &str,
) -> Result<()> {
    // Compose a short summarisation prompt
    let prompt = format!(
        "You are a memory assistant. A coding agent just completed a task.\n\n\
Task: {task}\n\
Steps executed: {steps}\n\
Summary: {summary}\n\n\
Extract 1-3 concise learning bullet points that would be useful to remember \
for future similar tasks. Each bullet should be ≤ 25 words.\n\
Respond ONLY with the bullet points in this exact format:\n\
- <learning 1>\n\
- <learning 2>\n\
(no other text)"
    );

    let messages = vec![Message {
        role: MessageRole::User,
        content: prompt,
    }];

    let response = provider.chat(&messages, None).await?;

    // Extract lines that start with `- `
    let bullets: Vec<&str> = response
        .lines()
        .filter(|l| l.trim_start().starts_with('-'))
        .collect();

    if bullets.is_empty() {
        return Ok(());
    }

    // Append to memory.md
    let memory_path = memory_file_path();
    if let Some(parent) = memory_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let header = format!(
        "\n<!-- auto-recorded {} -->\n",
        chrono_now_utc()
    );
    let entry = format!("{}{}\n", header, bullets.join("\n"));

    // Append (or create)
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_path)?;
    file.write_all(entry.as_bytes())?;

    tracing::info!(
        "Auto-recorded {} bullet(s) to {}",
        bullets.len(),
        memory_path.display()
    );
    Ok(())
}

fn memory_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibecli").join("memory.md")
}

fn chrono_now_utc() -> String {
    // Use SystemTime to avoid pulling in chrono
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date from unix timestamp (handles leap years correctly)
    let mut days = (secs / 86400) as i64;
    // Shift epoch from 1970-01-01 to 0000-03-01 for easier leap year math
    days += 719468; // days from 0000-03-01 to 1970-01-01
    let era = days / 146097; // 400-year era
    let doe = days - era * 146097; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month starting from March=0
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}
