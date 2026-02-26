#![allow(dead_code)]
//! VibeCLI Notebook Runner
//!
//! Runs `.vibe` notebook files — executable Markdown with YAML frontmatter.
//!
//! # File format
//!
//! ```markdown
//! ---
//! name: My Notebook
//! description: Demonstrates notebook runner
//! ---
//!
//! # Hello World
//!
//! Some prose text.
//!
//! ```bash
//! echo "Hello from bash"
//! ```
//!
//! ```python
//! print(1 + 1)
//! ```
//!
//! ```rust
//! fn main() { println!("Hello Rust"); }
//! ```
//!
//! Supported languages: bash, sh, python, python3, ruby, node, js, deno, rust
//!
//! # Usage
//!
//! ```
//! vibecli notebook script.vibe
//! vibecli notebook script.vibe --continue-on-error
//! ```

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NotebookMeta {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct CodeCell {
    pub language: String,
    pub source: String,
    /// 1-based line number in the source file where the cell starts.
    pub line: usize,
}

#[derive(Debug)]
pub struct CellResult {
    pub cell: CodeCell,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

pub struct Notebook {
    pub meta: NotebookMeta,
    pub cells: Vec<CodeCell>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

impl Notebook {
    /// Parse a `.vibe` file from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read notebook: {}", path.display()))?;
        Self::parse(&content)
    }

    /// Parse notebook content from a string.
    pub fn parse(content: &str) -> Result<Self> {
        let (meta, body) = extract_frontmatter(content);
        let cells = extract_code_cells(body);
        Ok(Self { meta, cells })
    }
}

fn extract_frontmatter(content: &str) -> (NotebookMeta, &str) {
    let mut name = String::new();
    let mut description = String::new();

    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let fm = &rest[..end];
            for line in fm.lines() {
                if let Some(v) = line.strip_prefix("name:") {
                    name = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("description:") {
                    description = v.trim().to_string();
                }
            }
            let body_start = 3 + end + 5; // "---\n" + fm + "\n---\n"
            return (NotebookMeta { name, description }, &content[body_start..]);
        }
    }
    (NotebookMeta { name, description }, content)
}

fn extract_code_cells(content: &str) -> Vec<CodeCell> {
    let mut cells = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(lang) = line.strip_prefix("```") {
            let lang = lang.trim().to_lowercase();
            if is_supported_language(&lang) {
                let cell_start_line = i + 1;
                i += 1;
                let mut src_lines: Vec<&str> = Vec::new();
                while i < lines.len() && lines[i].trim() != "```" {
                    src_lines.push(lines[i]);
                    i += 1;
                }
                cells.push(CodeCell {
                    language: lang,
                    source: src_lines.join("\n"),
                    line: cell_start_line + 1, // +1 for 1-based
                });
            }
        }
        i += 1;
    }
    cells
}

fn is_supported_language(lang: &str) -> bool {
    matches!(lang, "bash" | "sh" | "python" | "python3" | "ruby" | "node" | "js" | "javascript" | "deno" | "rust")
}

// ── Executor ──────────────────────────────────────────────────────────────────

