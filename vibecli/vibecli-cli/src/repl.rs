use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator, ValidationContext, ValidationResult};
use rustyline::{Context, Helper};
use std::borrow::Cow;

// ── All known top-level slash commands ────────────────────────────────────────

pub static COMMANDS: &[&str] = &[
    "/agent",
    "/agents",
    "/aiml",
    "/arena",
    "/apply",
    "/autofix",
    "/bisect",
    "/chat",
    "/compliance",
    "/config",
    "/cost",
    "/context",
    "/demo",
    "/deploy",
    "/deps",
    "/diff",
    "/env",
    "/exec",
    "/exit",
    "/fork",
    "/generate",
    "/help",
    "/index",
    "/init",
    "/jobs",
    "/linear",
    "/logs",
    "/markers",
    "/memory",
    "/mcp",
    "/mock",
    "/migration",
    "/model",
    "/notebook",
    "/plan",
    "/plugin",
    "/profile",
    "/profiler",
    "/qa",
    "/quit",
    "/redteam",
    "/remind",
    "/resume",
    "/rewind",
    "/sandbox",
    "/schedule",
    "/sessions",
    "/share",
    "/snippet",
    "/soul",
    "/spec",
    "/status",
    "/team",
    "/test",
    "/theme",
    "/trace",
    "/transform",
    "/marketplace",
    "/voice",
    "/discover",
    "/pair",
    "/workflow",
    "/orchestrate",
    "/verify",
    "/handoff",
    "/orient",
    "/research",
    "/appbuilder",
    "/icontext",
    "/batch",
    "/qavalidate",
    "/legacymigrate",
    "/gitplatform",
    "/bundle",
    "/cloud",
    "/benchmark",
    "/metering",
    "/blueteam",
    "/purpleteam",
    "/idp",
    "/quantum",
    "/autoresearch",
    "/daemon",
    "/vm",
    "/branch-agent",
    "/design",
    "/audio",
    "/org",
    "/share-session",
    "/data",
    "/ci-gates",
    "/extension",
    "/agentic",
    "/openmemory",
    "/vulnscan",
    "/wizard",
    "/resources",
];

// ── Sub-command tables ────────────────────────────────────────────────────────

/// Sub-commands for `/profile <sub>`
static PROFILE_SUBS: &[&str] = &["list", "show", "create", "delete"];

/// Sub-commands for `/plugin <sub>`
static PLUGIN_SUBS: &[&str] = &["list", "install", "remove", "info"];

/// Sub-commands for `/memory <sub>`
static MEMORY_SUBS: &[&str] = &["show", "edit"];

/// Sub-commands for `/spec <sub>`
static SPEC_SUBS: &[&str] = &["list", "show", "new", "run", "done"];

/// Sub-commands for `/agents <sub>`
static AGENTS_SUBS: &[&str] = &["list", "status", "new"];

/// Sub-commands for `/team <sub>`
static TEAM_SUBS: &[&str] = &["create", "status", "messages", "show", "knowledge", "sync"];

/// Sub-commands for `/trace <sub>`
static TRACE_SUBS: &[&str] = &["view"];

/// Sub-commands for `/mcp <sub>`
static MCP_SUBS: &[&str] = &["list", "tools"];

/// Sub-commands for `/snippet <sub>`
static SNIPPET_SUBS: &[&str] = &["list", "save", "use", "show", "delete"];

/// Sub-commands for `/linear <sub>`
static LINEAR_SUBS: &[&str] = &["list", "new", "open", "attach"];

/// Sub-commands for `/logs <sub>`
static LOGS_SUBS: &[&str] = &["tail", "sources", "errors", "analyze"];

/// Sub-commands for `/remind <sub>`
static REMIND_SUBS: &[&str] = &["in", "list", "cancel"];

/// Sub-commands for `/schedule <sub>`
static SCHEDULE_SUBS: &[&str] = &["every", "list", "cancel"];

/// Sub-commands for `/workflow <sub>`
static WORKFLOW_SUBS: &[&str] = &["new", "list", "show", "advance", "check", "generate"];

/// Sub-commands for `/orchestrate <sub>`
static ORCHESTRATE_SUBS: &[&str] = &["status", "lessons", "lesson", "todo", "verify", "reset"];

/// Sub-commands for `/sandbox <sub>`
static SANDBOX_SUBS: &[&str] = &["status", "start", "stop", "list", "exec", "logs", "runtime"];

/// Sub-commands for `/appbuilder <sub>`
static APPBUILDER_SUBS: &[&str] = &["enhance", "template", "provision", "scaffold", "templates"];

/// Sub-commands for `/icontext <sub>`
static ICONTEXT_SUBS: &[&str] = &["status", "expand", "compress", "refresh", "summary", "clear"];

