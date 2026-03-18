//! Soul.md generator — creates project philosophy documents.
//!
//! Analyzes a project's structure, dependencies, license, README, and
//! configuration to generate a SOUL.md capturing the project's identity,
//! core beliefs, and design principles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Data types ───────────────────────────────────────────────────────────────

/// Signals discovered by scanning the project directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectSignals {
    pub name: String,
    pub description: String,
    pub license: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_docker: bool,
    pub has_readme: bool,
    pub has_contributing: bool,
    pub has_changelog: bool,
    pub is_monorepo: bool,
    pub is_open_source: bool,
    pub package_manager: Option<String>,
    pub extras: HashMap<String, String>,
}

/// A section in the generated SOUL.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulSection {
    pub heading: String,
    pub body: String,
}

/// The full generated soul document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulDocument {
    pub project_name: String,
    pub sections: Vec<SoulSection>,
}

impl SoulDocument {
    /// Render to Markdown string.
    pub fn to_markdown(&self) -> String {
        let mut md = format!("# The Soul of {}\n", self.project_name);
        for section in &self.sections {
            md.push_str(&format!("\n## {}\n\n{}\n", section.heading, section.body));
        }
        md
    }
}

// ── Project scanner ──────────────────────────────────────────────────────────

/// Scan a project directory and extract signals for soul generation.
#[allow(clippy::field_reassign_with_default)]
pub fn scan_project(workspace: &Path) -> ProjectSignals {
    let mut signals = ProjectSignals::default();

    // Project name from directory
    signals.name = workspace
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    // License detection
    for name in &["LICENSE", "LICENSE.md", "LICENSE.txt", "LICENCE"] {
        let path = workspace.join(name);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lower = content.to_lowercase();
                if lower.contains("mit license") {
                    signals.license = "MIT".to_string();
                } else if lower.contains("apache license") {
                    signals.license = "Apache-2.0".to_string();
                } else if lower.contains("gnu general public license") {
                    signals.license = "GPL".to_string();
                } else if lower.contains("bsd") {
                    signals.license = "BSD".to_string();
                } else if lower.contains("mozilla public license") {
                    signals.license = "MPL-2.0".to_string();
                } else if lower.contains("isc license") {
                    signals.license = "ISC".to_string();
                } else {
                    signals.license = "Custom".to_string();
                }
                signals.is_open_source = true;
            }
            break;
        }
    }

    // README
    signals.has_readme = workspace.join("README.md").exists() || workspace.join("readme.md").exists();

    // Extract description from README first line
    if signals.has_readme {
        let readme_path = if workspace.join("README.md").exists() {
            workspace.join("README.md")
        } else {
            workspace.join("readme.md")
        };
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            for line in content.lines().take(10) {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("![") {
                    signals.description = trimmed.to_string();
                    break;
                }
            }
        }
    }

    // Contributing
    signals.has_contributing = workspace.join("CONTRIBUTING.md").exists()
        || workspace.join("docs/contributing.md").exists();

    // Changelog
    signals.has_changelog = workspace.join("CHANGELOG.md").exists();

    // CI detection
    signals.has_ci = workspace.join(".github/workflows").exists()
        || workspace.join(".gitlab-ci.yml").exists()
        || workspace.join(".circleci").exists()
        || workspace.join("Jenkinsfile").exists();

    // Docker
    signals.has_docker = workspace.join("Dockerfile").exists()
        || workspace.join("docker-compose.yml").exists()
        || workspace.join("docker-compose.yaml").exists();

    // Language and framework detection
    detect_languages_and_frameworks(workspace, &mut signals);

    // Monorepo detection
    let cargo_ws = workspace.join("Cargo.toml");
    if cargo_ws.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_ws) {
            if content.contains("[workspace]") {
                signals.is_monorepo = true;
            }
        }
    }
    if workspace.join("lerna.json").exists()
        || workspace.join("pnpm-workspace.yaml").exists()
        || workspace.join("nx.json").exists()
    {
        signals.is_monorepo = true;
    }
    // package.json workspaces
    let pkg_json = workspace.join("package.json");
    if pkg_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if content.contains("\"workspaces\"") {
                signals.is_monorepo = true;
            }
        }
    }

    // Test detection
    signals.has_tests = workspace.join("tests").exists()
        || workspace.join("test").exists()
        || workspace.join("__tests__").exists()
        || workspace.join("spec").exists()
        || workspace.join("src/test").exists();
    // Also check for test config files
    if !signals.has_tests {
        for name in &["jest.config.js", "jest.config.ts", "vitest.config.ts", "pytest.ini", "pyproject.toml"] {
            if workspace.join(name).exists() {
                signals.has_tests = true;
                break;
            }
        }
    }

    // Package name from package.json or Cargo.toml
    if let Ok(content) = std::fs::read_to_string(&pkg_json) {
        if let Some(name) = extract_json_string_field(&content, "name") {
            signals.name = name;
        }
        if signals.description.is_empty() {
            if let Some(desc) = extract_json_string_field(&content, "description") {
                signals.description = desc;
            }
        }
    } else if let Ok(content) = std::fs::read_to_string(&cargo_ws) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") && trimmed.contains('=') {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let name = val.trim().trim_matches('"').trim_matches('\'');
                    if !name.is_empty() && !name.contains('{') {
                        signals.name = name.to_string();
                        break;
                    }
                }
            }
        }
    }

    signals
}

