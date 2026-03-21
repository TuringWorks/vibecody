//! Smart project initialization and auto-context engine.
//!
//! Provides zero-to-productive onboarding for brownfield projects by:
//! 1. Auto-detecting project type, build system, test framework, and key files
//! 2. Reading and caching key file summaries (README, config, entry points)
//! 3. Extracting relevant files based on task description analysis
//! 4. Generating a project summary for injection into the agent system prompt
//!
//! This module closes the gap with competitors (Cursor, Windsurf, Claude Code)
//! that auto-index projects on open.

use std::path::Path;

use serde::{Deserialize, Serialize};

// ── Project Profile ────────────────────────────────────────────────────────

/// Comprehensive project profile built from filesystem analysis.
/// Cached to `.vibecli/project-profile.json` to avoid re-scanning on every session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectProfile {
    /// Project name (from package.json, Cargo.toml, or directory name).
    pub name: String,
    /// One-line description extracted from README or package manifest.
    pub description: String,
    /// Detected primary languages (e.g. ["Rust", "TypeScript"]).
    pub languages: Vec<String>,
    /// Detected frameworks (e.g. ["React", "Tauri", "Tokio"]).
    pub frameworks: Vec<String>,
    /// Project architecture type.
    pub architecture: ProjectArchitecture,
    /// Build commands detected for each language/tool.
    pub build_commands: Vec<BuildCommand>,
    /// Test commands detected for each test framework.
    pub test_commands: Vec<TestCommand>,
    /// Lint/format commands.
    pub lint_commands: Vec<LintCommand>,
    /// Key files that provide critical project context.
    pub key_files: Vec<KeyFile>,
    /// Entry points for the application.
    pub entry_points: Vec<String>,
    /// Detected package managers.
    pub package_managers: Vec<String>,
    /// Environment variables referenced in the project.
    pub env_vars: Vec<String>,
    /// Quick-reference summary for injection into LLM context.
    pub summary: String,
    /// Timestamp of last scan (epoch seconds).
    pub scanned_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum ProjectArchitecture {
    #[default]
    SinglePackage,
    Monorepo,
    Library,
    FullStackApp,
    MicroserviceCluster,
    CLITool,
}

impl std::fmt::Display for ProjectArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SinglePackage => write!(f, "single package"),
            Self::Monorepo => write!(f, "monorepo"),
            Self::Library => write!(f, "library"),
            Self::FullStackApp => write!(f, "full-stack application"),
            Self::MicroserviceCluster => write!(f, "microservice cluster"),
            Self::CLITool => write!(f, "CLI tool"),
        }
    }
}

/// A detected build command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCommand {
    pub label: String,
    pub command: String,
    pub working_dir: Option<String>,
}

/// A detected test command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    pub label: String,
    pub command: String,
    pub framework: String,
}

/// A detected lint/format command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintCommand {
    pub label: String,
    pub command: String,
}

/// A key file with its first N lines cached for LLM context injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    pub path: String,
    pub role: KeyFileRole,
    /// First ~50 lines or a summary of the file.
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyFileRole {
    Readme,
    Config,
    EntryPoint,
    Schema,
    Migration,
    TestConfig,
    CIConfig,
    Dockerfile,
    EnvExample,
    Contributing,
    APISpec,
}

impl std::fmt::Display for KeyFileRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Readme => write!(f, "readme"),
            Self::Config => write!(f, "config"),
            Self::EntryPoint => write!(f, "entry point"),
            Self::Schema => write!(f, "schema"),
            Self::Migration => write!(f, "migration"),
            Self::TestConfig => write!(f, "test config"),
            Self::CIConfig => write!(f, "CI config"),
            Self::Dockerfile => write!(f, "Dockerfile"),
            Self::EnvExample => write!(f, "env example"),
            Self::Contributing => write!(f, "contributing guide"),
            Self::APISpec => write!(f, "API spec"),
        }
    }
}

// ── Scanner ───────────────────────────────────────────────────────────────

/// Scan a workspace and build a comprehensive project profile.
pub fn scan_workspace(workspace: &Path) -> ProjectProfile {
    let mut profile = ProjectProfile::default();

    // Basic identity
    profile.name = workspace
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    profile.scanned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Detect languages, frameworks, build/test/lint commands
    detect_rust(workspace, &mut profile);
    detect_node(workspace, &mut profile);
    detect_python(workspace, &mut profile);
    detect_go(workspace, &mut profile);
    detect_java(workspace, &mut profile);
    detect_dotnet(workspace, &mut profile);
    detect_ruby(workspace, &mut profile);
    detect_php(workspace, &mut profile);

    // Architecture detection
    detect_architecture(workspace, &mut profile);

    // Key files
    collect_key_files(workspace, &mut profile);

    // Entry points
    detect_entry_points(workspace, &mut profile);

    // Env vars
    detect_env_vars(workspace, &mut profile);

    // Generate summary
    profile.summary = generate_summary(&profile);
    profile.description = extract_description(workspace).unwrap_or_default();

    profile
}

// ── Language/Framework Detectors ──────────────────────────────────────────

