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
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    unix_secs_to_date_string(secs)
}

/// Convert a unix timestamp (seconds since 1970-01-01) to a "YYYY-MM-DD" date string.
/// Civil date algorithm handles leap years correctly.
fn unix_secs_to_date_string(secs: u64) -> String {
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

/// Extract bullet lines from a response string.
/// Returns lines whose trimmed form starts with '-'.
#[cfg(test)]
fn extract_bullets(response: &str) -> Vec<&str> {
    response
        .lines()
        .filter(|l| l.trim_start().starts_with('-'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_1970_01_01() {
        assert_eq!(unix_secs_to_date_string(0), "1970-01-01");
    }

    #[test]
    fn known_date_2024_01_01() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(unix_secs_to_date_string(1_704_067_200), "2024-01-01");
    }

    #[test]
    fn leap_day_2024_02_29() {
        // 2024-02-29 00:00:00 UTC = 1709164800
        assert_eq!(unix_secs_to_date_string(1_709_164_800), "2024-02-29");
    }

    #[test]
    fn known_date_2026_03_06() {
        // 2026-03-06 00:00:00 UTC = 1772784000
        assert_eq!(unix_secs_to_date_string(1_772_784_000), "2026-03-06");
    }

    #[test]
    fn end_of_day_still_same_date() {
        // 2024-01-01 23:59:59 UTC = 1704067200 + 86399 = 1704153599
        assert_eq!(unix_secs_to_date_string(1_704_153_599), "2024-01-01");
    }

    #[test]
    fn memory_file_path_uses_home_env() {
        // memory_file_path reads $HOME; verify it returns the expected suffix
        let path = memory_file_path();
        assert!(path.ends_with(".vibecli/memory.md"));
    }

    #[test]
    fn extract_bullets_filters_correctly() {
        let response = "Here is the summary:\n- Learning one\n- Learning two\nSome other text\n  - Indented bullet";
        let bullets = extract_bullets(response);
        assert_eq!(bullets.len(), 3);
        assert_eq!(bullets[0], "- Learning one");
        assert_eq!(bullets[1], "- Learning two");
        assert_eq!(bullets[2], "  - Indented bullet");
    }

    #[test]
    fn extract_bullets_empty_response() {
        let bullets = extract_bullets("");
        assert!(bullets.is_empty());
    }

    #[test]
    fn extract_bullets_no_bullets() {
        let response = "No bullet points here.\nJust plain text.";
        let bullets = extract_bullets(response);
        assert!(bullets.is_empty());
    }

    #[test]
    fn unix_secs_2000_01_01() {
        // 2000-01-01 00:00:00 UTC = 946684800
        assert_eq!(unix_secs_to_date_string(946_684_800), "2000-01-01");
    }

    #[test]
    fn unix_secs_dec_31() {
        // 2023-12-31 00:00:00 UTC = 1703980800
        assert_eq!(unix_secs_to_date_string(1_703_980_800), "2023-12-31");
    }

    #[test]
    fn unix_secs_non_leap_feb_28() {
        // 2023-02-28 00:00:00 UTC = 1677542400
        assert_eq!(unix_secs_to_date_string(1_677_542_400), "2023-02-28");
    }

    #[test]
    fn unix_secs_mid_day() {
        // 2024-06-15 12:00:00 UTC = 1718452800
        assert_eq!(unix_secs_to_date_string(1_718_452_800), "2024-06-15");
    }

    #[test]
    fn extract_bullets_only_bullets() {
        let response = "- first\n- second\n- third";
        let bullets = extract_bullets(response);
        assert_eq!(bullets.len(), 3);
    }

    #[test]
    fn extract_bullets_mixed_with_whitespace() {
        let response = "Header\n\n  - indented one\n\n- normal one\n\nFooter";
        let bullets = extract_bullets(response);
        assert_eq!(bullets.len(), 2);
        assert_eq!(bullets[0].trim(), "- indented one");
        assert_eq!(bullets[1], "- normal one");
    }

    #[test]
    fn extract_bullets_dash_in_middle_of_line_not_counted() {
        let response = "This has a - dash in the middle";
        let bullets = extract_bullets(response);
        assert!(bullets.is_empty());
    }

    #[test]
    fn memory_file_path_is_absolute() {
        let path = memory_file_path();
        // The path should contain .vibecli/memory.md
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".vibecli"));
        assert!(path_str.ends_with("memory.md"));
    }

    #[test]
    fn chrono_now_utc_format() {
        let date = chrono_now_utc();
        // Should match YYYY-MM-DD format
        assert_eq!(date.len(), 10);
        assert_eq!(&date[4..5], "-");
        assert_eq!(&date[7..8], "-");
        // Year should parse as a number
        assert!(date[0..4].parse::<u32>().is_ok());
        // Month should be 01-12
        let month: u32 = date[5..7].parse().unwrap();
        assert!((1..=12).contains(&month));
        // Day should be 01-31
        let day: u32 = date[8..10].parse().unwrap();
        assert!((1..=31).contains(&day));
    }
}
