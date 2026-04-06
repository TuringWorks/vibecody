//! A8: Smart Extension Auto-Detection for VibeCLI.
//!
//! Inspects the current workspace directory and returns a list of recommended
//! skill files / REPL commands to load automatically — closing the gap with
//! Goose's built-in extension discovery.
//!
//! Detection is purely file-system based (no API calls) and runs in < 1 ms
//! for typical project roots. Results are printed as hints at session start
//! when `--smart-detect` is passed or `auto_detect = true` in config.toml.

use std::path::Path;

// ── Detection result ──────────────────────────────────────────────────────────

/// A recommended skill/extension identified by workspace inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionHint {
    /// Human-readable label, e.g. "Rust".
    pub label: String,
    /// The skill file name (without path) or REPL command to load.
    pub skill: String,
    /// Why this hint was generated.
    pub reason: String,
}

// ── Probe helpers ─────────────────────────────────────────────────────────────

fn file_exists(root: &Path, name: &str) -> bool {
    root.join(name).exists()
}

fn any_file_with_ext(root: &Path, ext: &str) -> bool {
    std::fs::read_dir(root)
        .ok()
        .and_then(|mut d| {
            d.find(|e| {
                e.as_ref()
                    .ok()
                    .and_then(|e| e.path().extension().and_then(|x| x.to_str()).map(|x| x == ext))
                    .unwrap_or(false)
            })
        })
        .is_some()
}

// ── Main detection logic ──────────────────────────────────────────────────────

/// Inspect `workspace_root` and return recommended extension hints.
pub fn detect_extensions(workspace_root: &Path) -> Vec<ExtensionHint> {
    let mut hints: Vec<ExtensionHint> = vec![];

    // ── Languages ─────────────────────────────────────────────────────────────

    if file_exists(workspace_root, "Cargo.toml") {
        hints.push(ExtensionHint {
            label: "Rust".into(),
            skill: "rust-dev".into(),
            reason: "Cargo.toml found".into(),
        });
    }

    if file_exists(workspace_root, "package.json") {
        let is_ts = file_exists(workspace_root, "tsconfig.json")
            || any_file_with_ext(workspace_root, "ts")
            || any_file_with_ext(workspace_root, "tsx");
        if is_ts {
            hints.push(ExtensionHint {
                label: "TypeScript".into(),
                skill: "typescript-dev".into(),
                reason: "package.json + TypeScript files detected".into(),
            });
        } else {
            hints.push(ExtensionHint {
                label: "Node.js / JavaScript".into(),
                skill: "nodejs-dev".into(),
                reason: "package.json found".into(),
            });
        }
    }

    if file_exists(workspace_root, "pyproject.toml")
        || file_exists(workspace_root, "setup.py")
        || file_exists(workspace_root, "requirements.txt")
        || any_file_with_ext(workspace_root, "py")
    {
        hints.push(ExtensionHint {
            label: "Python".into(),
            skill: "python-dev".into(),
            reason: "Python project files detected".into(),
        });
    }

    if file_exists(workspace_root, "go.mod") {
        hints.push(ExtensionHint {
            label: "Go".into(),
            skill: "go-dev".into(),
            reason: "go.mod found".into(),
        });
    }

    if file_exists(workspace_root, "pom.xml")
        || file_exists(workspace_root, "build.gradle")
        || file_exists(workspace_root, "build.gradle.kts")
    {
        hints.push(ExtensionHint {
            label: "Java / JVM".into(),
            skill: "java-dev".into(),
            reason: "Maven/Gradle build file found".into(),
        });
    }

    // ── Frameworks ────────────────────────────────────────────────────────────

    if file_exists(workspace_root, "tauri.conf.json")
        || workspace_root.join("src-tauri").exists()
    {
        hints.push(ExtensionHint {
            label: "Tauri".into(),
            skill: "tauri-dev".into(),
            reason: "Tauri project structure detected".into(),
        });
    }

    if file_exists(workspace_root, "next.config.js")
        || file_exists(workspace_root, "next.config.ts")
        || file_exists(workspace_root, "next.config.mjs")
    {
        hints.push(ExtensionHint {
            label: "Next.js".into(),
            skill: "nextjs-dev".into(),
            reason: "next.config.* found".into(),
        });
    }

    if file_exists(workspace_root, "vite.config.ts")
        || file_exists(workspace_root, "vite.config.js")
    {
        hints.push(ExtensionHint {
            label: "Vite".into(),
            skill: "vite-dev".into(),
            reason: "vite.config.* found".into(),
        });
    }

    if file_exists(workspace_root, "Dockerfile")
        || file_exists(workspace_root, "docker-compose.yml")
        || file_exists(workspace_root, "docker-compose.yaml")
    {
        hints.push(ExtensionHint {
            label: "Docker".into(),
            skill: "docker-ops".into(),
            reason: "Dockerfile / docker-compose found".into(),
        });
    }

    if workspace_root.join(".github").join("workflows").exists() {
        hints.push(ExtensionHint {
            label: "GitHub Actions".into(),
            skill: "github-actions".into(),
            reason: ".github/workflows/ directory found".into(),
        });
    }

    // ── Workspace-level hints ─────────────────────────────────────────────────

    if file_exists(workspace_root, ".vibehints") {
        hints.push(ExtensionHint {
            label: "VibeCLI Workspace Hints".into(),
            skill: "load-vibehints".into(),
            reason: ".vibehints file found at workspace root".into(),
        });
    }

    hints
}