fn detect_rust(workspace: &Path, profile: &mut ProjectProfile) {
    let cargo_toml = workspace.join("Cargo.toml");
    if !cargo_toml.exists() {
        return;
    }
    profile.languages.push("Rust".to_string());

    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        if content.contains("[workspace]") {
            profile.build_commands.push(BuildCommand {
                label: "Cargo workspace build".to_string(),
                command: "cargo build --workspace".to_string(),
                working_dir: None,
            });
            profile.test_commands.push(TestCommand {
                label: "Cargo workspace test".to_string(),
                command: "cargo test --workspace".to_string(),
                framework: "cargo test".to_string(),
            });
        } else {
            profile.build_commands.push(BuildCommand {
                label: "Cargo build".to_string(),
                command: "cargo build".to_string(),
                working_dir: None,
            });
            profile.test_commands.push(TestCommand {
                label: "Cargo test".to_string(),
                command: "cargo test".to_string(),
                framework: "cargo test".to_string(),
            });
        }

        // Detect frameworks
        if content.contains("tokio") {
            profile.frameworks.push("Tokio".to_string());
        }
        if content.contains("actix") {
            profile.frameworks.push("Actix".to_string());
        }
        if content.contains("axum") {
            profile.frameworks.push("Axum".to_string());
        }
        if content.contains("tauri") {
            profile.frameworks.push("Tauri".to_string());
        }
        if content.contains("ratatui") || content.contains("tui") {
            profile.frameworks.push("Ratatui/TUI".to_string());
        }
        if content.contains("warp") {
            profile.frameworks.push("Warp".to_string());
        }
        if content.contains("rocket") {
            profile.frameworks.push("Rocket".to_string());
        }
    }

    profile.lint_commands.push(LintCommand {
        label: "Clippy".to_string(),
        command: "cargo clippy --workspace -- -D warnings".to_string(),
    });
    profile.lint_commands.push(LintCommand {
        label: "Rustfmt check".to_string(),
        command: "cargo fmt --all -- --check".to_string(),
    });
    profile.package_managers.push("cargo".to_string());
}

fn detect_node(workspace: &Path, profile: &mut ProjectProfile) {
    let pkg_json = workspace.join("package.json");
    if !pkg_json.exists() {
        return;
    }

    if let Ok(content) = std::fs::read_to_string(&pkg_json) {
        // Language detection
        if workspace.join("tsconfig.json").exists() || content.contains("typescript") {
            profile.languages.push("TypeScript".to_string());
        } else {
            profile.languages.push("JavaScript".to_string());
        }

        // Framework detection
        if content.contains("\"react\"") {
            profile.frameworks.push("React".to_string());
        }
        if content.contains("\"next\"") || content.contains("\"next\":") {
            profile.frameworks.push("Next.js".to_string());
        }
        if content.contains("\"vue\"") {
            profile.frameworks.push("Vue".to_string());
        }
        if content.contains("\"nuxt\"") {
            profile.frameworks.push("Nuxt".to_string());
        }
        if content.contains("\"svelte\"") || content.contains("\"@sveltejs") {
            profile.frameworks.push("Svelte".to_string());
        }
        if content.contains("\"express\"") {
            profile.frameworks.push("Express".to_string());
        }
        if content.contains("\"fastify\"") {
            profile.frameworks.push("Fastify".to_string());
        }
        if content.contains("\"angular\"") || content.contains("\"@angular") {
            profile.frameworks.push("Angular".to_string());
        }
        if content.contains("\"remix\"") || content.contains("\"@remix-run") {
            profile.frameworks.push("Remix".to_string());
        }
        if content.contains("\"astro\"") {
            profile.frameworks.push("Astro".to_string());
        }
        if content.contains("\"electron\"") {
            profile.frameworks.push("Electron".to_string());
        }
        if content.contains("\"vite\"") {
            profile.frameworks.push("Vite".to_string());
        }

        // Build commands from scripts
        if content.contains("\"build\"") {
            profile.build_commands.push(BuildCommand {
                label: "npm build".to_string(),
                command: detect_npm_runner(workspace).to_string() + " run build",
                working_dir: None,
            });
        }
        if content.contains("\"dev\"") {
            profile.build_commands.push(BuildCommand {
                label: "Dev server".to_string(),
                command: detect_npm_runner(workspace).to_string() + " run dev",
                working_dir: None,
            });
        }

        // Test commands
        if content.contains("\"jest\"") || content.contains("\"@jest") {
            profile.test_commands.push(TestCommand {
                label: "Jest".to_string(),
                command: detect_npm_runner(workspace).to_string() + " test",
                framework: "Jest".to_string(),
            });
        }
        if content.contains("\"vitest\"") {
            profile.test_commands.push(TestCommand {
                label: "Vitest".to_string(),
                command: detect_npm_runner(workspace).to_string() + " run test",
                framework: "Vitest".to_string(),
            });
        }
        if content.contains("\"playwright\"") || content.contains("\"@playwright") {
            profile.test_commands.push(TestCommand {
                label: "Playwright".to_string(),
                command: "npx playwright test".to_string(),
                framework: "Playwright".to_string(),
            });
        }
        if content.contains("\"cypress\"") {
            profile.test_commands.push(TestCommand {
                label: "Cypress".to_string(),
                command: "npx cypress run".to_string(),
                framework: "Cypress".to_string(),
            });
        }
        if content.contains("\"mocha\"") {
            profile.test_commands.push(TestCommand {
                label: "Mocha".to_string(),
                command: "npx mocha".to_string(),
                framework: "Mocha".to_string(),
            });
        }
        if content.contains("\"test\"") && profile.test_commands.is_empty() {
            profile.test_commands.push(TestCommand {
                label: "npm test".to_string(),
                command: detect_npm_runner(workspace).to_string() + " test",
                framework: "npm scripts".to_string(),
            });
        }

        // Lint commands
        if content.contains("\"eslint\"") || content.contains("\"@eslint") {
            profile.lint_commands.push(LintCommand {
                label: "ESLint".to_string(),
                command: "npx eslint .".to_string(),
            });
        }
        if content.contains("\"prettier\"") {
            profile.lint_commands.push(LintCommand {
                label: "Prettier check".to_string(),
                command: "npx prettier --check .".to_string(),
            });
        }
        if content.contains("\"biome\"") || content.contains("\"@biomejs") {
            profile.lint_commands.push(LintCommand {
                label: "Biome".to_string(),
                command: "npx biome check .".to_string(),
            });
        }
    }

    // Package manager detection
    if workspace.join("pnpm-lock.yaml").exists() {
        profile.package_managers.push("pnpm".to_string());
    } else if workspace.join("yarn.lock").exists() {
        profile.package_managers.push("yarn".to_string());
    } else if workspace.join("bun.lockb").exists() || workspace.join("bun.lock").exists() {
        profile.package_managers.push("bun".to_string());
    } else {
        profile.package_managers.push("npm".to_string());
    }
}

