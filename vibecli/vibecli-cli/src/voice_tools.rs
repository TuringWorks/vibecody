//! Tools for a spoken turn — reading the project the user is asking about.
//!
//! The typed path answers "summarise this project" by reading `README.md`; the
//! voice path had no way to open a file, so the same question got "just a
//! collection of directories and files". Context alone cannot close that gap:
//! whatever the client preloads is a guess about what will be asked.
//!
//! What voice needs from a tool loop is not what an agent needs. An agent may
//! take twenty steps and report at the end; here every step is silence in a
//! conversation, so the loop is capped hard and the tools are the cheap ones.
//! Execution itself is [`crate::tool_executor::ToolExecutor`] — the same
//! path-guarded, workspace-jailed executor the agent uses. Nothing here opens a
//! file by itself.

use std::sync::Arc;

use vibe_ai::tools::ToolCall;

/// Where a client's answer to "may I change this?" arrives.
///
/// One slot rather than a queue: the assistant asks and then waits, so a
/// second question cannot exist until the first is answered or abandoned.
pub type ApprovalSlot = Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>;

/// How long the turn waits for an answer before treating silence as "no".
///
/// Long enough to read the question and reach for the mouse, short enough that
/// a user who walked away is not leaving a microphone open on a pending write.
pub const APPROVAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// Tool rounds per spoken turn. Two is enough to list a directory and read the
/// file it named; beyond that the user is listening to nothing happen.
pub const MAX_ROUNDS: usize = 2;

/// Calls honoured in one round. A model that asks for six files at once is
/// answered with the first two and told so.
pub const MAX_CALLS_PER_ROUND: usize = 2;

/// Per-result cap, in characters. The result is fed back into a prompt whose
/// answer must fit in two spoken sentences.
pub const MAX_RESULT_CHARS: usize = 4_000;

/// The head of a stream is enough to tell a tool call from an answer, because
/// the contract says a tool turn contains the call and nothing else.
const GATE_CHARS: usize = 24;

/// Openings that mean "this turn is a tool call, do not speak it".
const TOOL_PREFIXES: &[&str] =
    &["<tool_call", "<read_file", "<list_dir", "<search_files", "<write_file", "<apply_patch"];

/// What the assistant is told it may do, when a workspace root is known.
///
/// Deliberately short: this rides on every turn's system prompt, including the
/// ones that are just conversation. The write half is only included when the
/// host asked for it — and even then a write does not happen until the user
/// says yes, which the prompt says plainly so the assistant does not promise
/// something it cannot deliver.
pub fn contract(may_change: bool) -> &'static str {
    if may_change {
        concat!(
            "\n\nYou can look at the project, and you can change it — but a change is asked \
             for out loud and the user has to agree before it happens. To use a tool, reply \
             with ONE tool call and nothing else; it is not read aloud:\n\
             <tool_call name=\"read_file\"><path>README.md</path></tool_call>\n\
             <tool_call name=\"list_dir\"><path>src</path></tool_call>\n\
             <tool_call name=\"search_files\"><query>fn main</query></tool_call>\n\
             <tool_call name=\"write_file\"><path>src/main.rs</path><content>…</content></tool_call>\n\
             Paths are relative to the project root. Read a file before rewriting it — \
             write_file replaces the whole file, so anything you did not include is gone. \
             You will be told whether the user agreed. If the workspace block above already \
             answers, answer from it — every look is a pause the user hears. If it does not, \
             look rather than saying you cannot tell: a question about what the project *is* \
             is usually answered by its README. Never claim to have read or changed a file \
             you did not."
        )
    } else {
        read_only_contract()
    }
}

/// The read-only half, for a turn that may not change anything.
pub fn read_only_contract() -> &'static str {
    "\n\nYou can look at the project before answering. To do that, reply with \
     ONE tool call and nothing else — no speech around it, since it is not read \
     aloud:\n\
     <tool_call name=\"read_file\"><path>README.md</path></tool_call>\n\
     <tool_call name=\"list_dir\"><path>src</path></tool_call>\n\
     <tool_call name=\"search_files\"><query>fn main</query></tool_call>\n\
     Paths are relative to the project root. You will be given the result and \
     can then answer. If the workspace block above already answers, answer from it — \
     every look is a pause the user hears. If it does not, look rather than saying \
     you cannot tell: a question about what the project *is* is usually answered by \
     its README. Never claim to have read a file you did not."
}