/// Run a single code cell. Returns CellResult with stdout/stderr/exit_code.
pub fn run_cell(cell: &CodeCell) -> Result<CellResult> {
    use std::io::Write;

    let (cmd, args, use_stdin, use_file): (&str, Vec<&str>, bool, bool) = match cell.language.as_str() {
        "bash" | "sh" => ("sh", vec!["-c"], false, false),
        "python" | "python3" => ("python3", vec!["-c"], false, false),
        "ruby" => ("ruby", vec!["-e"], false, false),
        "node" | "js" | "javascript" => ("node", vec!["-e"], false, false),
        "deno" => ("deno", vec!["eval"], false, false),
        "rust" => {
            // Write to temp file and compile+run with rustc
            return run_rust_cell(cell);
        }
        _ => return Err(anyhow::anyhow!("Unsupported language: {}", cell.language)),
    };

    let output = if !args.is_empty() && !use_stdin && !use_file {
        Command::new(cmd)
            .args(&args)
            .arg(&cell.source)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to run {} interpreter", cmd))?
    } else {
        let mut child = Command::new(cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn {} interpreter", cmd))?;
        if let Some(stdin) = child.stdin.take() {
            let mut stdin = stdin;
            let _ = stdin.write_all(cell.source.as_bytes());
        }
        child.wait_with_output()?
    };

    let exit_code = output.status.code().unwrap_or(-1);
    Ok(CellResult {
        cell: cell.clone(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code,
        success: output.status.success(),
    })
}

fn run_rust_cell(cell: &CodeCell) -> Result<CellResult> {
    let dir = tempfile::tempdir().context("Failed to create temp dir for Rust cell")?;
    let src_path = dir.path().join("cell.rs");
    let exe_path = dir.path().join("cell");

    std::fs::write(&src_path, &cell.source)?;

    // Compile
    let compile = Command::new("rustc")
        .arg(&src_path)
        .arg("-o").arg(&exe_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run rustc (is it installed?)")?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr).into_owned();
        return Ok(CellResult {
            cell: cell.clone(),
            stdout: String::new(),
            stderr,
            exit_code: compile.status.code().unwrap_or(1),
            success: false,
        });
    }

    // Run
    let run = Command::new(&exe_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run compiled Rust cell")?;

    Ok(CellResult {
        cell: cell.clone(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
        exit_code: run.status.code().unwrap_or(-1),
        success: run.status.success(),
    })
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Run all cells in a notebook. Prints output. Returns false if any cell failed.
pub fn run_notebook(path: &Path, continue_on_error: bool) -> Result<bool> {
    let notebook = Notebook::load(path)?;
    let mut all_ok = true;

    println!("📓 Notebook: {}", if notebook.meta.name.is_empty() {
        path.display().to_string()
    } else {
        notebook.meta.name.clone()
    });
    if !notebook.meta.description.is_empty() {
        println!("   {}", notebook.meta.description);
    }
    println!("   {} cell(s)\n", notebook.cells.len());

    for (idx, cell) in notebook.cells.iter().enumerate() {
        let cell_num = idx + 1;
        let preview: String = cell.source.lines().next().unwrap_or("").chars().take(60).collect();
        println!("▶ Cell {} ({}) — {}", cell_num, cell.language, preview);
        println!("{}", "─".repeat(60));

        match run_cell(cell) {
            Ok(result) => {
                if !result.stdout.is_empty() {
                    for line in result.stdout.lines() {
                        println!("  {}", line);
                    }
                }
                if !result.stderr.is_empty() {
                    for line in result.stderr.lines() {
                        eprintln!("  \x1b[33m{}\x1b[0m", line); // yellow
                    }
                }
                if result.success {
                    println!("✅ exit {}\n", result.exit_code);
                } else {
                    println!("❌ exit {}\n", result.exit_code);
                    all_ok = false;
                    if !continue_on_error {
                        println!("Stopping at cell {} (use --continue-on-error to run all cells)", cell_num);
                        return Ok(false);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Cell {} error: {}\n", cell_num, e);
                all_ok = false;
                if !continue_on_error {
                    return Ok(false);
                }
            }
        }
    }

    Ok(all_ok)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"---
name: Test Notebook
description: Unit test notebook
---

# Section 1

Some prose.

```bash
echo "hello"
```

# Section 2

```python
print(2 + 2)
```
"#;

    #[test]
    fn test_parse_frontmatter() {
        let nb = Notebook::parse(SAMPLE).unwrap();
        assert_eq!(nb.meta.name, "Test Notebook");
        assert_eq!(nb.meta.description, "Unit test notebook");
    }

    #[test]
    fn test_parse_cells() {
        let nb = Notebook::parse(SAMPLE).unwrap();
        assert_eq!(nb.cells.len(), 2);
        assert_eq!(nb.cells[0].language, "bash");
        assert_eq!(nb.cells[0].source.trim(), "echo \"hello\"");
        assert_eq!(nb.cells[1].language, "python");
        assert_eq!(nb.cells[1].source.trim(), "print(2 + 2)");
    }

    #[test]
    fn test_run_bash_cell() {
        let cell = CodeCell {
            language: "bash".to_string(),
            source: "echo hello_notebook".to_string(),
            line: 1,
        };
        let result = run_cell(&cell).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("hello_notebook"));
    }

    #[test]
    fn test_run_bash_cell_failure() {
        let cell = CodeCell {
            language: "bash".to_string(),
            source: "exit 42".to_string(),
            line: 1,
        };
        let result = run_cell(&cell).unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 42);
    }

    #[test]
    fn test_unsupported_language_skipped() {
        let nb = Notebook::parse("```go\nfmt.Println()\n```\n").unwrap();
        assert_eq!(nb.cells.len(), 0, "Go is not a supported language, cell should be skipped");
    }

    #[test]
    fn test_no_frontmatter() {
        let nb = Notebook::parse("```bash\necho hi\n```\n").unwrap();
        assert_eq!(nb.meta.name, "");
        assert_eq!(nb.cells.len(), 1);
    }
}