fn detect_languages_and_frameworks(workspace: &Path, signals: &mut ProjectSignals) {
    // Rust
    if workspace.join("Cargo.toml").exists() {
        signals.languages.push("Rust".to_string());
        if let Ok(content) = std::fs::read_to_string(workspace.join("Cargo.toml")) {
            if content.contains("actix") { signals.frameworks.push("Actix".to_string()); }
            if content.contains("axum") { signals.frameworks.push("Axum".to_string()); }
            if content.contains("rocket") { signals.frameworks.push("Rocket".to_string()); }
            if content.contains("tauri") { signals.frameworks.push("Tauri".to_string()); }
            if content.contains("tokio") { signals.frameworks.push("Tokio".to_string()); }
            if content.contains("ratatui") { signals.frameworks.push("Ratatui".to_string()); }
            if content.contains("wasm") { signals.frameworks.push("WASM".to_string()); }
        }
        signals.package_manager = Some("cargo".to_string());
    }

    // JavaScript / TypeScript
    let pkg_json = workspace.join("package.json");
    if pkg_json.exists() {
        if workspace.join("tsconfig.json").exists() {
            signals.languages.push("TypeScript".to_string());
        } else {
            signals.languages.push("JavaScript".to_string());
        }
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if content.contains("\"react\"") { signals.frameworks.push("React".to_string()); }
            if content.contains("\"next\"") { signals.frameworks.push("Next.js".to_string()); }
            if content.contains("\"vue\"") { signals.frameworks.push("Vue".to_string()); }
            if content.contains("\"svelte\"") { signals.frameworks.push("Svelte".to_string()); }
            if content.contains("\"angular\"") || content.contains("\"@angular/core\"") {
                signals.frameworks.push("Angular".to_string());
            }
            if content.contains("\"express\"") { signals.frameworks.push("Express".to_string()); }
            if content.contains("\"fastify\"") { signals.frameworks.push("Fastify".to_string()); }
            if content.contains("\"vite\"") { signals.frameworks.push("Vite".to_string()); }
        }
        // Package manager
        if workspace.join("pnpm-lock.yaml").exists() {
            signals.package_manager = Some("pnpm".to_string());
        } else if workspace.join("yarn.lock").exists() {
            signals.package_manager = Some("yarn".to_string());
        } else if workspace.join("bun.lockb").exists() || workspace.join("bun.lock").exists() {
            signals.package_manager = Some("bun".to_string());
        } else if signals.package_manager.is_none() {
            signals.package_manager = Some("npm".to_string());
        }
    }

    // Python
    if workspace.join("pyproject.toml").exists()
        || workspace.join("setup.py").exists()
        || workspace.join("requirements.txt").exists()
    {
        signals.languages.push("Python".to_string());
        if let Ok(content) = std::fs::read_to_string(workspace.join("pyproject.toml").as_path())
            .or_else(|_| std::fs::read_to_string(workspace.join("requirements.txt").as_path()))
        {
            if content.contains("django") { signals.frameworks.push("Django".to_string()); }
            if content.contains("flask") { signals.frameworks.push("Flask".to_string()); }
            if content.contains("fastapi") { signals.frameworks.push("FastAPI".to_string()); }
            if content.contains("torch") || content.contains("pytorch") { signals.frameworks.push("PyTorch".to_string()); }
            if content.contains("tensorflow") { signals.frameworks.push("TensorFlow".to_string()); }
        }
    }

    // Go
    if workspace.join("go.mod").exists() {
        signals.languages.push("Go".to_string());
        if let Ok(content) = std::fs::read_to_string(workspace.join("go.mod")) {
            if content.contains("gin-gonic") { signals.frameworks.push("Gin".to_string()); }
            if content.contains("echo") { signals.frameworks.push("Echo".to_string()); }
            if content.contains("fiber") { signals.frameworks.push("Fiber".to_string()); }
        }
    }

    // Java / Kotlin
    if workspace.join("pom.xml").exists() || workspace.join("build.gradle").exists()
        || workspace.join("build.gradle.kts").exists()
    {
        if workspace.join("build.gradle.kts").exists() || workspace.join("src/main/kotlin").exists() {
            signals.languages.push("Kotlin".to_string());
        } else {
            signals.languages.push("Java".to_string());
        }
        signals.frameworks.push("Gradle/Maven".to_string());
    }

    // C# / .NET
    if workspace.join("*.csproj").exists() || workspace.join("*.sln").exists()
        || workspace.join("global.json").exists()
    {
        signals.languages.push("C#".to_string());
        signals.frameworks.push(".NET".to_string());
    }

    // Swift
    if workspace.join("Package.swift").exists() {
        signals.languages.push("Swift".to_string());
    }

    // Elixir
    if workspace.join("mix.exs").exists() {
        signals.languages.push("Elixir".to_string());
        if let Ok(content) = std::fs::read_to_string(workspace.join("mix.exs")) {
            if content.contains(":phoenix") { signals.frameworks.push("Phoenix".to_string()); }
        }
    }
}