/// Whether a turn is a tool call, decided from its first characters.
///
/// Speech starts at the first sentence boundary, which is far beyond
/// [`GATE_CHARS`], so holding the head back costs nothing audible — and gets us
/// out of the alternative, which is speaking half a `<tool_call>` aloud before
/// noticing what it was.
#[derive(Debug, Default)]
pub struct ToolGate {
    head: String,
    decided: Option<bool>,
}

impl ToolGate {
    /// Feed a token. Returns the text to speak — empty while undecided, and
    /// empty forever once this turn is known to be a tool call.
    pub fn push(&mut self, tok: &str) -> String {
        match self.decided {
            Some(true) => {
                self.head.push_str(tok);
                String::new()
            }
            Some(false) => tok.to_string(),
            None => {
                self.head.push_str(tok);
                if self.head.trim_start().len() < GATE_CHARS && !looks_like_tool(&self.head) {
                    return String::new();
                }
                let is_tool = looks_like_tool(&self.head);
                self.decided = Some(is_tool);
                if is_tool {
                    String::new()
                } else {
                    std::mem::take(&mut self.head)
                }
            }
        }
    }

    /// End of stream: whatever is still held back. A turn shorter than the
    /// gate window never decided, and short answers ("Yes.") are common.
    pub fn finish(&mut self) -> String {
        match self.decided {
            Some(true) => String::new(),
            _ => {
                self.decided = Some(false);
                std::mem::take(&mut self.head)
            }
        }
    }

    /// The raw text of a tool turn, for parsing. Empty unless this turn was
    /// decided to be one.
    pub fn tool_text(&self) -> Option<&str> {
        (self.decided == Some(true)).then_some(self.head.as_str())
    }
}

fn looks_like_tool(head: &str) -> bool {
    let h = head.trim_start();
    TOOL_PREFIXES.iter().any(|p| h.starts_with(p))
}

/// Whether a call only reads. Reads run unattended; everything else has to ask.
pub fn is_read_only(call: &ToolCall) -> bool {
    matches!(
        call,
        ToolCall::ReadFile { .. } | ToolCall::ListDirectory { .. } | ToolCall::SearchFiles { .. }
    )
}

/// Whether a call may run at all in a spoken turn, given approval.
///
/// Writing a file is a change the user can see and undo in their editor.
/// Running a command is not — a spoken "yes" to `rm -rf` is the same word as a
/// spoken "yes" to a formatter, and the user cannot read the command from a
/// speaker. Shell stays out of voice until there is a surface that shows
/// exactly what would run.
pub fn is_permitted(call: &ToolCall) -> bool {
    is_read_only(call) || matches!(call, ToolCall::WriteFile { .. } | ToolCall::ApplyPatch { .. })
}

/// The question the user is asked, out loud and on screen. Says what will
/// change and where — "may I do that" with no object is not consent.
pub fn approval_question(call: &ToolCall) -> String {
    match call {
        ToolCall::WriteFile { path, content } => {
            let lines = content.lines().count();
            format!("May I write {path}? It replaces the file with {lines} lines.")
        }
        ToolCall::ApplyPatch { path, .. } => format!("May I patch {path}?"),
        other => format!("May I run {}?", describe(other)),
    }
}

/// How a call is described to the client, so a caption can say what is
/// happening during the pause: "Reading README.md".
pub fn describe(call: &ToolCall) -> String {
    match call {
        ToolCall::ReadFile { path } => format!("Reading {path}"),
        ToolCall::ListDirectory { path } => format!("Listing {path}"),
        ToolCall::SearchFiles { query, .. } => format!("Searching for {query}"),
        other => format!("{other:?}"),
    }
}

