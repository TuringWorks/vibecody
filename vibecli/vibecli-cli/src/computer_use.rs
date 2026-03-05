//! Computer Use / Visual Self-Testing — agents can launch apps, take
//! screenshots, and make visual assertions via an LLM.

use anyhow::Result;
use serde::{Deserialize, Serialize};

// ── Data Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAssertion {
    pub screenshot_path: String,
    pub assertion: String,
    pub passed: bool,
    pub confidence: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualTestStep {
    pub action: String,
    pub screenshot: Option<ScreenshotResult>,
    pub assertion: Option<VisualAssertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualTestSession {
    pub id: String,
    pub url: String,
    pub steps: Vec<VisualTestStep>,
    pub passed: bool,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

// ── Screenshot capture ──────────────────────────────────────────────────────

/// Take a screenshot using platform-native tools.
///
/// - macOS: `screencapture -x`
/// - Linux: `scrot`
/// - Windows: PowerShell screen-capture
pub fn take_screenshot(output_path: &std::path::Path) -> Result<ScreenshotResult> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cmd = if cfg!(target_os = "macos") {
        format!("screencapture -x {}", output_path.display())
    } else if cfg!(target_os = "linux") {
        format!("scrot {}", output_path.display())
    } else {
        // Windows: PowerShell screenshot
        format!(
            "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
             [System.Windows.Forms.Screen]::PrimaryScreen | ForEach-Object {{ \
             $bitmap = New-Object System.Drawing.Bitmap($_.Bounds.Width, $_.Bounds.Height); \
             $graphics = [System.Drawing.Graphics]::FromImage($bitmap); \
             $graphics.CopyFromScreen($_.Bounds.Location, [System.Drawing.Point]::Empty, $_.Bounds.Size); \
             $bitmap.Save('{}') }}\"",
            output_path.display()
        )
    };

    let output = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .output()
        .map_err(|e| anyhow::anyhow!("Screenshot failed: {}", e))?;

    if !output.status.success() {
        anyhow::bail!(
            "Screenshot command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Try to read PNG dimensions from the file header, fall back to defaults.
    let (width, height) = read_png_dimensions(output_path).unwrap_or((1920, 1080));

    Ok(ScreenshotResult {
        path: output_path.to_string_lossy().to_string(),
        width,
        height,
        timestamp: now,
    })
}

/// Attempt to read width/height from a PNG file header (IHDR chunk).
fn read_png_dimensions(path: &std::path::Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    // PNG: 8-byte signature, then IHDR chunk: 4-byte length, "IHDR", 4-byte width, 4-byte height
    if data.len() < 24 {
        return None;
    }
    // Check PNG signature
    if &data[0..4] != b"\x89PNG" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return None;
    }
    Some((width, height))
}

// ── LLM-based visual assertion ──────────────────────────────────────────────

/// Build a prompt asking an LLM to evaluate a visual assertion against a
/// screenshot. The caller is responsible for attaching the actual screenshot
/// image when invoking the LLM.
pub fn build_visual_assert_prompt(assertion: &str, screenshot_path: &str) -> String {
    format!(
        "You are a visual QA tester. You have been shown a screenshot of an application \
         at '{screenshot_path}'.\n\n\
         Please evaluate this visual assertion:\n\"{assertion}\"\n\n\
         Respond with JSON:\n\
         {{\"passed\": true/false, \"confidence\": 0.0-1.0, \"details\": \"explanation\"}}"
    )
}

/// Parse an LLM response into a [`VisualAssertion`].
///
/// Tries JSON first; falls back to keyword heuristics if the response is
/// free-form text.
pub fn parse_visual_assertion(
    response: &str,
    screenshot_path: &str,
    assertion: &str,
) -> VisualAssertion {
    // Try to extract JSON from the response (may be wrapped in markdown fences).
    let json_str = extract_json_block(response);

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        return VisualAssertion {
            screenshot_path: screenshot_path.to_string(),
            assertion: assertion.to_string(),
            passed: v["passed"].as_bool().unwrap_or(false),
            confidence: v["confidence"].as_f64().unwrap_or(0.5),
            details: v["details"]
                .as_str()
                .unwrap_or("No details")
                .to_string(),
        };
    }

    // Fallback: heuristic keyword matching.
    let lower = response.to_lowercase();
    let passed =
        lower.contains("pass") || lower.contains("correct") || lower.contains("matches");
    VisualAssertion {
        screenshot_path: screenshot_path.to_string(),
        assertion: assertion.to_string(),
        passed,
        confidence: 0.5,
        details: response.to_string(),
    }
}

/// Extract the first JSON object from a string, handling optional markdown
/// code fences.
fn extract_json_block(text: &str) -> &str {
    // Look for ```json ... ``` blocks
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    // Look for ``` ... ``` blocks
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    // Look for first { ... }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return &text[start..=end];
        }
    }
    text.trim()
}