/// Extract a top-level string field from a JSON string (simple parser, no serde needed).
fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let pos = json.find(&pattern)?;
    let rest = &json[pos + pattern.len()..];
    // Skip whitespace and colon
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

// ── Soul generation ──────────────────────────────────────────────────────────

/// Build a prompt for an LLM to generate a SOUL.md.
pub fn build_generation_prompt(signals: &ProjectSignals) -> String {
    let mut prompt = String::with_capacity(2000);

    prompt.push_str("You are writing a SOUL.md file for a software project. ");
    prompt.push_str("A SOUL.md captures the project's philosophy, core beliefs, and design principles. ");
    prompt.push_str("It answers WHY the project exists and WHAT it believes, not HOW it works.\n\n");
    prompt.push_str("Write in a direct, confident voice. No corporate jargon. No buzzwords. ");
    prompt.push_str("Be specific to this project — generic platitudes like 'we value quality' are useless.\n\n");

    prompt.push_str("## Project Signals\n\n");
    prompt.push_str(&format!("- **Name:** {}\n", signals.name));
    if !signals.description.is_empty() {
        prompt.push_str(&format!("- **Description:** {}\n", signals.description));
    }
    if !signals.license.is_empty() {
        prompt.push_str(&format!("- **License:** {}\n", signals.license));
    }
    if !signals.languages.is_empty() {
        prompt.push_str(&format!("- **Languages:** {}\n", signals.languages.join(", ")));
    }
    if !signals.frameworks.is_empty() {
        prompt.push_str(&format!("- **Frameworks:** {}\n", signals.frameworks.join(", ")));
    }
    prompt.push_str(&format!("- **Has tests:** {}\n", signals.has_tests));
    prompt.push_str(&format!("- **Has CI:** {}\n", signals.has_ci));
    prompt.push_str(&format!("- **Has Docker:** {}\n", signals.has_docker));
    prompt.push_str(&format!("- **Monorepo:** {}\n", signals.is_monorepo));
    prompt.push_str(&format!("- **Open source:** {}\n", signals.is_open_source));
    if let Some(ref pm) = signals.package_manager {
        prompt.push_str(&format!("- **Package manager:** {}\n", pm));
    }
    for (k, v) in &signals.extras {
        prompt.push_str(&format!("- **{}:** {}\n", k, v));
    }

    prompt.push_str("\n## Required Sections\n\n");
    prompt.push_str("Generate the following sections in Markdown:\n\n");
    prompt.push_str("1. **Why This Project Exists** — The problem it solves and why existing solutions fall short\n");
    prompt.push_str("2. **Core Beliefs** — 3-6 numbered principles that guide every decision (each with a bold title and 2-3 sentence explanation)\n");
    prompt.push_str("3. **Design Principles** — Technical philosophy: architecture patterns, testing stance, dependency policy\n");
    prompt.push_str("4. **What This Project Is Not** — Explicit boundaries (bullet list)\n");
    prompt.push_str("5. **How to Know If a Change Belongs** — 3-5 questions contributors should ask before adding a feature\n\n");

    prompt.push_str("Start with `# The Soul of <project-name>` as the title.\n");
    prompt.push_str("Output ONLY the Markdown. No preamble, no commentary.\n");

    prompt
}