fn detect_python(workspace: &Path, profile: &mut ProjectProfile) {
    let has_python = workspace.join("pyproject.toml").exists()
        || workspace.join("setup.py").exists()
        || workspace.join("requirements.txt").exists()
        || workspace.join("Pipfile").exists()
        || workspace.join("setup.cfg").exists();

    if !has_python {
        return;
    }
    profile.languages.push("Python".to_string());

    // Framework detection
    if let Ok(content) = std::fs::read_to_string(workspace.join("pyproject.toml"))
        .or_else(|_| std::fs::read_to_string(workspace.join("requirements.txt")))
    {
        if content.contains("django") {
            profile.frameworks.push("Django".to_string());
        }
        if content.contains("flask") {
            profile.frameworks.push("Flask".to_string());
        }
        if content.contains("fastapi") {
            profile.frameworks.push("FastAPI".to_string());
        }
        if content.contains("pytorch") || content.contains("torch") {
            profile.frameworks.push("PyTorch".to_string());
        }
        if content.contains("tensorflow") {
            profile.frameworks.push("TensorFlow".to_string());
        }
        if content.contains("streamlit") {
            profile.frameworks.push("Streamlit".to_string());
        }
    }

    // Test commands
    if workspace.join("pytest.ini").exists()
        || workspace.join("pyproject.toml").exists()
        || workspace.join("conftest.py").exists()
    {
        profile.test_commands.push(TestCommand {
            label: "Pytest".to_string(),
            command: "python -m pytest".to_string(),
            framework: "pytest".to_string(),
        });
    }

    // Build/run
    if workspace.join("manage.py").exists() {
        profile.build_commands.push(BuildCommand {
            label: "Django runserver".to_string(),
            command: "python manage.py runserver".to_string(),
            working_dir: None,
        });
    }

    // Lint
    if workspace.join("pyproject.toml").exists() {
        profile.lint_commands.push(LintCommand {
            label: "Ruff".to_string(),
            command: "ruff check .".to_string(),
        });
    }

    // Package manager
    if workspace.join("poetry.lock").exists() {
        profile.package_managers.push("poetry".to_string());
    } else if workspace.join("Pipfile.lock").exists() {
        profile.package_managers.push("pipenv".to_string());
    } else if workspace.join("uv.lock").exists() {
        profile.package_managers.push("uv".to_string());
    } else {
        profile.package_managers.push("pip".to_string());
    }
}

fn detect_go(workspace: &Path, profile: &mut ProjectProfile) {
    if !workspace.join("go.mod").exists() {
        return;
    }
    profile.languages.push("Go".to_string());
    profile.build_commands.push(BuildCommand {
        label: "Go build".to_string(),
        command: "go build ./...".to_string(),
        working_dir: None,
    });
    profile.test_commands.push(TestCommand {
        label: "Go test".to_string(),
        command: "go test ./...".to_string(),
        framework: "go test".to_string(),
    });
    profile.lint_commands.push(LintCommand {
        label: "Go vet".to_string(),
        command: "go vet ./...".to_string(),
    });
    profile.package_managers.push("go modules".to_string());

    if let Ok(content) = std::fs::read_to_string(workspace.join("go.mod")) {
        if content.contains("github.com/gin-gonic") {
            profile.frameworks.push("Gin".to_string());
        }
        if content.contains("github.com/gofiber") {
            profile.frameworks.push("Fiber".to_string());
        }
        if content.contains("github.com/labstack/echo") {
            profile.frameworks.push("Echo".to_string());
        }
    }
}

fn detect_java(workspace: &Path, profile: &mut ProjectProfile) {
    let has_maven = workspace.join("pom.xml").exists();
    let has_gradle = workspace.join("build.gradle").exists() || workspace.join("build.gradle.kts").exists();
    if !has_maven && !has_gradle {
        return;
    }
    profile.languages.push("Java".to_string());
    if has_maven {
        profile.build_commands.push(BuildCommand {
            label: "Maven build".to_string(),
            command: "mvn clean install".to_string(),
            working_dir: None,
        });
        profile.test_commands.push(TestCommand {
            label: "Maven test".to_string(),
            command: "mvn test".to_string(),
            framework: "JUnit/Maven".to_string(),
        });
        profile.package_managers.push("maven".to_string());
    }
    if has_gradle {
        let wrapper = if workspace.join("gradlew").exists() { "./gradlew" } else { "gradle" };
        profile.build_commands.push(BuildCommand {
            label: "Gradle build".to_string(),
            command: format!("{} build", wrapper),
            working_dir: None,
        });
        profile.test_commands.push(TestCommand {
            label: "Gradle test".to_string(),
            command: format!("{} test", wrapper),
            framework: "JUnit/Gradle".to_string(),
        });
        profile.package_managers.push("gradle".to_string());
    }
}