/// Sub-commands for `/batch <sub>`
static BATCH_SUBS: &[&str] = &["new", "start", "pause", "resume", "cancel", "status", "list", "estimate", "history"];

/// Sub-commands for `/qavalidate <sub>`
static QAVALIDATE_SUBS: &[&str] = &["run", "status", "report", "findings", "resolve", "config", "history"];

/// Sub-commands for `/legacymigrate <sub>`
static LEGACYMIGRATE_SUBS: &[&str] = &["analyze", "plan", "translate", "validate", "report", "rules", "pairs"];

/// Sub-commands for `/gitplatform <sub>`
static GITPLATFORM_SUBS: &[&str] = &["add", "list", "remove", "default", "pr", "issue", "pipeline", "webhook"];

/// Sub-commands for `/bundle <sub>`
static BUNDLE_SUBS: &[&str] = &["create", "activate", "deactivate", "list", "share", "import", "export", "delete"];

/// Sub-commands for `/cloud <sub>`
static CLOUD_SUBS: &[&str] = &["scan", "iam", "terraform", "cloudformation", "pulumi", "cost", "providers"];

/// Sub-commands for `/benchmark <sub>`
static BENCHMARK_SUBS: &[&str] = &["run", "compare", "export", "list"];

/// Sub-commands for `/metering <sub>`
static METERING_SUBS: &[&str] = &["status", "budget", "report", "alerts", "top"];

/// Sub-commands for `/blueteam <sub>`
static BLUETEAM_SUBS: &[&str] = &["status", "scan", "incidents", "iocs", "rules", "forensics", "playbooks", "siem", "hunt", "report"];

/// Sub-commands for `/purpleteam <sub>`
static PURPLETEAM_SUBS: &[&str] = &["status", "exercise", "simulate", "validate", "matrix", "gaps", "heatmap", "report"];

/// Sub-commands for `/idp <sub>`
static IDP_SUBS: &[&str] = &["status", "catalog", "register", "golden", "scorecard", "infra", "team", "onboard", "backstage", "platforms", "report"];

/// Sub-commands for `/quantum <sub>`
static QUANTUM_SUBS: &[&str] = &["languages", "os", "hardware", "algorithms", "circuits", "projects", "create", "export", "compat", "status"];

/// Sub-commands for `/autoresearch <sub>`
static AUTORESEARCH_SUBS: &[&str] = &["new", "start", "stop", "pause", "status", "list", "analyze", "export", "suggest", "lessons", "config"];

/// Sub-commands for `/openmemory <sub>`
static OPENMEMORY_SUBS: &[&str] = &["add", "query", "list", "delete", "pin", "unpin", "fact", "facts", "decay", "consolidate", "reflect", "summary", "health", "at-risk", "dedup", "ingest", "import", "stats", "export", "encrypt", "context"];

/// Sub-commands for `/vulnscan <sub>`
static VULNSCAN_SUBS: &[&str] = &["scan", "deps", "file", "lockfile", "sarif", "report", "summary", "db-update", "db-status", "cache-clear"];

/// Sub-commands for `/resources <sub>`
static RESOURCES_SUBS: &[&str] = &["status", "export", "verify", "path"];

/// Sub-commands for `/arena <sub>`
static ARENA_SUBS: &[&str] = &["compare", "stats", "history"];

/// Sub-commands for `/bisect <sub>`
static BISECT_SUBS: &[&str] = &["start", "good", "bad", "skip", "reset", "log", "analyze"];

/// Sub-commands for `/markers <sub>`
static MARKERS_SUBS: &[&str] = &["scan", "list", "bookmarks"];

/// Sub-commands for `/mock <sub>`
static MOCK_SUBS: &[&str] = &["start", "stop", "add", "remove", "list", "log", "import"];

/// Sub-commands for `/migration <sub>`
static MIGRATION_SUBS: &[&str] = &["status", "migrate", "rollback", "generate"];

/// Sub-commands for `/profiler <sub>`
static PROFILER_SUBS: &[&str] = &["run", "top", "list-tools"];

/// Sub-commands for `/env <sub>`
static ENV_SUBS: &[&str] = &["list", "get", "set", "delete", "switch", "files", "create"];

/// Sub-commands for `/deps <sub>`
static DEPS_SUBS: &[&str] = &["scan", "outdated", "vulnerable", "upgrade", "list"];

/// Sub-commands for `/deploy <sub>`
static DEPLOY_SUBS: &[&str] = &[
    "list", "vercel", "netlify", "railway", "github-pages",
    "gcp", "firebase", "aws", "aws-apprunner", "aws-s3", "aws-lambda", "aws-ecs",
    "azure", "azure-appservice", "azure-container", "azure-static",
    "digitalocean", "kubernetes", "helm", "oci", "ibm",
];