/// Create a new [`VisualTestSession`] with the given URL.
pub fn new_visual_test_session(url: &str) -> VisualTestSession {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    VisualTestSession {
        id: format!("vt-{}", now),
        url: url.to_string(),
        steps: Vec::new(),
        passed: true,
        started_at: now,
        finished_at: None,
    }
}

/// Persist a visual test session to `~/.vibecli/visual-tests/<id>.json`.
pub fn save_visual_test_session(session: &VisualTestSession) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(&home)
        .join(".vibecli")
        .join("visual-tests");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.id));
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json)?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_visual_assert_prompt() {
        let prompt = build_visual_assert_prompt("Login button is blue", "/tmp/shot.png");
        assert!(prompt.contains("Login button is blue"));
        assert!(prompt.contains("/tmp/shot.png"));
        assert!(prompt.contains("visual QA tester"));
        assert!(prompt.contains("\"passed\""));
    }

    #[test]
    fn test_parse_visual_assertion_json() {
        let response = r#"{"passed": true, "confidence": 0.95, "details": "The button is indeed blue."}"#;
        let va = parse_visual_assertion(response, "/tmp/shot.png", "Button is blue");
        assert!(va.passed);
        assert!((va.confidence - 0.95).abs() < f64::EPSILON);
        assert_eq!(va.details, "The button is indeed blue.");
        assert_eq!(va.screenshot_path, "/tmp/shot.png");
        assert_eq!(va.assertion, "Button is blue");
    }

    #[test]
    fn test_parse_visual_assertion_json_in_code_fence() {
        let response = "Here is the result:\n```json\n{\"passed\": false, \"confidence\": 0.3, \"details\": \"No button found.\"}\n```";
        let va = parse_visual_assertion(response, "/tmp/shot.png", "Button visible");
        assert!(!va.passed);
        assert!((va.confidence - 0.3).abs() < f64::EPSILON);
        assert_eq!(va.details, "No button found.");
    }

    #[test]
    fn test_parse_visual_assertion_fallback() {
        let response = "The assertion looks correct — the heading matches the expected text.";
        let va = parse_visual_assertion(response, "/tmp/x.png", "Heading matches");
        assert!(va.passed); // contains "correct"
        assert!((va.confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(va.details, response);
    }

    #[test]
    fn test_parse_visual_assertion_fallback_fail() {
        let response = "The element is missing and the layout is broken.";
        let va = parse_visual_assertion(response, "/tmp/x.png", "Element present");
        assert!(!va.passed);
    }

    #[test]
    fn test_screenshot_result_serde() {
        let sr = ScreenshotResult {
            path: "/tmp/test.png".to_string(),
            width: 1920,
            height: 1080,
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&sr).unwrap();
        let decoded: ScreenshotResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.path, sr.path);
        assert_eq!(decoded.width, sr.width);
        assert_eq!(decoded.height, sr.height);
        assert_eq!(decoded.timestamp, sr.timestamp);
    }

    #[test]
    fn test_visual_test_session_serde() {
        let session = VisualTestSession {
            id: "vt-123".to_string(),
            url: "http://localhost:3000".to_string(),
            steps: vec![VisualTestStep {
                action: "navigate".to_string(),
                screenshot: Some(ScreenshotResult {
                    path: "/tmp/s.png".to_string(),
                    width: 800,
                    height: 600,
                    timestamp: 100,
                }),
                assertion: None,
            }],
            passed: true,
            started_at: 100,
            finished_at: Some(200),
        };
        let json = serde_json::to_string(&session).unwrap();
        let decoded: VisualTestSession = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "vt-123");
        assert_eq!(decoded.steps.len(), 1);
        assert!(decoded.steps[0].screenshot.is_some());
    }

    #[test]
    fn test_new_visual_test_session() {
        let session = new_visual_test_session("http://localhost:8080");
        assert!(session.id.starts_with("vt-"));
        assert_eq!(session.url, "http://localhost:8080");
        assert!(session.steps.is_empty());
        assert!(session.passed);
        assert!(session.finished_at.is_none());
    }

    #[test]
    fn test_extract_json_block() {
        assert_eq!(
            extract_json_block("{\"a\": 1}"),
            "{\"a\": 1}"
        );
        assert_eq!(
            extract_json_block("prefix {\"a\": 1} suffix"),
            "{\"a\": 1}"
        );
        assert_eq!(
            extract_json_block("```json\n{\"a\": 1}\n```"),
            "{\"a\": 1}"
        );
    }
}