fn detect_dotnet(workspace: &Path, profile: &mut ProjectProfile) {
    let has_csproj = std::fs::read_dir(workspace)
        .map(|entries| entries.filter_map(|e| e.ok())
            .any(|e| e.path().extension().map(|x| x == "csproj" || x == "fsproj").unwrap_or(false)))
        .unwrap_or(false);
    let has_sln = std::fs::read_dir(workspace)
        .map(|entries| entries.filter_map(|e| e.ok())
            .any(|e| e.path().extension().map(|x| x == "sln").unwrap_or(false)))
        .unwrap_or(false);

    if !has_csproj && !has_sln {
        return;
    }
    profile.languages.push("C#".to_string());
    profile.build_commands.push(BuildCommand {
        label: "dotnet build".to_string(),
        command: "dotnet build".to_string(),
        working_dir: None,
    });
    profile.test_commands.push(TestCommand {
        label: "dotnet test".to_string(),
        command: "dotnet test".to_string(),
        framework: "xUnit/NUnit".to_string(),
    });
    profile.package_managers.push("nuget".to_string());
}

fn detect_ruby(workspace: &Path, profile: &mut ProjectProfile) {
    if !workspace.join("Gemfile").exists() {
        return;
    }
    profile.languages.push("Ruby".to_string());
    if workspace.join("Rakefile").exists() || workspace.join("config/routes.rb").exists() {
        profile.frameworks.push("Rails".to_string());
        profile.build_commands.push(BuildCommand {
            label: "Rails server".to_string(),
            command: "bundle exec rails server".to_string(),
            working_dir: None,
        });
        profile.test_commands.push(TestCommand {
            label: "RSpec".to_string(),
            command: "bundle exec rspec".to_string(),
            framework: "RSpec".to_string(),
        });
    }
    profile.package_managers.push("bundler".to_string());
}

fn detect_php(workspace: &Path, profile: &mut ProjectProfile) {
    if !workspace.join("composer.json").exists() {
        return;
    }
    profile.languages.push("PHP".to_string());
    if let Ok(content) = std::fs::read_to_string(workspace.join("composer.json")) {
        if content.contains("laravel") {
            profile.frameworks.push("Laravel".to_string());
        }
        if content.contains("symfony") {
            profile.frameworks.push("Symfony".to_string());
        }
    }
    if workspace.join("phpunit.xml").exists() || workspace.join("phpunit.xml.dist").exists() {
        profile.test_commands.push(TestCommand {
            label: "PHPUnit".to_string(),
            command: "vendor/bin/phpunit".to_string(),
            framework: "PHPUnit".to_string(),
        });
    }
    profile.package_managers.push("composer".to_string());
}

// ── Architecture Detection ───────────────────────────────────────────────

fn detect_architecture(workspace: &Path, profile: &mut ProjectProfile) {
    // Monorepo signals
    let has_workspace_cargo = workspace.join("Cargo.toml").exists()
        && std::fs::read_to_string(workspace.join("Cargo.toml"))
            .map(|c| c.contains("[workspace]"))
            .unwrap_or(false);
    let has_workspace_npm = workspace.join("package.json").exists()
        && std::fs::read_to_string(workspace.join("package.json"))
            .map(|c| c.contains("\"workspaces\""))
            .unwrap_or(false);
    let has_lerna = workspace.join("lerna.json").exists();
    let has_nx = workspace.join("nx.json").exists();
    let has_pnpm_ws = workspace.join("pnpm-workspace.yaml").exists();
    let has_turborepo = workspace.join("turbo.json").exists();

    if has_workspace_cargo || has_workspace_npm || has_lerna || has_nx || has_pnpm_ws || has_turborepo {
        profile.architecture = ProjectArchitecture::Monorepo;
        return;
    }

    // Library signals
    if profile.languages.contains(&"Rust".to_string()) {
        if let Ok(content) = std::fs::read_to_string(workspace.join("Cargo.toml")) {
            if content.contains("[lib]") && !content.contains("[[bin]]") {
                profile.architecture = ProjectArchitecture::Library;
                return;
            }
        }
    }

    // CLI tool signals
    if workspace.join("src/main.rs").exists()
        || workspace.join("cmd/").exists()
        || workspace.join("bin/").exists()
    {
        if !workspace.join("src/App.tsx").exists()
            && !workspace.join("src/App.jsx").exists()
            && !workspace.join("src/index.html").exists()
            && !workspace.join("public/index.html").exists()
        {
            // Has binary entry but no frontend — likely CLI
            if profile.languages.len() == 1 {
                profile.architecture = ProjectArchitecture::CLITool;
                return;
            }
        }
    }

    // Full-stack signals
    let has_frontend = workspace.join("src/App.tsx").exists()
        || workspace.join("src/App.jsx").exists()
        || workspace.join("src/App.vue").exists()
        || workspace.join("pages/").exists()
        || workspace.join("app/").exists();
    let has_backend = workspace.join("src/main.rs").exists()
        || workspace.join("server/").exists()
        || workspace.join("api/").exists()
        || workspace.join("manage.py").exists()
        || workspace.join("cmd/").exists();

    if has_frontend && has_backend {
        profile.architecture = ProjectArchitecture::FullStackApp;
        return;
    }

    // Microservice signals
    let docker_compose = workspace.join("docker-compose.yml").exists()
        || workspace.join("docker-compose.yaml").exists();
    if docker_compose {
        if let Ok(content) = std::fs::read_to_string(workspace.join("docker-compose.yml"))
            .or_else(|_| std::fs::read_to_string(workspace.join("docker-compose.yaml")))
        {
            let service_count = content.matches("image:").count() + content.matches("build:").count();
            if service_count >= 3 {
                profile.architecture = ProjectArchitecture::MicroserviceCluster;
                return;
            }
        }
    }

    profile.architecture = ProjectArchitecture::SinglePackage;
}