/// Sub-commands for `/compliance <sub>`
static COMPLIANCE_SUBS: &[&str] = &["soc2", "fedramp"];

/// Sub-commands for `/redteam <sub>`
static REDTEAM_SUBS: &[&str] = &["scan", "list", "show", "report", "config"];

/// Sub-commands for `/verify <sub>`
static VERIFY_SUBS: &[&str] = &["full", "quick", "security", "performance", "testing"];

/// Sub-commands for `/handoff <sub>`
static HANDOFF_SUBS: &[&str] = &["list", "show", "create"];

/// Built-in theme names for `/theme <name>` completion
static THEME_NAMES: &[&str] = &["dark", "light", "monokai", "solarized", "nord"];

// ── Hint strings ─────────────────────────────────────────────────────────────

/// Return an inline hint for a complete command word (after the user pressed space).
fn command_hint(cmd: &str) -> Option<&'static str> {
    match cmd {
        "/agent"   => Some("<task description>"),
        "/arena"   => Some("[compare <p1> <p2>|stats|history]  — blind A/B model comparison arena"),
        "/autofix" => Some("[clippy|eslint|ruff|gofmt|prettier]  — run linter auto-fix and show diff"),
        "/bisect"  => Some("[start <bad> <good>|good|bad|skip|reset|log|analyze]  — git bisect workflow"),
        "/plan"    => Some("<task description>"),
        "/chat"    => Some("<message>"),
        "/deps"    => Some("[scan|outdated|vulnerable|upgrade <pkg>|list]  — dependency management"),
        "/deploy"  => Some("[target|list]  — deploy to cloud (aws|azure|gcp|vercel|k8s|digitalocean|...)"),
        "/env"     => Some("[list|get <key>|set <key> <val>|delete <key>|switch <env>|files|create <env>]"),
        "/profiler"=> Some("[run [target]|top|list-tools]  — CPU/memory profiling"),
        "/generate"=> Some("<description>"),
        "/diff"    => Some("<file>"),
        "/apply"   => Some("<file> <changes>"),
        "/exec"    => Some("<task>"),
        "/qa"      => Some("<question about the codebase>"),
        "/index"   => Some("[embedding-model]"),
        "/resume"  => Some("[session-id] [task]"),
        "/profile" => Some("[list|show|create|delete]"),
        "/plugin"  => Some("[list|install|remove|info]"),
        "/memory"  => Some("[show|edit]"),
        "/openmemory" => Some("[add|query|list|fact|facts|decay|consolidate|stats|export|encrypt|context]  — cognitive memory engine"),
        "/vulnscan" => Some("[scan|deps|file|lockfile|sarif|report|db-update|db-status|cache-clear]  — vulnerability scanner"),
        "/resources" => Some("[status|export|verify|path]  — manage externalized resource files"),
        "/trace"   => Some("[view <id>]"),
        "/mcp"     => Some("[list|tools <server>]"),
        "/logs"    => Some("[tail <file>|sources|errors <file>|analyze <file>]  — log viewer & analyzer"),
        "/markers" => Some("[scan|list|bookmarks]  — scan TODO/FIXME/HACK markers & manage bookmarks"),
        "/mock"    => Some("[start <port>|stop|add|remove|list|log|import]  — API mock server (VibeUI)"),
        "/migration" => Some("[status|migrate|rollback|generate <name>]  — database migration management"),
        "/model"   => Some("<provider> [model]  — switch active model"),
        "/notebook" => Some("<file.vibe>  — run interactive notebook cells"),
        "/compliance" => Some("[soc2|fedramp]  — generate compliance report (SOC2/FedRAMP)"),
        "/cost"    => Some("— show token usage & estimated cost for this session"),
        "/context" => Some("— show active context window size"),
        "/status"  => Some("— show provider, model, session info"),
        "/fork"    => Some("[session-name]  — fork current session into a named branch"),
        "/rewind"   => Some("[list | <timestamp>]  — save or restore a conversation checkpoint"),
        "/spec"     => Some("[list|show <n>|new <n>|run <n>|done <n> <id>]  — spec-driven development"),
        "/agents"   => Some("[list|status|new <name> <task>]  — background agent definitions"),
        "/team"     => Some("[create <goal>|status|messages|show|knowledge|sync]  — agent teams & peer communication"),
        "/test"     => Some("[command]  — run project tests (auto-detects cargo/npm/pytest/go)"),
        "/theme"    => Some("[name]  — switch TUI color theme (dark|light|monokai|solarized|nord)"),
        "/snippet"  => Some("[list|save <name>|use <name>|show <name>|delete <name>]"),
        "/linear"   => Some("[list|new \"title\"|open <id>|attach <id>]  — Linear issue tracker"),
        "/remind"   => Some("in <dur> \"task\"  |  list  |  cancel <id>"),
        "/schedule" => Some("every <dur> \"task\"  |  list  |  cancel <id>"),
        "/jobs"     => Some("— list background agent jobs"),
        "/sessions" => Some("[<id_prefix>]  — list recent agent sessions from history (SQLite)"),
        "/share"    => Some("<session_id>  — print shareable URL for a session (requires vibecli serve)"),
        "/workflow" => Some("[new <name>|list|show <n>|advance <n>|check <n> <id>|generate <n>]  — Code Complete workflow"),
        "/redteam"  => Some("[scan <url>|list|show <id>|report <id>|config]  — autonomous security scanning"),
        "/voice"    => Some("[transcribe <file>|speak <text>]  — voice transcription (Whisper) & TTS (ElevenLabs)"),
        "/discover" => Some("— discover VibeCLI peers on the local network"),
        "/pair"     => Some("[host:port]  — generate device pairing URL with one-time token"),
        "/sandbox"  => Some("[status|start|stop|list|exec <cmd>|logs|runtime]  — container sandbox management"),
        "/verify"   => Some("[full|quick|security|performance|testing]  — structured verification checklist"),
        "/handoff"  => Some("[list|show <id>|create]  — session handoff documents"),
        "/orient"   => Some("— analyze current project structure and stack"),
        "/research" => Some("<topic>  — research a topic in context of the codebase"),
        "/bundle"   => Some("[create|activate|deactivate|list|share|import|export|delete]  — context bundles"),
        "/cloud"    => Some("[scan|iam|terraform|cloudformation|pulumi|cost|providers]  — cloud provider tools"),
        "/benchmark"=> Some("[run|compare|export|list]  — SWE-bench benchmarking"),
        "/metering" => Some("[status|budget|report|alerts|top]  — usage metering & credits"),
        "/blueteam" => Some("[status|scan|incidents|iocs|rules|forensics|playbooks|siem|hunt|report]  — defensive security"),
        "/purpleteam" => Some("[status|exercise|simulate|validate|matrix|gaps|heatmap|report]  — ATT&CK exercises"),
        "/idp" => Some("[status|catalog|register|golden|scorecard|infra|team|onboard|backstage|platforms|report]  — internal developer platform"),
        "/quantum" => Some("[languages|os|hardware|algorithms|circuits|projects|create|export|compat|status]  — quantum computing"),
        "/autoresearch" => Some("[new|start|stop|pause|status|list|analyze|export|suggest|lessons|config]  — autonomous iterative research agent"),
        _ => None,
    }
}

