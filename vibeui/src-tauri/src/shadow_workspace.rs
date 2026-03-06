//! Shadow Workspace — a temporary directory copy of the workspace used to
//! pre-validate AI-proposed file changes before applying them.
//!
//! Flow:
//! 1. Agent proposes `write_file` → write to shadow workspace
//! 2. Run linter/type-checker on the shadow file
//! 3. Show diff annotated with lint errors
//! 4. On user Accept → copy shadow file to real workspace
//!
//! The shadow workspace is created once per VibeUI session and cleaned up on exit.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

// ── LintResult ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub line: u32,
    pub column: u32,
    pub severity: String,  // "error" | "warning" | "info"
    pub message: String,
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub file: String,
    pub diagnostics: Vec<LintDiagnostic>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl LintResult {
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == "error").count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == "warning").count()
    }
}

// ── ShadowWorkspace ───────────────────────────────────────────────────────────

pub struct ShadowWorkspace {
    /// Temporary directory for shadow files.
    pub path: PathBuf,
    /// Real workspace root.
    real_root: PathBuf,
    /// Cached lint results per file (relative path).
    pub lint_results: Arc<Mutex<HashMap<String, LintResult>>>,
}

impl ShadowWorkspace {
    /// Create a shadow workspace as a temp directory.
    pub fn new(real_root: &Path) -> Result<Self> {
        let shadow_path = std::env::temp_dir()
            .join("vibecli_shadow")
            .join(format!("{}-{:016x}", std::process::id(), rand::random::<u64>()));
        std::fs::create_dir_all(&shadow_path)?;
        Ok(Self {
            path: shadow_path,
            real_root: real_root.to_path_buf(),
            lint_results: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Write proposed content to the shadow workspace.
    /// Returns the shadow file path.
    ///
    /// The relative path is jail-checked to prevent traversal outside the
    /// shadow directory (e.g. `../../.ssh/id_rsa`).
    pub fn sync_file(&self, rel_path: &str, content: &str) -> Result<PathBuf> {
        let shadow_file = Self::safe_join(&self.path, rel_path)?;
        if let Some(parent) = shadow_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&shadow_file, content)?;
        Ok(shadow_file)
    }

    /// Run the appropriate linter for the file extension.
    /// Returns LintResult with diagnostics.
    pub fn run_lint(&self, rel_path: &str) -> Result<LintResult> {
        let shadow_file = Self::safe_join(&self.path, rel_path)?;

        let ext = Path::new(rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let (result, stdout, stderr) = match ext {
            "rs" => self.lint_rust(rel_path),
            "ts" | "tsx" | "js" | "jsx" => self.lint_typescript(rel_path),
            "py" => self.lint_python(rel_path),
            "go" => self.lint_go(rel_path),
            _ => (vec![], String::new(), String::new()),
        };

        let success = result.iter().all(|d: &LintDiagnostic| d.severity != "error");
        let lint_result = LintResult {
            file: rel_path.to_string(),
            diagnostics: result,
            success,
            stdout,
            stderr,
        };

        self.lint_results.lock().unwrap_or_else(|e| e.into_inner())
            .insert(rel_path.to_string(), lint_result.clone());

        let _ = shadow_file; // suppress unused warning
        Ok(lint_result)
    }

    fn lint_rust(&self, rel_path: &str) -> (Vec<LintDiagnostic>, String, String) {
        // Use rustfmt --check for syntax errors (fast, no full compilation needed)
        let shadow_file = self.path.join(rel_path);
        let out = Command::new("rustfmt")
            .args(["--check", "--edition", "2021"])
            .arg(&shadow_file)
            .output();

        match out {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let diagnostics = if !output.status.success() {
                    vec![LintDiagnostic {
                        line: 1,
                        column: 1,
                        severity: "warning".to_string(),
                        message: "File differs from rustfmt formatting".to_string(),
                        rule: Some("rustfmt".to_string()),
                    }]
                } else {
                    vec![]
                };
                (diagnostics, stdout, stderr)
            }
            Err(_) => (vec![], String::new(), String::new()),
        }
    }

    fn lint_typescript(&self, rel_path: &str) -> (Vec<LintDiagnostic>, String, String) {
        let shadow_file = self.path.join(rel_path);
        // Try eslint --no-eslintrc with basic rules
        let out = Command::new("npx")
            .args(["--yes", "eslint", "--format", "json", "--no-eslintrc",
                   "--rule", "no-undef: error", "--rule", "no-unused-vars: warn"])
            .arg(&shadow_file)
            .output();

        match out {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let diagnostics = parse_eslint_json(&stdout);
                (diagnostics, stdout, stderr)
            }
            Err(_) => (vec![], String::new(), String::new()),
        }
    }

    fn lint_python(&self, rel_path: &str) -> (Vec<LintDiagnostic>, String, String) {
        let shadow_file = self.path.join(rel_path);
        let out = Command::new("python3")
            .args(["-m", "py_compile"])
            .arg(&shadow_file)
            .output();

        match out {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let diagnostics = if !output.status.success() {
                    vec![LintDiagnostic {
                        line: 1,
                        column: 1,
                        severity: "error".to_string(),
                        message: stderr.lines().next().unwrap_or("Syntax error").to_string(),
                        rule: Some("py_compile".to_string()),
                    }]
                } else {
                    vec![]
                };
                (diagnostics, stdout, stderr)
            }
            Err(_) => (vec![], String::new(), String::new()),
        }
    }