// ── Key File Collection ──────────────────────────────────────────────────

fn collect_key_files(workspace: &Path, profile: &mut ProjectProfile) {
    let key_file_specs: Vec<(&[&str], KeyFileRole, usize)> = vec![
        (&["README.md", "readme.md", "README.rst", "README"], KeyFileRole::Readme, 60),
        (&["CONTRIBUTING.md", "CONTRIBUTING.rst"], KeyFileRole::Contributing, 30),
        (&[".env.example", ".env.sample", ".env.template"], KeyFileRole::EnvExample, 40),
        (&["Dockerfile", "dockerfile"], KeyFileRole::Dockerfile, 30),
        (&[
            ".github/workflows/ci.yml", ".github/workflows/ci.yaml",
            ".github/workflows/build.yml", ".github/workflows/test.yml",
            ".gitlab-ci.yml", ".circleci/config.yml", "Jenkinsfile",
        ], KeyFileRole::CIConfig, 30),
        (&["openapi.yaml", "openapi.json", "swagger.yaml", "swagger.json", "api.yaml"], KeyFileRole::APISpec, 40),
        (&[
            "tsconfig.json", "vite.config.ts", "next.config.js", "next.config.mjs",
            "webpack.config.js", "tailwind.config.js", "tailwind.config.ts",
        ], KeyFileRole::Config, 20),
        (&[
            "jest.config.js", "jest.config.ts", "vitest.config.ts",
            "pytest.ini", "phpunit.xml",
        ], KeyFileRole::TestConfig, 15),
    ];

    for (filenames, role, max_lines) in key_file_specs {
        for filename in filenames {
            let path = workspace.join(filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let preview: String = content.lines()
                        .take(max_lines)
                        .collect::<Vec<_>>()
                        .join("\n");
                    profile.key_files.push(KeyFile {
                        path: filename.to_string(),
                        role: role.clone(),
                        preview,
                    });
                }
                break; // Only first match per role
            }
        }
    }

    // Schema files (search for common patterns)
    for schema_name in &[
        "schema.prisma", "prisma/schema.prisma",
        "drizzle.config.ts", "knexfile.js",
        "migrations/", "db/schema.rb",
    ] {
        let path = workspace.join(schema_name);
        if path.exists() && path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let preview: String = content.lines().take(30).collect::<Vec<_>>().join("\n");
                profile.key_files.push(KeyFile {
                    path: schema_name.to_string(),
                    role: KeyFileRole::Schema,
                    preview,
                });
            }
            break;
        }
    }
}

// ── Entry Points ─────────────────────────────────────────────────────────

fn detect_entry_points(workspace: &Path, profile: &mut ProjectProfile) {
    let candidates = [
        "src/main.rs", "src/lib.rs",
        "src/index.ts", "src/index.tsx", "src/index.js", "src/index.jsx",
        "src/App.tsx", "src/App.jsx", "src/App.vue", "src/App.svelte",
        "src/main.ts", "src/main.tsx", "src/main.js",
        "main.go", "cmd/main.go",
        "app/main.py", "main.py", "app.py", "manage.py",
        "src/main/java", "src/main/kotlin",
        "index.html", "public/index.html",
        "pages/index.tsx", "pages/index.jsx",
        "app/page.tsx", "app/page.jsx",
    ];

    for candidate in &candidates {
        let path = workspace.join(candidate);
        if path.exists() {
            profile.entry_points.push(candidate.to_string());
        }
    }
}

// ── Env Var Detection ────────────────────────────────────────────────────

fn detect_env_vars(workspace: &Path, profile: &mut ProjectProfile) {
    // Read .env.example or .env.sample
    for env_file in &[".env.example", ".env.sample", ".env.template"] {
        let path = workspace.join(env_file);
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some(key) = trimmed.split('=').next() {
                        let key = key.trim();
                        if !key.is_empty() {
                            profile.env_vars.push(key.to_string());
                        }
                    }
                }
            }
            break;
        }
    }
}

// ── Summary Generator ────────────────────────────────────────────────────

fn generate_summary(profile: &ProjectProfile) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "**{}** is a {} project",
        profile.name, profile.architecture,
    ));

    if !profile.languages.is_empty() {
        parts.push(format!("using {}", profile.languages.join(", ")));
    }
    if !profile.frameworks.is_empty() {
        parts.push(format!("with {}", profile.frameworks.join(", ")));
    }

    let mut summary = parts.join(" ") + ".";

    if !profile.build_commands.is_empty() {
        summary.push_str("\n\nBuild: ");
        let cmds: Vec<_> = profile.build_commands.iter()
            .map(|c| format!("`{}`", c.command))
            .collect();
        summary.push_str(&cmds.join(" | "));
    }
    if !profile.test_commands.is_empty() {
        summary.push_str("\nTest: ");
        let cmds: Vec<_> = profile.test_commands.iter()
            .map(|c| format!("`{}` ({})", c.command, c.framework))
            .collect();
        summary.push_str(&cmds.join(" | "));
    }
    if !profile.lint_commands.is_empty() {
        summary.push_str("\nLint: ");
        let cmds: Vec<_> = profile.lint_commands.iter()
            .map(|c| format!("`{}`", c.command))
            .collect();
        summary.push_str(&cmds.join(" | "));
    }
    if !profile.entry_points.is_empty() {
        summary.push_str(&format!("\nEntry points: {}", profile.entry_points.join(", ")));
    }

    summary
}