// ── VibeHelper ────────────────────────────────────────────────────────────────

pub struct VibeHelper {
    file_completer: FilenameCompleter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    highlighter: MatchingBracketHighlighter,
}

impl VibeHelper {
    pub fn new() -> Self {
        Self {
            file_completer: FilenameCompleter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
            highlighter: MatchingBracketHighlighter::new(),
        }
    }
}

// ── Completion logic ──────────────────────────────────────────────────────────

fn complete_slash(line: &str) -> Option<(usize, Vec<Pair>)> {
    if !line.starts_with('/') {
        return None;
    }

    // Split into: first token (the command) and optional rest.
    let mut iter = line.splitn(2, ' ');
    let first = iter.next().unwrap_or("");
    let rest = iter.next(); // None if no space yet

    match rest {
        // ── No space typed yet: complete the root command ──────────────────
        None => {
            let matches: Vec<Pair> = COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(first))
                .map(|cmd| {
                    // Append a space so the user is ready to type args.
                    let repl = format!("{} ", cmd);
                    Pair { display: cmd.to_string(), replacement: repl }
                })
                .collect();
            if matches.is_empty() {
                None
            } else {
                Some((0, matches))
            }
        }

        // ── Space typed: complete sub-commands or file paths ───────────────
        Some(after_space) => {
            let subs: Option<&[&str]> = match first {
                "/arena"    => Some(ARENA_SUBS),
                "/bisect"   => Some(BISECT_SUBS),
                "/deps"     => Some(DEPS_SUBS),
                "/deploy"   => Some(DEPLOY_SUBS),
                "/env"      => Some(ENV_SUBS),
                "/logs"     => Some(LOGS_SUBS),
                "/markers"  => Some(MARKERS_SUBS),
                "/mock"     => Some(MOCK_SUBS),
                "/profiler" => Some(PROFILER_SUBS),
                "/profile"  => Some(PROFILE_SUBS),
                "/plugin"   => Some(PLUGIN_SUBS),
                "/memory"   => Some(MEMORY_SUBS),
                "/openmemory" => Some(OPENMEMORY_SUBS),
                "/vulnscan" => Some(VULNSCAN_SUBS),
                "/resources" => Some(RESOURCES_SUBS),
                "/migration" => Some(MIGRATION_SUBS),
                "/spec"     => Some(SPEC_SUBS),
                "/agents"   => Some(AGENTS_SUBS),
                "/team"     => Some(TEAM_SUBS),
                "/trace"    => Some(TRACE_SUBS),
                "/mcp"      => Some(MCP_SUBS),
                "/theme"    => Some(THEME_NAMES),
                "/snippet"  => Some(SNIPPET_SUBS),
                "/linear"   => Some(LINEAR_SUBS),
                "/remind"   => Some(REMIND_SUBS),
                "/sandbox"  => Some(SANDBOX_SUBS),
                "/schedule" => Some(SCHEDULE_SUBS),
                "/workflow" => Some(WORKFLOW_SUBS),
                "/orchestrate" => Some(ORCHESTRATE_SUBS),
                "/redteam"  => Some(REDTEAM_SUBS),
                "/compliance" => Some(COMPLIANCE_SUBS),
                "/verify"   => Some(VERIFY_SUBS),
                "/handoff"  => Some(HANDOFF_SUBS),
                "/appbuilder" => Some(APPBUILDER_SUBS),
                "/icontext" => Some(ICONTEXT_SUBS),
                "/batch" => Some(BATCH_SUBS),
                "/qavalidate" => Some(QAVALIDATE_SUBS),
                "/legacymigrate" => Some(LEGACYMIGRATE_SUBS),
                "/gitplatform" => Some(GITPLATFORM_SUBS),
                "/bundle" => Some(BUNDLE_SUBS),
                "/cloud" => Some(CLOUD_SUBS),
                "/benchmark" => Some(BENCHMARK_SUBS),
                "/metering" => Some(METERING_SUBS),
                "/blueteam" => Some(BLUETEAM_SUBS),
                "/purpleteam" => Some(PURPLETEAM_SUBS),
                "/idp" => Some(IDP_SUBS),
                "/quantum" => Some(QUANTUM_SUBS),
                "/autoresearch" => Some(AUTORESEARCH_SUBS),
                _ => None,
            };

            if let Some(subs) = subs {
                // Only complete the immediate sub-command word (no deeper nesting).
                // If after_space has no space itself, complete sub-commands.
                if !after_space.contains(' ') {
                    let prefix = after_space;
                    let start = first.len() + 1; // position after "/<cmd> "
                    let matches: Vec<Pair> = subs
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| Pair { display: s.to_string(), replacement: s.to_string() })
                        .collect();
                    if !matches.is_empty() {
                        return Some((start, matches));
                    }
                }
            }

            // For commands that accept file paths, delegate to filename completion.
            // We'll signal this by returning None and letting the caller fall back.
            None
        }
    }
}

