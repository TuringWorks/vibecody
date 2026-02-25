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
    "/apply",
    "/chat",
    "/config",
    "/cost",
    "/context",
    "/diff",
    "/exec",
    "/exit",
    "/fork",
    "/generate",
    "/help",
    "/index",
    "/memory",
    "/mcp",
    "/model",
    "/plan",
    "/plugin",
    "/profile",
    "/qa",
    "/quit",
    "/resume",
    "/rewind",
    "/status",
    "/trace",
];

// ── Sub-command tables ────────────────────────────────────────────────────────

/// Sub-commands for `/profile <sub>`
static PROFILE_SUBS: &[&str] = &["list", "show", "create", "delete"];

/// Sub-commands for `/plugin <sub>`
static PLUGIN_SUBS: &[&str] = &["list", "install", "remove", "info"];

/// Sub-commands for `/memory <sub>`
static MEMORY_SUBS: &[&str] = &["show", "edit"];

/// Sub-commands for `/trace <sub>`
static TRACE_SUBS: &[&str] = &["view"];

/// Sub-commands for `/mcp <sub>`
static MCP_SUBS: &[&str] = &["list", "tools"];

// ── Hint strings ─────────────────────────────────────────────────────────────

/// Return an inline hint for a complete command word (after the user pressed space).
fn command_hint(cmd: &str) -> Option<&'static str> {
    match cmd {
        "/agent"   => Some("<task description>"),
        "/plan"    => Some("<task description>"),
        "/chat"    => Some("<message>"),
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
        "/model"   => Some("<provider> [model]  — switch active model"),
        "/cost"    => Some("— show token usage & estimated cost for this session"),
        "/context" => Some("— show active context window size"),
        "/status"  => Some("— show provider, model, session info"),
        "/fork"    => Some("[session-name]  — fork current session into a named branch"),
        "/rewind"  => Some("[list | <timestamp>]  — save or restore a conversation checkpoint"),
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
                "/profile" => Some(PROFILE_SUBS),
                "/plugin"  => Some(PLUGIN_SUBS),
                "/memory"  => Some(MEMORY_SUBS),
                "/trace"   => Some(TRACE_SUBS),
                "/mcp"     => Some(MCP_SUBS),
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