fn extract_description(workspace: &Path) -> Option<String> {
    // Try README first paragraph
    for name in &["README.md", "readme.md"] {
        let path = workspace.join(name);
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().take(15) {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && !trimmed.starts_with("![")
                    && !trimmed.starts_with('[')
                    && !trimmed.starts_with("---")
                {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    // Try package.json description
    if let Ok(content) = std::fs::read_to_string(workspace.join("package.json")) {
        if let Some(start) = content.find("\"description\"") {
            let rest = &content[start..];
            if let Some(colon_pos) = rest.find(':') {
                let after_colon = rest[colon_pos + 1..].trim();
                if let Some(start_quote) = after_colon.find('"') {
                    let inner = &after_colon[start_quote + 1..];
                    if let Some(end_quote) = inner.find('"') {
                        return Some(inner[..end_quote].to_string());
                    }
                }
            }
        }
    }
    None
}

fn detect_npm_runner(workspace: &Path) -> &'static str {
    if workspace.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if workspace.join("yarn.lock").exists() {
        "yarn"
    } else if workspace.join("bun.lockb").exists() || workspace.join("bun.lock").exists() {
        "bun"
    } else {
        "npm"
    }
}

// ── Task-Based Auto-Context ──────────────────────────────────────────────

/// Analyze a user's task description and return file paths likely relevant to it.
/// This enables automatic context gathering — the key feature gap vs Cursor/Windsurf.
pub fn extract_relevant_files_for_task(workspace: &Path, task: &str) -> Vec<String> {
    let task_lower = task.to_lowercase();
    let mut relevant: Vec<(String, u32)> = Vec::new(); // (path, score)

    // 1. Extract explicit file paths mentioned in the task
    for word in task.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == ',');
        if (cleaned.contains('/') || cleaned.contains('.'))
            && !cleaned.starts_with("http")
            && !cleaned.starts_with("www")
        {
            let path = workspace.join(cleaned);
            if path.exists() {
                relevant.push((cleaned.to_string(), 100));
            }
        }
    }

    // 2. Keyword-based file matching
    let keyword_file_map: Vec<(&[&str], &[&str])> = vec![
        (&["test", "spec", "testing"], &["tests/", "test/", "__tests__/", "spec/", "pytest.ini", "jest.config.js", "vitest.config.ts"]),
        (&["build", "compile", "bundle"], &["Cargo.toml", "package.json", "tsconfig.json", "webpack.config.js", "vite.config.ts"]),
        (&["deploy", "ci", "pipeline"], &[".github/workflows/", ".gitlab-ci.yml", "Dockerfile", "docker-compose.yml"]),
        (&["auth", "login", "session", "jwt"], &["src/auth/", "src/middleware/", "src/lib/auth/"]),
        (&["database", "db", "migration", "schema", "model"], &["prisma/", "migrations/", "src/models/", "src/db/", "schema.prisma"]),
        (&["api", "endpoint", "route", "handler"], &["src/api/", "src/routes/", "src/handlers/", "src/controllers/", "pages/api/"]),
        (&["style", "css", "theme", "design"], &["src/styles/", "tailwind.config.js", "src/App.css", "src/index.css"]),
        (&["config", "setting", "env"], &[".env.example", "config/", "src/config/", "src/config.rs", "src/config.ts"]),
        (&["readme", "doc", "documentation"], &["README.md", "docs/", "CONTRIBUTING.md"]),
        (&["lint", "format", "eslint", "prettier", "clippy"], &[".eslintrc", "eslint.config.js", "biome.json", "rustfmt.toml"]),
        (&["component", "ui", "widget", "panel"], &["src/components/", "src/ui/"]),
        (&["hook", "middleware", "plugin"], &["src/hooks/", "src/middleware/", "src/plugins/"]),
        (&["error", "bug", "fix", "crash"], &["src/", "Cargo.toml", "package.json"]),
        (&["performance", "optimize", "speed", "memory"], &["src/", "Cargo.toml"]),
        (&["security", "vulnerability", "xss", "injection"], &["src/", "Cargo.toml", "package.json"]),
    ];

    for (keywords, file_patterns) in &keyword_file_map {
        if keywords.iter().any(|k| task_lower.contains(k)) {
            for pattern in *file_patterns {
                let path = workspace.join(pattern);
                if path.exists() {
                    relevant.push((pattern.to_string(), 50));
                }
            }
        }
    }

    // 3. Always include key project files for brownfield understanding
    for key_file in &["README.md", "package.json", "Cargo.toml", "go.mod", "pyproject.toml"] {
        if workspace.join(key_file).exists() && !relevant.iter().any(|(p, _)| p == *key_file) {
            relevant.push((key_file.to_string(), 10));
        }
    }

    // Sort by score (highest first), deduplicate
    relevant.sort_by(|a, b| b.1.cmp(&a.1));
    let mut seen = std::collections::HashSet::new();
    relevant
        .into_iter()
        .filter(|(path, _)| seen.insert(path.clone()))
        .map(|(path, _)| path)
        .take(15)
        .collect()
}