impl Completer for VibeHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        if let Some(result) = complete_slash(line) {
            return Ok(result);
        }
        // Fall back to filename completion (useful for /diff, /apply, /read paths)
        self.file_completer.complete(line, pos, ctx)
    }
}

// ── Hint ──────────────────────────────────────────────────────────────────────

impl Hinter for VibeHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        // Show inline argument hint when the user has typed a complete command + space.
        if line.starts_with('/') && line.ends_with(' ') {
            let cmd = line.trim_end();
            if let Some(hint_text) = command_hint(cmd) {
                return Some(hint_text.to_string());
            }
        }
        // Fall back to history-based hints.
        self.hinter.hint(line, pos, ctx)
    }
}

// ── Validator ─────────────────────────────────────────────────────────────────

impl Validator for VibeHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

// ── Highlighter ───────────────────────────────────────────────────────────────

impl Highlighter for VibeHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Render history/arg hints in dim grey.
        Cow::Owned("\x1b[2m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        // Colour slash commands cyan, everything else default.
        if line.starts_with('/') {
            let end = line.find(' ').unwrap_or(line.len());
            let cmd = &line[..end];
            if COMMANDS.contains(&cmd) {
                let coloured = format!("\x1b[36m{}\x1b[m{}", cmd, &line[end..]);
                return Cow::Owned(coloured);
            }
        }
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Helper for VibeHelper {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_command_completion() {
        let result = complete_slash("/pr");
        let (start, pairs) = result.unwrap();
        assert_eq!(start, 0);
        assert!(pairs.iter().any(|p| p.display == "/profile"));
        // /pr should not match /agent
        assert!(!pairs.iter().any(|p| p.display == "/agent"));
    }

    #[test]
    fn test_root_command_adds_trailing_space() {
        let result = complete_slash("/agent");
        let (_, pairs) = result.unwrap();
        assert!(pairs.iter().any(|p| p.replacement == "/agent "));
    }

    #[test]
    fn test_profile_subcommand_completion() {
        let result = complete_slash("/profile li");
        let (start, pairs) = result.unwrap();
        assert_eq!(start, "/profile ".len());
        assert!(pairs.iter().any(|p| p.display == "list"));
        assert!(!pairs.iter().any(|p| p.display == "delete")); // doesn't start with "li"
    }

    #[test]
    fn test_plugin_subcommand_completion() {
        let (_, pairs) = complete_slash("/plugin ").unwrap();
        assert_eq!(pairs.len(), PLUGIN_SUBS.len());
    }

    #[test]
    fn test_no_match_returns_none() {
        assert!(complete_slash("/zzz").unwrap_or((0, vec![])).1.is_empty());
    }

    #[test]
    fn test_non_slash_returns_none() {
        assert!(complete_slash("hello").is_none());
    }

    #[test]
    fn test_command_hint() {
        assert!(command_hint("/agent").is_some());
        assert!(command_hint("/profile").is_some());
        assert!(command_hint("/exit").is_none());
    }

    #[test]
    fn test_rewind_hint_is_present() {
        // /rewind must expose an inline hint
        let hint = command_hint("/rewind");
        assert!(hint.is_some(), "/rewind should have an argument hint");
        let hint_text = hint.unwrap();
        assert!(hint_text.contains("list") || hint_text.contains("timestamp"),
            "hint should describe list / timestamp usage, got: {hint_text}");
    }

    #[test]
    fn test_rewind_completion() {
        // "/rew" should complete to "/rewind"
        let result = complete_slash("/rew");
        let (start, pairs) = result.unwrap();
        assert_eq!(start, 0);
        assert!(pairs.iter().any(|p| p.display == "/rewind"),
            "/rew should complete to /rewind");
    }

    #[test]
    fn test_rewind_trailing_space() {
        // completing "/rewind" exactly should offer a trailing-space replacement
        let result = complete_slash("/rewind");
        let (_, pairs) = result.unwrap();
        assert!(pairs.iter().any(|p| p.replacement == "/rewind "),
            "completion for /rewind should add trailing space");
    }

    #[test]
    fn test_all_commands_in_list() {
        // Spot-check that every command we care about is discoverable
        for cmd in &["/agent", "/chat", "/rewind", "/fork", "/model", "/cost", "/mcp"] {
            let result = complete_slash(cmd);
            assert!(
                result.map(|(_, v)| v.iter().any(|p| &p.display == cmd)).unwrap_or(false),
                "{cmd} must be in COMMANDS and completable exactly"
            );
        }
    }

    // ── Sub-command completion for additional commands ──

    #[test]
    fn test_sandbox_subcommand_completion() {
        let (_, pairs) = complete_slash("/sandbox ").unwrap();
        assert_eq!(pairs.len(), SANDBOX_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "status"));
        assert!(pairs.iter().any(|p| p.display == "exec"));
    }

    #[test]
    fn test_workflow_subcommand_completion() {
        let (_, pairs) = complete_slash("/workflow ").unwrap();
        assert_eq!(pairs.len(), WORKFLOW_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "new"));
        assert!(pairs.iter().any(|p| p.display == "advance"));
    }

    #[test]
    fn test_redteam_subcommand_completion() {
        let (_, pairs) = complete_slash("/redteam ").unwrap();
        assert_eq!(pairs.len(), REDTEAM_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "scan"));
    }

    #[test]
    fn test_verify_subcommand_completion() {
        let (_, pairs) = complete_slash("/verify ").unwrap();
        assert_eq!(pairs.len(), VERIFY_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "full"));
        assert!(pairs.iter().any(|p| p.display == "security"));
    }

    #[test]
    fn test_handoff_subcommand_completion() {
        let (_, pairs) = complete_slash("/handoff ").unwrap();
        assert_eq!(pairs.len(), HANDOFF_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "create"));
    }

    #[test]
    fn test_compliance_subcommand_completion() {
        let (_, pairs) = complete_slash("/compliance ").unwrap();
        assert_eq!(pairs.len(), COMPLIANCE_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "soc2"));
        assert!(pairs.iter().any(|p| p.display == "fedramp"));
    }

    #[test]
    fn test_bisect_subcommand_completion() {
        let (_, pairs) = complete_slash("/bisect ").unwrap();
        assert_eq!(pairs.len(), BISECT_SUBS.len());
        assert!(pairs.iter().any(|p| p.display == "start"));
        assert!(pairs.iter().any(|p| p.display == "analyze"));
    }

    // ── Sub-command prefix filtering ──

    #[test]
    fn test_subcommand_prefix_filtering() {
        // "/deploy v" should only match "vercel"
        let result = complete_slash("/deploy v");
        let (_, pairs) = result.unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].display, "vercel");
    }

    #[test]
    fn test_subcommand_no_deeper_nesting() {
        // After the sub-command word is complete, no more completions
        let result = complete_slash("/profile list something");
        assert!(result.is_none());
    }

    // ── command_hint for additional commands ──

    #[test]
    fn test_command_hint_deploy() {
        let hint = command_hint("/deploy").unwrap();
        assert!(hint.contains("deploy") || hint.contains("target") || hint.contains("list"));
    }

    #[test]
    fn test_command_hint_sandbox() {
        let hint = command_hint("/sandbox").unwrap();
        assert!(hint.contains("sandbox") || hint.contains("status"));
    }

    #[test]
    fn test_command_hint_workflow() {
        let hint = command_hint("/workflow").unwrap();
        assert!(hint.contains("workflow") || hint.contains("new"));
    }

    // ── COMMANDS list completeness ──

    #[test]
    fn test_commands_list_contains_core_commands() {
        let core = &[
            "/agent", "/chat", "/exit", "/quit", "/help", "/model",
            "/plan", "/exec", "/diff", "/apply", "/status", "/config",
        ];
        for cmd in core {
            assert!(
                COMMANDS.contains(cmd),
                "COMMANDS list missing core command: {cmd}"
            );
        }
    }

    #[test]
    fn test_commands_list_contains_workflow_commands() {
        let workflow = &[
            "/workflow", "/orchestrate", "/spec", "/deploy", "/test", "/bisect",
            "/sandbox", "/arena", "/redteam", "/verify", "/handoff",
        ];
        for cmd in workflow {
            assert!(
                COMMANDS.contains(cmd),
                "COMMANDS list missing workflow command: {cmd}"
            );
        }
    }

    #[test]
    fn test_commands_list_contains_utility_commands() {
        let util = &[
            "/snippet", "/memory", "/trace", "/mcp", "/theme",
            "/remind", "/schedule", "/jobs", "/sessions", "/share",
        ];
        for cmd in util {
            assert!(
                COMMANDS.contains(cmd),
                "COMMANDS list missing utility command: {cmd}"
            );
        }
    }

    #[test]
    fn test_commands_all_start_with_slash() {
        for cmd in COMMANDS {
            assert!(
                cmd.starts_with('/'),
                "command {cmd} must start with '/'"
            );
        }
    }

    #[test]
    fn test_commands_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for cmd in COMMANDS {
            assert!(
                seen.insert(cmd),
                "duplicate command in COMMANDS list: {cmd}"
            );
        }
    }

    // ── Command parsing edge cases ──

    #[test]
    fn test_slash_alone() {
        // "/" should match all commands
        let result = complete_slash("/");
        let (start, pairs) = result.unwrap();
        assert_eq!(start, 0);
        assert_eq!(pairs.len(), COMMANDS.len());
    }

    #[test]
    fn test_empty_string_returns_none() {
        assert!(complete_slash("").is_none());
    }

    #[test]
    fn test_whitespace_only_returns_none() {
        assert!(complete_slash("   ").is_none());
    }

    #[test]
    fn test_subcommand_empty_prefix_returns_all_subs() {
        // "/team " with empty sub should list all team subs
        let (_, pairs) = complete_slash("/team ").unwrap();
        assert_eq!(pairs.len(), TEAM_SUBS.len());
        for sub in TEAM_SUBS {
            assert!(
                pairs.iter().any(|p| p.display == *sub),
                "missing team sub: {sub}"
            );
        }
    }

    #[test]
    fn test_subcommand_partial_match() {
        // "/team cr" should match only "create"
        let (_, pairs) = complete_slash("/team cr").unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].display, "create");
    }

    #[test]
    fn test_subcommand_no_match_returns_none() {
        // "/team zzz" has no matching sub-command
        let result = complete_slash("/team zzz");
        assert!(result.is_none());
    }

    #[test]
    fn test_command_without_subs_space_returns_none() {
        // "/exit " has no sub-commands, should return None
        let result = complete_slash("/exit ");
        assert!(result.is_none());
    }

    // ── Sub-command table coverage ──

    #[test]
    fn test_all_sub_tables_accessible_via_completion() {
        let cmds_with_subs: &[(&str, &[&str])] = &[
            ("/arena", ARENA_SUBS),
            ("/bisect", BISECT_SUBS),
            ("/deps", DEPS_SUBS),
            ("/deploy", DEPLOY_SUBS),
            ("/env", ENV_SUBS),
            ("/logs", LOGS_SUBS),
            ("/markers", MARKERS_SUBS),
            ("/mock", MOCK_SUBS),
            ("/profiler", PROFILER_SUBS),
            ("/profile", PROFILE_SUBS),
            ("/plugin", PLUGIN_SUBS),
            ("/memory", MEMORY_SUBS),
            ("/migration", MIGRATION_SUBS),
            ("/spec", SPEC_SUBS),
            ("/agents", AGENTS_SUBS),
            ("/team", TEAM_SUBS),
            ("/trace", TRACE_SUBS),
            ("/mcp", MCP_SUBS),
            ("/theme", THEME_NAMES),
            ("/snippet", SNIPPET_SUBS),
            ("/linear", LINEAR_SUBS),
            ("/remind", REMIND_SUBS),
            ("/sandbox", SANDBOX_SUBS),
            ("/schedule", SCHEDULE_SUBS),
            ("/workflow", WORKFLOW_SUBS),
            ("/orchestrate", ORCHESTRATE_SUBS),
            ("/redteam", REDTEAM_SUBS),
            ("/compliance", COMPLIANCE_SUBS),
            ("/verify", VERIFY_SUBS),
            ("/handoff", HANDOFF_SUBS),
            ("/appbuilder", APPBUILDER_SUBS),
            ("/icontext", ICONTEXT_SUBS),
            ("/batch", BATCH_SUBS),
            ("/qavalidate", QAVALIDATE_SUBS),
            ("/legacymigrate", LEGACYMIGRATE_SUBS),
            ("/gitplatform", GITPLATFORM_SUBS),
        ];
        for (cmd, subs) in cmds_with_subs {
            let input = format!("{} ", cmd);
            let result = complete_slash(&input);
            assert!(result.is_some(), "completion failed for '{input}'");
            let (_, pairs) = result.unwrap();
            assert_eq!(
                pairs.len(), subs.len(),
                "sub-command count mismatch for {cmd}: expected {}, got {}",
                subs.len(), pairs.len()
            );
        }
    }

    // ── command_hint coverage ──

    #[test]
    fn test_every_hinted_command_exists_in_commands_list() {
        // Every command that has a hint should exist in the COMMANDS list
        let hinted = &[
            "/agent", "/arena", "/autofix", "/bisect", "/plan", "/chat",
            "/deps", "/deploy", "/env", "/profiler", "/generate", "/diff",
            "/apply", "/exec", "/qa", "/index", "/resume", "/profile",
            "/plugin", "/memory", "/trace", "/mcp", "/logs", "/markers",
            "/mock", "/migration", "/model", "/notebook", "/compliance",
            "/cost", "/context", "/status", "/fork", "/rewind", "/spec",
            "/agents", "/team", "/test", "/theme", "/snippet", "/linear",
            "/remind", "/schedule", "/jobs", "/sessions", "/share",
            "/workflow", "/redteam", "/voice", "/discover", "/pair",
            "/sandbox", "/verify", "/handoff", "/orient", "/research",
        ];
        for cmd in hinted {
            assert!(command_hint(cmd).is_some(), "{cmd} should have a hint");
            assert!(
                COMMANDS.contains(cmd),
                "hinted command {cmd} missing from COMMANDS"
            );
        }
    }

    #[test]
    fn test_command_hint_returns_none_for_unknown() {
        assert!(command_hint("/nonexistent").is_none());
        assert!(command_hint("agent").is_none()); // no leading slash
        assert!(command_hint("").is_none());
    }

    #[test]
    fn test_command_hint_exit_and_quit_have_no_hint() {
        // /exit and /quit take no arguments, so no hint
        assert!(command_hint("/exit").is_none());
        assert!(command_hint("/quit").is_none());
    }

    // ── Completion position (start offset) ──

    #[test]
    fn test_root_completion_starts_at_zero() {
        let (start, _) = complete_slash("/a").unwrap();
        assert_eq!(start, 0);
    }

    #[test]
    fn test_sub_completion_start_offset_correct() {
        // For "/deploy v", start should be len("/deploy ") = 8
        let (start, _) = complete_slash("/deploy v").unwrap();
        assert_eq!(start, "/deploy ".len());
    }

    // ── Highlighter logic (unit-testable parts) ──

    #[test]
    fn test_highlight_known_command_produces_cyan() {
        let helper = VibeHelper::new();
        let line = "/agent do something";
        let result = helper.highlight(line, 0);
        // Should contain the ANSI cyan code \x1b[36m
        assert!(result.contains("\x1b[36m"), "known command should be coloured cyan");
        assert!(result.contains("/agent"), "should contain the command text");
        assert!(result.contains("do something"), "should contain rest of text");
    }

    #[test]
    fn test_highlight_unknown_slash_no_cyan() {
        let helper = VibeHelper::new();
        let line = "/zzz whatever";
        let result = helper.highlight(line, 0);
        // Unknown commands should NOT get cyan colouring by our logic
        assert!(!result.contains("\x1b[36m"), "unknown command should not be cyan");
    }

    #[test]
    fn test_highlight_hint_renders_dim() {
        let helper = VibeHelper::new();
        let result = helper.highlight_hint("some hint text");
        assert!(result.contains("\x1b[2m"), "hint should start with dim ANSI");
        assert!(result.contains("\x1b[m"), "hint should end with ANSI reset");
        assert!(result.contains("some hint text"), "hint text should be present");
    }
}