/// Trim a tool result to something an answer can be built from.
pub fn clamp_result(s: &str) -> String {
    match s.char_indices().nth(MAX_RESULT_CHARS) {
        Some((byte, _)) => format!("{}\n…(truncated)", &s[..byte]),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the gate one token at a time, as the stream does.
    fn spoken(tokens: &[&str]) -> String {
        let mut gate = ToolGate::default();
        let mut out = String::new();
        for t in tokens {
            out.push_str(&gate.push(t));
        }
        out.push_str(&gate.finish());
        out
    }

    #[test]
    fn an_ordinary_answer_is_spoken_whole() {
        assert_eq!(
            spoken(&["It is a ", "second brain for ", "notes and links."]),
            "It is a second brain for notes and links."
        );
    }

    /// The failure this exists to prevent: half a tag read aloud.
    #[test]
    fn a_tool_call_is_never_spoken() {
        assert_eq!(
            spoken(&["<tool_", "call name=\"read_file\"><path>README.md</path>", "</tool_call>"]),
            ""
        );
    }

    #[test]
    fn a_tool_turn_keeps_its_text_for_parsing() {
        let mut gate = ToolGate::default();
        for t in ["<tool_call name=\"read_file\">", "<path>README.md</path></tool_call>"] {
            assert_eq!(gate.push(t), "");
        }
        gate.finish();
        assert!(gate.tool_text().unwrap().contains("README.md"));
    }

    /// A short reply never reaches the gate window, and must still be spoken.
    #[test]
    fn a_reply_shorter_than_the_gate_window_still_speaks() {
        assert_eq!(spoken(&["Yes."]), "Yes.");
        assert_eq!(spoken(&["Two.", ""]), "Two.");
    }

    #[test]
    fn an_answer_that_merely_mentions_a_tag_is_not_a_tool_call() {
        let text = "You would write a read_file call for that.";
        assert_eq!(spoken(&[text]), text);
    }

    #[test]
    fn reads_run_unattended_and_writes_do_not() {
        assert!(is_read_only(&ToolCall::ReadFile { path: "a".into() }));
        assert!(is_read_only(&ToolCall::ListDirectory { path: ".".into() }));
        assert!(!is_read_only(&ToolCall::WriteFile { path: "a".into(), content: "x".into() }));
    }

    /// A spoken "yes" to `rm -rf` is the same word as a spoken "yes" to a
    /// formatter, and the user cannot read the command off a speaker. Shell
    /// stays out of voice entirely — approval is not enough for it.
    #[test]
    fn shell_is_not_reachable_by_voice_even_with_approval() {
        assert!(!is_permitted(&ToolCall::Bash { command: "ls".into() }));
        assert!(is_permitted(&ToolCall::WriteFile { path: "a".into(), content: "x".into() }));
        assert!(is_permitted(&ToolCall::ReadFile { path: "a".into() }));
    }

    /// "May I do that?" is not consent — the question has to name the file and
    /// say what happens to it, because the user is answering it from memory of
    /// what they asked for several seconds ago.
    #[test]
    fn the_question_names_the_file_and_the_damage() {
        let q = approval_question(&ToolCall::WriteFile {
            path: "src/main.rs".into(),
            content: "a\nb\nc".into(),
        });
        assert!(q.contains("src/main.rs"), "{q}");
        assert!(q.contains("replaces"), "{q}");
        assert!(q.contains('3'), "{q}");
    }

    /// A write emitted in the element dialect is still a tool call, and still
    /// must not be read aloud — `<write_file path="…">` spoken as prose is the
    /// same failure as a spoken `<tool_call>`, with a file changed at the end.
    #[test]
    fn a_write_in_element_form_is_gated_too() {
        assert_eq!(spoken(&["<write_file path=\"a.rs\">", "x</write_file>"]), "");
    }

    #[test]
    fn results_are_bounded_and_say_so() {
        let big = "x".repeat(MAX_RESULT_CHARS * 2);
        let out = clamp_result(&big);
        assert!(out.contains("(truncated)"));
        assert!(out.len() < big.len());
        assert_eq!(clamp_result("short"), "short");
    }
}