// ── Profile Cache ────────────────────────────────────────────────────────

/// Load a cached project profile from `.vibecli/project-profile.json`.
/// Returns `None` if no cache exists or it's older than `max_age_secs`.
pub fn load_cached_profile(workspace: &Path, max_age_secs: u64) -> Option<ProjectProfile> {
    let cache_path = workspace.join(".vibecli").join("project-profile.json");
    if let Ok(content) = std::fs::read_to_string(&cache_path) {
        if let Ok(profile) = serde_json::from_str::<ProjectProfile>(&content) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now - profile.scanned_at < max_age_secs {
                return Some(profile);
            }
        }
    }
    None
}

/// Save the project profile to `.vibecli/project-profile.json`.
pub fn save_profile_cache(workspace: &Path, profile: &ProjectProfile) -> std::io::Result<()> {
    let dir = workspace.join(".vibecli");
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(profile)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(dir.join("project-profile.json"), content)
}

/// Get or create a project profile (cached for 1 hour).
pub fn get_or_scan_profile(workspace: &Path) -> ProjectProfile {
    // Try cache first (1 hour TTL)
    if let Some(cached) = load_cached_profile(workspace, 3600) {
        return cached;
    }
    let profile = scan_workspace(workspace);
    let _ = save_profile_cache(workspace, &profile);
    profile
}

// ── Display ──────────────────────────────────────────────────────────────

impl ProjectProfile {
    /// Render a human-readable project orientation report.
    pub fn display(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!("Project: {}\n", self.name));
        if !self.description.is_empty() {
            out.push_str(&format!("  {}\n", self.description));
        }
        out.push_str(&format!("Architecture: {}\n", self.architecture));
        if !self.languages.is_empty() {
            out.push_str(&format!("Languages: {}\n", self.languages.join(", ")));
        }
        if !self.frameworks.is_empty() {
            out.push_str(&format!("Frameworks: {}\n", self.frameworks.join(", ")));
        }
        if !self.package_managers.is_empty() {
            out.push_str(&format!("Package managers: {}\n", self.package_managers.join(", ")));
        }

        if !self.build_commands.is_empty() {
            out.push_str("\nBuild commands:\n");
            for cmd in &self.build_commands {
                out.push_str(&format!("  {} → {}\n", cmd.label, cmd.command));
            }
        }
        if !self.test_commands.is_empty() {
            out.push_str("\nTest commands:\n");
            for cmd in &self.test_commands {
                out.push_str(&format!("  {} → {} ({})\n", cmd.label, cmd.command, cmd.framework));
            }
        }
        if !self.lint_commands.is_empty() {
            out.push_str("\nLint commands:\n");
            for cmd in &self.lint_commands {
                out.push_str(&format!("  {} → {}\n", cmd.label, cmd.command));
            }
        }
        if !self.entry_points.is_empty() {
            out.push_str(&format!("\nEntry points: {}\n", self.entry_points.join(", ")));
        }
        if !self.env_vars.is_empty() {
            out.push_str(&format!("\nExpected env vars: {}\n", self.env_vars.join(", ")));
        }
        if !self.key_files.is_empty() {
            out.push_str(&format!("\nKey files: {}\n",
                self.key_files.iter().map(|f| format!("{} ({})", f.path, f.role)).collect::<Vec<_>>().join(", ")));
        }