/// Print workspace extension hints to stdout (called at session start).
pub fn print_extension_hints(workspace_root: &Path) {
    let hints = detect_extensions(workspace_root);
    if hints.is_empty() { return; }

    println!("\x1b[1;36m🔍 Workspace detected:\x1b[0m");
    for h in &hints {
        println!("   \x1b[33m{}\x1b[0m — {} ({})", h.label, h.skill, h.reason);
    }
    println!("   Tip: run \x1b[2m/skills load <skill>\x1b[0m to activate a skill.");
    println!();
}

// ── REPL handler ─────────────────────────────────────────────────────────────

/// Handle `/workspace-detect` REPL command.
pub fn handle_workspace_detect_command() -> String {
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let hints = detect_extensions(&root);
    if hints.is_empty() {
        return "No specific extensions detected for this workspace.\n".to_string();
    }
    let mut out = format!("🔍 Workspace extensions detected ({}):\n", hints.len());
    for h in &hints {
        out.push_str(&format!("  {} — skill: {}  ({})\n", h.label, h.skill, h.reason));
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> TempDir { tempfile::tempdir().unwrap() }

    #[test]
    fn test_detects_rust() {
        let d = tmp();
        fs::write(d.path().join("Cargo.toml"), "[package]").unwrap();
        let hints = detect_extensions(d.path());
        assert!(hints.iter().any(|h| h.label == "Rust"));
    }

    #[test]
    fn test_detects_typescript() {
        let d = tmp();
        fs::write(d.path().join("package.json"), "{}").unwrap();
        fs::write(d.path().join("tsconfig.json"), "{}").unwrap();
        let hints = detect_extensions(d.path());
        assert!(hints.iter().any(|h| h.label == "TypeScript"));
    }

    #[test]
    fn test_detects_docker() {
        let d = tmp();
        fs::write(d.path().join("Dockerfile"), "FROM ubuntu").unwrap();
        let hints = detect_extensions(d.path());
        assert!(hints.iter().any(|h| h.label == "Docker"));
    }

    #[test]
    fn test_empty_dir_no_hints() {
        let d = tmp();
        let hints = detect_extensions(d.path());
        // Should not panic; vibehints hint is absent
        assert!(hints.iter().all(|h| h.skill != "load-vibehints"));
    }

    #[test]
    fn test_detects_python() {
        let d = tmp();
        fs::write(d.path().join("requirements.txt"), "requests").unwrap();
        let hints = detect_extensions(d.path());
        assert!(hints.iter().any(|h| h.label == "Python"));
    }
}