    fn lint_go(&self, rel_path: &str) -> (Vec<LintDiagnostic>, String, String) {
        let shadow_file = self.path.join(rel_path);
        let out = Command::new("gofmt")
            .args(["-e", "-l"])
            .arg(&shadow_file)
            .output();

        match out {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let diagnostics = if !stderr.is_empty() {
                    vec![LintDiagnostic {
                        line: 1,
                        column: 1,
                        severity: "error".to_string(),
                        message: stderr.lines().next().unwrap_or("Go syntax error").to_string(),
                        rule: Some("gofmt".to_string()),
                    }]
                } else {
                    vec![]
                };
                (diagnostics, stdout, stderr)
            }
            Err(_) => (vec![], String::new(), String::new()),
        }
    }

    /// Accept a shadow file — copy it to the real workspace.
    ///
    /// Both the shadow path and real destination are jail-checked.
    pub fn accept_file(&self, rel_path: &str) -> Result<()> {
        let shadow_file = Self::safe_join(&self.path, rel_path)?;
        let real_file = Self::safe_join(&self.real_root, rel_path)?;
        if let Some(parent) = real_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&shadow_file, &real_file)?;
        Ok(())
    }

    /// Discard a shadow file.
    pub fn discard_file(&self, rel_path: &str) -> Result<()> {
        let shadow_file = Self::safe_join(&self.path, rel_path)?;
        if shadow_file.exists() {
            std::fs::remove_file(shadow_file)?;
        }
        self.lint_results.lock().unwrap_or_else(|e| e.into_inner()).remove(rel_path);
        Ok(())
    }

    /// Get cached lint result for a file.
    pub fn get_lint_result(&self, rel_path: &str) -> Option<LintResult> {
        self.lint_results.lock().unwrap_or_else(|e| e.into_inner()).get(rel_path).cloned()
    }

    /// Join `base` and `rel_path`, then verify the result stays inside `base`.
    ///
    /// Prevents path traversal attacks where `rel_path` contains `..` or
    /// absolute components that would escape the intended directory.
    fn safe_join(base: &Path, rel_path: &str) -> Result<PathBuf> {
        // Reject obviously absolute paths in rel_path
        if Path::new(rel_path).is_absolute() {
            anyhow::bail!(
                "Path traversal blocked: relative path '{}' is absolute",
                rel_path
            );
        }

        // Canonicalize base for comparison (must exist).
        // Use the canonical form for both joining and comparison so that
        // macOS symlinks like /var → /private/var don't cause false positives.
        let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
        let joined = canonical_base.join(rel_path);

        // Normalize manually: resolve . and .. without touching the filesystem,
        // so this works for files that don't exist yet.
        let mut resolved = PathBuf::new();
        for component in joined.components() {
            match component {
                std::path::Component::ParentDir => {
                    resolved.pop();
                }
                std::path::Component::CurDir => {}
                c => resolved.push(c),
            }
        }

        if !resolved.starts_with(&canonical_base) {
            anyhow::bail!(
                "Path traversal blocked: '{}' escapes directory '{}'",
                rel_path,
                base.display()
            );
        }

        Ok(resolved)
    }

    /// Clean up the entire shadow workspace.
    pub fn cleanup(&self) {
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

impl Drop for ShadowWorkspace {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// ── ESLint JSON parser ────────────────────────────────────────────────────────

fn parse_eslint_json(json_str: &str) -> Vec<LintDiagnostic> {
    #[derive(Deserialize)]
    struct EslintFile {
        messages: Vec<EslintMessage>,
    }
    #[derive(Deserialize)]
    struct EslintMessage {
        line: Option<u32>,
        column: Option<u32>,
        severity: Option<u32>, // 1=warn, 2=error
        message: String,
        #[serde(rename = "ruleId")]
        rule_id: Option<String>,
    }

    serde_json::from_str::<Vec<EslintFile>>(json_str)
        .unwrap_or_default()
        .into_iter()
        .flat_map(|f| f.messages.into_iter().map(|m| {
            LintDiagnostic {
                line: m.line.unwrap_or(1),
                column: m.column.unwrap_or(1),
                severity: match m.severity.unwrap_or(1) {
                    2 => "error".to_string(),
                    _ => "warning".to_string(),
                },
                message: m.message,
                rule: m.rule_id,
            }
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_workspace_sync_and_accept() {
        let tmp = std::env::temp_dir().join(format!("vibe_sw_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let shadow = ShadowWorkspace::new(&tmp).unwrap();

        shadow.sync_file("src/main.rs", "fn main() {}").unwrap();
        shadow.accept_file("src/main.rs").unwrap();

        let real_path = tmp.join("src/main.rs");
        assert!(real_path.exists());
        assert_eq!(std::fs::read_to_string(real_path).unwrap(), "fn main() {}");
    }

    #[test]
    fn sync_file_blocks_path_traversal() {
        let tmp = std::env::temp_dir().join(format!("vibe_sw_trav_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let shadow = ShadowWorkspace::new(&tmp).unwrap();

        let result = shadow.sync_file("../../etc/passwd", "pwned");
        assert!(result.is_err(), "path traversal must be blocked");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("traversal blocked"), "error should mention traversal: {}", err_msg);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn discard_removes_shadow_file() {
        let tmp = std::env::temp_dir().join(format!("vibe_sw_test2_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let shadow = ShadowWorkspace::new(&tmp).unwrap();

        shadow.sync_file("src/foo.rs", "bad code").unwrap();
        shadow.discard_file("src/foo.rs").unwrap();

        let shadow_path = shadow.path.join("src/foo.rs");
        assert!(!shadow_path.exists());
    }

    // ── safe_join tests ──────────────────────────────────────────────────

    #[test]
    fn safe_join_blocks_absolute_path() {
        let tmp = std::env::temp_dir().join(format!("vibe_sj_abs_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let result = ShadowWorkspace::safe_join(&tmp, "/etc/passwd");
        assert!(result.is_err(), "absolute path must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("traversal blocked"),
            "error should mention traversal: {}",
            msg
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn safe_join_blocks_dot_dot_traversal() {
        let tmp = std::env::temp_dir().join(format!("vibe_sj_dotdot_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let result = ShadowWorkspace::safe_join(&tmp, "../../../etc/shadow");
        assert!(result.is_err(), "../ traversal must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("traversal blocked"),
            "error should mention traversal: {}",
            msg
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn safe_join_allows_valid_deep_path() {
        let tmp = std::env::temp_dir().join(format!("vibe_sj_deep_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let result = ShadowWorkspace::safe_join(&tmp, "src/main.rs");
        assert!(result.is_ok(), "valid relative path should be allowed");
        let resolved = result.unwrap();
        assert!(
            resolved.ends_with("src/main.rs"),
            "resolved path should end with src/main.rs, got {:?}",
            resolved
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn safe_join_allows_dot_component() {
        let tmp = std::env::temp_dir().join(format!("vibe_sj_dot_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        // A path with ./  should be allowed since it stays inside the base
        let result = ShadowWorkspace::safe_join(&tmp, "./src/foo.rs");
        assert!(result.is_ok(), "path with . component should be allowed");
        let resolved = result.unwrap();
        assert!(
            resolved.ends_with("src/foo.rs"),
            "resolved path should end with src/foo.rs, got {:?}",
            resolved
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── parse_eslint_json tests ──────────────────────────────────────────

    #[test]
    fn parse_eslint_json_severity_2_is_error() {
        let json = r#"[{
            "messages": [
                {
                    "line": 10,
                    "column": 5,
                    "severity": 2,
                    "message": "Unexpected var",
                    "ruleId": "no-var"
                }
            ]
        }]"#;
        let diags = parse_eslint_json(json);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "error");
        assert_eq!(diags[0].line, 10);
        assert_eq!(diags[0].column, 5);
        assert_eq!(diags[0].message, "Unexpected var");
        assert_eq!(diags[0].rule, Some("no-var".to_string()));
    }

    #[test]
    fn parse_eslint_json_empty_messages() {
        let json = r#"[{"messages": []}]"#;
        let diags = parse_eslint_json(json);
        assert!(diags.is_empty(), "empty messages array should produce no diagnostics");
    }

    #[test]
    fn parse_eslint_json_malformed_json() {
        let diags = parse_eslint_json("not valid json at all {{{");
        assert!(
            diags.is_empty(),
            "malformed JSON should produce empty vec, got {} items",
            diags.len()
        );
    }

    // ── LintResult counter tests ─────────────────────────────────────────

    #[test]
    fn lint_result_error_and_warning_counts() {
        let result = LintResult {
            file: "test.rs".to_string(),
            diagnostics: vec![
                LintDiagnostic {
                    line: 1,
                    column: 1,
                    severity: "error".to_string(),
                    message: "err1".to_string(),
                    rule: None,
                },
                LintDiagnostic {
                    line: 2,
                    column: 1,
                    severity: "warning".to_string(),
                    message: "warn1".to_string(),
                    rule: None,
                },
                LintDiagnostic {
                    line: 3,
                    column: 1,
                    severity: "error".to_string(),
                    message: "err2".to_string(),
                    rule: None,
                },
                LintDiagnostic {
                    line: 4,
                    column: 1,
                    severity: "info".to_string(),
                    message: "info1".to_string(),
                    rule: None,
                },
            ],
            success: false,
            stdout: String::new(),
            stderr: String::new(),
        };

        assert_eq!(result.error_count(), 2, "should count 2 errors");
        assert_eq!(result.warning_count(), 1, "should count 1 warning");
    }

    #[test]
    fn lint_result_counts_with_no_diagnostics() {
        let result = LintResult {
            file: "clean.rs".to_string(),
            diagnostics: vec![],
            success: true,
            stdout: String::new(),
            stderr: String::new(),
        };

        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
    }
}