        out
    }

    /// Generate a concise context block for injection into the LLM system prompt.
    /// This is the key differentiator — always-available project understanding.
    pub fn to_system_prompt_context(&self) -> String {
        let mut ctx = String::from("\n\n## Project Context\n");
        ctx.push_str(&self.summary);

        // Include key file previews (README especially)
        for kf in &self.key_files {
            if kf.role == KeyFileRole::Readme && !kf.preview.is_empty() {
                ctx.push_str(&format!("\n\n### {} (first lines)\n```\n{}\n```", kf.path, kf.preview));
                break;
            }
        }

        // Include env vars if any
        if !self.env_vars.is_empty() {
            ctx.push_str(&format!("\n\nRequired env vars: {}", self.env_vars.join(", ")));
        }

        ctx
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_workspace() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn scan_empty_workspace() {
        let (_dir, path) = temp_workspace();
        let profile = scan_workspace(&path);
        assert_eq!(profile.architecture, ProjectArchitecture::SinglePackage);
        assert!(profile.languages.is_empty());
    }

    #[test]
    fn scan_rust_project() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Cargo.toml"), r#"
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
tokio = "1"
"#).unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/main.rs"), "fn main() {}").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.languages.contains(&"Rust".to_string()));
        assert!(profile.frameworks.contains(&"Tokio".to_string()));
        assert!(!profile.build_commands.is_empty());
        assert!(!profile.test_commands.is_empty());
        assert!(profile.entry_points.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn scan_node_react_project() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("package.json"), r#"{
            "name": "my-react-app",
            "description": "A cool app",
            "dependencies": { "react": "^18.0.0", "vite": "^5.0.0" },
            "devDependencies": { "vitest": "^1.0.0", "eslint": "^8.0.0" },
            "scripts": { "build": "vite build", "test": "vitest", "dev": "vite" }
        }"#).unwrap();
        fs::write(path.join("tsconfig.json"), "{}").unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/App.tsx"), "export default function App() {}").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.languages.contains(&"TypeScript".to_string()));
        assert!(profile.frameworks.contains(&"React".to_string()));
        assert!(profile.frameworks.contains(&"Vite".to_string()));
        assert!(!profile.test_commands.is_empty());
        assert!(profile.test_commands.iter().any(|t| t.framework == "Vitest"));
    }

    #[test]
    fn scan_monorepo() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]").unwrap();
        fs::write(path.join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();

        let profile = scan_workspace(&path);
        assert_eq!(profile.architecture, ProjectArchitecture::Monorepo);
    }

    #[test]
    fn scan_python_django() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("requirements.txt"), "django>=4.0\ndjango-rest-framework").unwrap();
        fs::write(path.join("manage.py"), "#!/usr/bin/env python").unwrap();
        fs::write(path.join("pytest.ini"), "[pytest]").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.languages.contains(&"Python".to_string()));
        assert!(profile.frameworks.contains(&"Django".to_string()));
        assert!(profile.test_commands.iter().any(|t| t.framework == "pytest"));
    }

    #[test]
    fn scan_go_project() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("go.mod"), "module example.com/myapp\n\nrequire github.com/gin-gonic/gin v1.9.0").unwrap();
        fs::write(path.join("main.go"), "package main").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.languages.contains(&"Go".to_string()));
        assert!(profile.frameworks.contains(&"Gin".to_string()));
    }

    #[test]
    fn key_files_collected() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("README.md"), "# My Project\n\nThis is a great project.\n\n## Setup\nRun npm install.").unwrap();
        fs::write(path.join("Dockerfile"), "FROM node:20\nWORKDIR /app\nCOPY . .\nRUN npm install").unwrap();
        fs::write(path.join(".env.example"), "DATABASE_URL=postgres://...\nAPI_KEY=your-key").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.key_files.iter().any(|f| f.role == KeyFileRole::Readme));
        assert!(profile.key_files.iter().any(|f| f.role == KeyFileRole::Dockerfile));
        assert!(profile.key_files.iter().any(|f| f.role == KeyFileRole::EnvExample));
        assert!(profile.env_vars.contains(&"DATABASE_URL".to_string()));
        assert!(profile.env_vars.contains(&"API_KEY".to_string()));
    }

    #[test]
    fn extract_relevant_files_finds_explicit_paths() {
        let (_dir, path) = temp_workspace();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let files = extract_relevant_files_for_task(&path, "fix the bug in src/main.rs");
        assert!(files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn extract_relevant_files_for_test_task() {
        let (_dir, path) = temp_workspace();
        fs::create_dir_all(path.join("tests")).unwrap();
        fs::write(path.join("tests/test_main.py"), "").unwrap();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let files = extract_relevant_files_for_task(&path, "add unit tests for the auth module");
        assert!(files.iter().any(|f| f.contains("test")));
    }

    #[test]
    fn extract_relevant_files_for_deploy_task() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Dockerfile"), "FROM node:20").unwrap();
        fs::write(path.join("docker-compose.yml"), "services:").unwrap();

        let files = extract_relevant_files_for_task(&path, "deploy to production");
        assert!(files.iter().any(|f| f.contains("Dockerfile") || f.contains("docker")));
    }

    #[test]
    fn profile_cache_round_trip() {
        let (_dir, path) = temp_workspace();
        let profile = scan_workspace(&path);
        save_profile_cache(&path, &profile).unwrap();

        let loaded = load_cached_profile(&path, 3600);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, profile.name);
    }

    #[test]
    fn summary_generation() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"myapp\"\n\n[dependencies]\ntokio = \"1\"").unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/main.rs"), "fn main() {}").unwrap();

        let profile = scan_workspace(&path);
        assert!(profile.summary.contains("Rust"));
        assert!(profile.summary.contains("cargo"));
    }

    #[test]
    fn system_prompt_context_generation() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("README.md"), "# Hello World\n\nA simple Rust project.").unwrap();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"hello\"").unwrap();

        let profile = scan_workspace(&path);
        let ctx = profile.to_system_prompt_context();
        assert!(ctx.contains("## Project Context"));
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("Hello World"));
    }

    #[test]
    fn display_output() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("package.json"), r#"{"name":"myapp","dependencies":{"react":"18"},"scripts":{"build":"vite build"}}"#).unwrap();
        fs::write(path.join("tsconfig.json"), "{}").unwrap();

        let profile = scan_workspace(&path);
        let display = profile.display();
        // Name comes from temp directory, not package.json
        assert!(display.contains("TypeScript"));
        assert!(display.contains("React"));
    }

    #[test]
    fn detect_library_architecture() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"mylib\"\n\n[lib]\nname = \"mylib\"").unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/lib.rs"), "pub fn hello() {}").unwrap();

        let profile = scan_workspace(&path);
        assert_eq!(profile.architecture, ProjectArchitecture::Library);
    }

    #[test]
    fn detect_fullstack_architecture() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("Cargo.toml"), "[package]\nname = \"fullstack\"").unwrap();
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(path.join("src/App.tsx"), "export default function App() {}").unwrap();

        let profile = scan_workspace(&path);
        assert_eq!(profile.architecture, ProjectArchitecture::FullStackApp);
    }

    #[test]
    fn npm_runner_detection() {
        let (_dir, path) = temp_workspace();
        fs::write(path.join("pnpm-lock.yaml"), "lockfileVersion: 6").unwrap();
        assert_eq!(detect_npm_runner(&path), "pnpm");

        let (_dir2, path2) = temp_workspace();
        fs::write(path2.join("yarn.lock"), "# yarn lock").unwrap();
        assert_eq!(detect_npm_runner(&path2), "yarn");

        let (_dir3, path3) = temp_workspace();
        assert_eq!(detect_npm_runner(&path3), "npm");
    }
}
