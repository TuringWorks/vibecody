use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator, ValidationContext, ValidationResult};
use rustyline::{Context, Helper};
use std::borrow::Cow;

// ── All known top-level slash commands ────────────────────────────────────────

static COMMANDS: &[&str] = &[
    "/agent",
    "/agents",
    "/arena",
    "/apply",
    "/autofix",
    "/bisect",
    "/chat",
    "/compliance",
    "/config",
    "/cost",
    "/context",
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

/// Sub-commands for `/sandbox <sub>`
static SANDBOX_SUBS: &[&str] = &["status", "start", "stop", "list", "exec", "logs", "runtime"];

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
                "/redteam"  => Some(REDTEAM_SUBS),
                "/compliance" => Some(COMPLIANCE_SUBS),
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
}