/// Generate a SOUL.md without an LLM — template-based fallback.
pub fn generate_template_soul(signals: &ProjectSignals) -> SoulDocument {
    let mut sections = Vec::new();

    // Section 1: Why
    let why_body = if !signals.description.is_empty() {
        format!(
            "{}.\n\nThis project exists because the problem it addresses deserves a focused, well-crafted solution — \
             not a bolted-on feature in a larger tool or a half-maintained side project.",
            signals.description.trim_end_matches('.')
        )
    } else {
        "Every project starts with a frustration. This one is no different.\n\n\
         We believe developers deserve tools that respect their time, their choices, and their intelligence."
            .to_string()
    };
    sections.push(SoulSection {
        heading: "Why This Project Exists".to_string(),
        body: why_body,
    });

    // Section 2: Core Beliefs
    let mut beliefs = Vec::new();

    if signals.is_open_source {
        beliefs.push(
            "### Open by default\n\n\
             The code is open source not as a marketing strategy, but as a commitment. \
             If you use this project, you can read every line, fork it, and make it yours."
        );
    }

    if signals.has_tests {
        beliefs.push(
            "### Tests are not optional\n\n\
             If a feature doesn't have tests, it doesn't exist. \
             The test suite is the project's immune system — it catches regressions before users do."
        );
    }

    if signals.languages.len() > 1 || signals.is_monorepo {
        beliefs.push(
            "### Shared foundations, separate surfaces\n\n\
             Common logic lives in shared libraries. Each application surface is just a frontend \
             to the same capabilities. A fix in the core improves everything."
        );
    }

    beliefs.push(
        "### Simplicity over cleverness\n\n\
         Readable code beats clever code. Standard formats beat custom ones. \
         If you can't understand how something works by reading the source, we've failed."
    );

    beliefs.push(
        "### Ship the tool, not the promise\n\n\
         Every feature in the documentation exists in code and can be built from source today. \
         If it's documented, it works. If it doesn't work, that's a bug."
    );

    if signals.has_docker || signals.has_ci {
        beliefs.push(
            "### Reproducible everywhere\n\n\
             It builds on your machine, on CI, and in a container. \
             Environment-specific surprises are bugs, not user errors."
        );
    }

    sections.push(SoulSection {
        heading: "Core Beliefs".to_string(),
        body: beliefs.join("\n\n"),
    });

    // Section 3: Design Principles
    let mut principles = Vec::new();

    if !signals.languages.is_empty() {
        principles.push(format!(
            "**Language choice:** {} {} chosen deliberately — for {}, not for trendiness.",
            signals.languages.join(" and "),
            if signals.languages.len() == 1 { "was" } else { "were" },
            match signals.languages.first().map(|s| s.as_str()) {
                Some("Rust") => "performance, safety, and correctness",
                Some("TypeScript") | Some("JavaScript") => "ecosystem reach and developer familiarity",
                Some("Python") => "readability and library ecosystem",
                Some("Go") => "simplicity, concurrency, and deployment",
                _ => "the right tradeoffs in this domain",
            }
        ));
    }

    if !signals.frameworks.is_empty() {
        principles.push(format!(
            "**Frameworks:** {} — chosen as dependencies, not as identity. \
             The project should survive any one dependency being replaced.",
            signals.frameworks.join(", ")
        ));
    }

    principles.push(
        "**Dependencies are liabilities.** Every dependency is a trust decision. \
         We prefer small, well-maintained libraries over sprawling frameworks."
            .to_string(),
    );

    if signals.has_tests {
        principles.push(
            "**Test at the boundaries.** Unit tests for logic, integration tests for I/O. \
             Don't mock what you own — test the real thing."
                .to_string(),
        );
    }

    sections.push(SoulSection {
        heading: "Design Principles".to_string(),
        body: principles.join("\n\n"),
    });

    // Section 4: What It Is Not
    let mut nots = Vec::new();
    nots.push("Not a framework — it's a tool that does one job well.".to_string());
    nots.push("Not a platform — there's no account to create, no server to depend on.".to_string());
    if signals.is_open_source {
        nots.push("Not a business masquerading as open source — the full tool is the free tool.".to_string());
    }
    nots.push("Not finished — but what's shipped today works today.".to_string());

    sections.push(SoulSection {
        heading: "What This Project Is Not".to_string(),
        body: nots.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n"),
    });

    // Section 5: Decision framework
    let questions = [
        "Does it solve a real problem that users actually have?",
        "Can you explain it in one sentence without jargon?",
        "Is it tested? If you can't write tests for it, is it well-defined enough to ship?",
        "Does it earn its complexity? A feature that helps one workflow but complicates ten others is a net negative.",
        "Would you be comfortable maintaining this in two years?",
    ];

    sections.push(SoulSection {
        heading: "How to Know If a Change Belongs".to_string(),
        body: format!(
            "Before adding a feature, ask:\n\n{}",
            questions
                .iter()
                .enumerate()
                .map(|(i, q)| format!("{}. {q}", i + 1))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    });

    SoulDocument {
        project_name: signals.name.clone(),
        sections,
    }
}

/// Write a SOUL.md to the given workspace directory.
pub fn write_soul(workspace: &Path, content: &str) -> std::io::Result<PathBuf> {
    let path = workspace.join("SOUL.md");
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Check if a SOUL.md already exists in the workspace.
pub fn soul_exists(workspace: &Path) -> bool {
    workspace.join("SOUL.md").exists()
        || workspace.join("soul.md").exists()
}

/// Read existing SOUL.md content.
pub fn read_soul(workspace: &Path) -> Option<String> {
    let path = workspace.join("SOUL.md");
    if path.exists() {
        return std::fs::read_to_string(&path).ok();
    }
    let path = workspace.join("soul.md");
    if path.exists() {
        return std::fs::read_to_string(&path).ok();
    }
    None
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_project(name: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("soul_test_{}_{}", name, id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn scan_empty_project() {
        let dir = temp_project("empty");
        let signals = scan_project(&dir);
        assert_eq!(signals.languages.len(), 0);
        assert!(!signals.has_tests);
        assert!(!signals.has_ci);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_mit_license() {
        let dir = temp_project("mit");
        fs::write(dir.join("LICENSE"), "MIT License\n\nCopyright (c) 2026").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.license, "MIT");
        assert!(signals.is_open_source);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_apache_license() {
        let dir = temp_project("apache");
        fs::write(dir.join("LICENSE"), "Apache License\nVersion 2.0").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.license, "Apache-2.0");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_rust() {
        let dir = temp_project("rust");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"myapp\"\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"Rust".to_string()));
        assert_eq!(signals.package_manager, Some("cargo".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_typescript_react() {
        let dir = temp_project("ts_react");
        fs::write(dir.join("package.json"), r#"{"name":"myapp","dependencies":{"react":"18"}}"#).unwrap();
        fs::write(dir.join("tsconfig.json"), "{}").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"TypeScript".to_string()));
        assert!(signals.frameworks.contains(&"React".to_string()));
        assert_eq!(signals.name, "myapp");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_python_fastapi() {
        let dir = temp_project("py_fastapi");
        fs::write(dir.join("pyproject.toml"), "[project]\nname = \"api\"\ndependencies = [\"fastapi\"]\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"Python".to_string()));
        assert!(signals.frameworks.contains(&"FastAPI".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_go() {
        let dir = temp_project("go");
        fs::write(dir.join("go.mod"), "module example.com/myapp\n\ngo 1.21\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"Go".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_monorepo_cargo_workspace() {
        let dir = temp_project("mono_cargo");
        fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"a\", \"b\"]\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.is_monorepo);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_monorepo_npm_workspaces() {
        let dir = temp_project("mono_npm");
        fs::write(dir.join("package.json"), r#"{"name":"mono","workspaces":["packages/*"]}"#).unwrap();
        let signals = scan_project(&dir);
        assert!(signals.is_monorepo);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_ci_github() {
        let dir = temp_project("ci");
        fs::create_dir_all(dir.join(".github/workflows")).unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_ci);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_docker() {
        let dir = temp_project("docker");
        fs::write(dir.join("Dockerfile"), "FROM rust:1.88").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_docker);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_tests_directory() {
        let dir = temp_project("tests_dir");
        fs::create_dir_all(dir.join("tests")).unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_tests);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_jest_config() {
        let dir = temp_project("jest");
        fs::write(dir.join("jest.config.ts"), "export default {}").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_tests);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_readme_description() {
        let dir = temp_project("readme");
        fs::write(dir.join("README.md"), "# MyApp\n\nA blazing fast widget factory.\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_readme);
        assert_eq!(signals.description, "A blazing fast widget factory.");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_contributing() {
        let dir = temp_project("contrib");
        fs::write(dir.join("CONTRIBUTING.md"), "# Contributing\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.has_contributing);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_pnpm() {
        let dir = temp_project("pnpm");
        fs::write(dir.join("package.json"), r#"{"name":"app"}"#).unwrap();
        fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.package_manager, Some("pnpm".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_bun() {
        let dir = temp_project("bun");
        fs::write(dir.join("package.json"), r#"{"name":"app"}"#).unwrap();
        fs::write(dir.join("bun.lockb"), "").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.package_manager, Some("bun".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_elixir_phoenix() {
        let dir = temp_project("elixir");
        fs::write(dir.join("mix.exs"), "defp deps do [{:phoenix, \"~> 1.7\"}] end").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"Elixir".to_string()));
        assert!(signals.frameworks.contains(&"Phoenix".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_name_from_cargo_package() {
        let dir = temp_project("cargo_name");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"cool-tool\"\nversion = \"0.1.0\"\n").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.name, "cool-tool");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn template_soul_has_all_sections() {
        let signals = ProjectSignals {
            name: "testproject".to_string(),
            has_tests: true,
            is_open_source: true,
            has_ci: true,
            languages: vec!["Rust".to_string()],
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        assert_eq!(doc.project_name, "testproject");
        assert_eq!(doc.sections.len(), 5);
        assert_eq!(doc.sections[0].heading, "Why This Project Exists");
        assert_eq!(doc.sections[1].heading, "Core Beliefs");
        assert_eq!(doc.sections[2].heading, "Design Principles");
        assert_eq!(doc.sections[3].heading, "What This Project Is Not");
        assert_eq!(doc.sections[4].heading, "How to Know If a Change Belongs");
    }

    #[test]
    fn template_soul_markdown_rendering() {
        let signals = ProjectSignals {
            name: "mylib".to_string(),
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let md = doc.to_markdown();
        assert!(md.starts_with("# The Soul of mylib\n"));
        assert!(md.contains("## Why This Project Exists"));
        assert!(md.contains("## Core Beliefs"));
        assert!(md.contains("## Design Principles"));
    }

    #[test]
    fn template_soul_open_source_belief() {
        let signals = ProjectSignals {
            name: "oss".to_string(),
            is_open_source: true,
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let beliefs = &doc.sections[1].body;
        assert!(beliefs.contains("Open by default"));
    }

    #[test]
    fn template_soul_test_belief() {
        let signals = ProjectSignals {
            name: "tested".to_string(),
            has_tests: true,
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let beliefs = &doc.sections[1].body;
        assert!(beliefs.contains("Tests are not optional"));
    }

    #[test]
    fn template_soul_monorepo_belief() {
        let signals = ProjectSignals {
            name: "mono".to_string(),
            is_monorepo: true,
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let beliefs = &doc.sections[1].body;
        assert!(beliefs.contains("Shared foundations"));
    }

    #[test]
    fn template_soul_no_open_source_belief() {
        let signals = ProjectSignals {
            name: "closed".to_string(),
            is_open_source: false,
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let beliefs = &doc.sections[1].body;
        assert!(!beliefs.contains("Open by default"));
    }

    #[test]
    fn build_prompt_includes_signals() {
        let signals = ProjectSignals {
            name: "myapp".to_string(),
            description: "A fast CLI tool".to_string(),
            license: "MIT".to_string(),
            languages: vec!["Rust".to_string()],
            frameworks: vec!["Tokio".to_string()],
            has_tests: true,
            ..Default::default()
        };
        let prompt = build_generation_prompt(&signals);
        assert!(prompt.contains("myapp"));
        assert!(prompt.contains("A fast CLI tool"));
        assert!(prompt.contains("MIT"));
        assert!(prompt.contains("Rust"));
        assert!(prompt.contains("Tokio"));
        assert!(prompt.contains("SOUL.md"));
    }

    #[test]
    fn build_prompt_required_sections() {
        let signals = ProjectSignals::default();
        let prompt = build_generation_prompt(&signals);
        assert!(prompt.contains("Why This Project Exists"));
        assert!(prompt.contains("Core Beliefs"));
        assert!(prompt.contains("Design Principles"));
        assert!(prompt.contains("What This Project Is Not"));
        assert!(prompt.contains("How to Know If a Change Belongs"));
    }

    #[test]
    fn write_and_read_soul() {
        let dir = temp_project("write_read");
        let content = "# The Soul of Test\n\nTest content.\n";
        let path = write_soul(&dir, content).unwrap();
        assert_eq!(path, dir.join("SOUL.md"));
        let read_back = read_soul(&dir).unwrap();
        assert_eq!(read_back, content);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn soul_exists_check() {
        let dir = temp_project("exists");
        assert!(!soul_exists(&dir));
        fs::write(dir.join("SOUL.md"), "test").unwrap();
        assert!(soul_exists(&dir));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn soul_exists_lowercase() {
        let dir = temp_project("exists_lower");
        fs::write(dir.join("soul.md"), "test").unwrap();
        assert!(soul_exists(&dir));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extract_json_field_works() {
        let json = r#"{"name": "foo", "version": "1.0"}"#;
        assert_eq!(extract_json_string_field(json, "name"), Some("foo".to_string()));
        assert_eq!(extract_json_string_field(json, "version"), Some("1.0".to_string()));
        assert_eq!(extract_json_string_field(json, "missing"), None);
    }

    #[test]
    fn soul_document_serde_roundtrip() {
        let doc = SoulDocument {
            project_name: "test".to_string(),
            sections: vec![SoulSection {
                heading: "Why".to_string(),
                body: "Because.".to_string(),
            }],
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: SoulDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_name, "test");
        assert_eq!(back.sections.len(), 1);
    }

    #[test]
    fn full_scan_and_generate_integration() {
        let dir = temp_project("integration");
        fs::write(dir.join("LICENSE"), "MIT License").unwrap();
        fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"a\"]\n[package]\nname = \"integtest\"\n").unwrap();
        fs::write(dir.join("README.md"), "# IntegTest\n\nAn integration test project.\n").unwrap();
        fs::create_dir_all(dir.join(".github/workflows")).unwrap();
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::write(dir.join("Dockerfile"), "FROM rust").unwrap();

        let signals = scan_project(&dir);
        assert_eq!(signals.name, "integtest");
        assert_eq!(signals.license, "MIT");
        assert!(signals.is_open_source);
        assert!(signals.is_monorepo);
        assert!(signals.has_ci);
        assert!(signals.has_docker);
        assert!(signals.has_tests);
        assert!(signals.has_readme);
        assert!(signals.languages.contains(&"Rust".to_string()));

        let doc = generate_template_soul(&signals);
        let md = doc.to_markdown();
        assert!(md.contains("The Soul of integtest"));
        assert!(md.contains("Open by default"));
        assert!(md.contains("Tests are not optional"));
        assert!(md.contains("Shared foundations"));
        assert!(md.contains("Reproducible everywhere"));

        let path = write_soul(&dir, &md).unwrap();
        assert!(path.exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_gpl_license() {
        let dir = temp_project("gpl");
        fs::write(dir.join("LICENSE"), "GNU General Public License\nVersion 3").unwrap();
        let signals = scan_project(&dir);
        assert_eq!(signals.license, "GPL");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_vue() {
        let dir = temp_project("vue");
        fs::write(dir.join("package.json"), r#"{"name":"app","dependencies":{"vue":"3"}}"#).unwrap();
        let signals = scan_project(&dir);
        assert!(signals.frameworks.contains(&"Vue".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_nextjs() {
        let dir = temp_project("next");
        fs::write(dir.join("package.json"), r#"{"name":"web","dependencies":{"next":"14","react":"18"}}"#).unwrap();
        let signals = scan_project(&dir);
        assert!(signals.frameworks.contains(&"Next.js".to_string()));
        assert!(signals.frameworks.contains(&"React".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_axum() {
        let dir = temp_project("axum");
        fs::write(dir.join("Cargo.toml"), "[dependencies]\naxum = \"0.7\"\ntokio = \"1\"\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.frameworks.contains(&"Axum".to_string()));
        assert!(signals.frameworks.contains(&"Tokio".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_django() {
        let dir = temp_project("django");
        fs::write(dir.join("requirements.txt"), "django>=4.2\ncelery\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.languages.contains(&"Python".to_string()));
        assert!(signals.frameworks.contains(&"Django".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_detects_gin() {
        let dir = temp_project("gin");
        fs::write(dir.join("go.mod"), "module myapp\nrequire github.com/gin-gonic/gin v1.9\n").unwrap();
        let signals = scan_project(&dir);
        assert!(signals.frameworks.contains(&"Gin".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn template_soul_language_rust_description() {
        let signals = ProjectSignals {
            name: "rs".to_string(),
            languages: vec!["Rust".to_string()],
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        assert!(doc.sections[2].body.contains("performance, safety, and correctness"));
    }

    #[test]
    fn template_soul_language_python_description() {
        let signals = ProjectSignals {
            name: "py".to_string(),
            languages: vec!["Python".to_string()],
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        assert!(doc.sections[2].body.contains("readability and library ecosystem"));
    }

    #[test]
    fn template_soul_language_go_description() {
        let signals = ProjectSignals {
            name: "go".to_string(),
            languages: vec!["Go".to_string()],
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        assert!(doc.sections[2].body.contains("simplicity, concurrency, and deployment"));
    }

    #[test]
    fn template_soul_not_items() {
        let signals = ProjectSignals {
            name: "nots".to_string(),
            is_open_source: true,
            ..Default::default()
        };
        let doc = generate_template_soul(&signals);
        let nots = &doc.sections[3].body;
        assert!(nots.contains("Not a framework"));
        assert!(nots.contains("Not a platform"));
        assert!(nots.contains("masquerading as open source"));
    }

    #[test]
    fn template_soul_decision_questions() {
        let signals = ProjectSignals::default();
        let doc = generate_template_soul(&signals);
        let questions = &doc.sections[4].body;
        assert!(questions.contains("real problem"));
        assert!(questions.contains("one sentence"));
        assert!(questions.contains("tested"));
        assert!(questions.contains("complexity"));
        assert!(questions.contains("two years"));
    }
}
