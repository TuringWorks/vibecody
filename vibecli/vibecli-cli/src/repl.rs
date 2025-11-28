use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator, ValidationContext, ValidationResult};
use rustyline::{Context, Helper};
use std::borrow::Cow;

pub struct VibeHelper {
    completer: FilenameCompleter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    highlighter: MatchingBracketHighlighter,
}

impl VibeHelper {
    pub fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
            highlighter: MatchingBracketHighlighter::new(),
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
        // Custom completion for slash commands
        if line.starts_with('/') {
            let commands = vec![
                "/chat",
                "/generate",
                "/diff",
                "/apply",
                "/exec",
                "/config",
                "/help",
                "/exit",
            ];
            
            let mut matches = Vec::new();
            for cmd in commands {
                if cmd.starts_with(line) {
                    matches.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
            
            if !matches.is_empty() {
                return Ok((0, matches));
            }
        }
        
        // Fallback to filename completion
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for VibeHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Validator for VibeHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl Highlighter for VibeHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Helper for VibeHelper {}
