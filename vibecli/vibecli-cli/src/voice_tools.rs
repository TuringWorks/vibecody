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
///
/// `<list_dir` is a prefix of `<list_directory` too, so both spellings are
/// gated even though only the canonical one is ever advertised.
const TOOL_PREFIXES: &[&str] = &[
    "<tool_call",
    "<read_file",
    "<list_dir",
    "<search_files",
    "<write_file",
    "<apply_patch",
    "<open_file",
];

/// The line that closes a tool round: the model has its results, now speak.
///
/// Shared because a round can end in either half — files opened for the client,
/// tools run against disk, or both — and all three have to hand back the same
/// instruction or the turn ends on a look with nothing said.
pub const ANSWER_NOW: &str = "Now answer the user in one or two short spoken sentences.";

/// What the assistant is told it may do, when a workspace root is known.
///
/// Deliberately short: this rides on every turn's system prompt, including the
/// ones that are just conversation. The write half is only included when the
/// host asked for it — and even then a write does not happen until the user
/// says yes, which the prompt says plainly so the assistant does not promise
/// something it cannot deliver.
///
/// `may_open` is a fact about the *client*, not about permission: VibeDesk and
/// VibeAIChat have no editor to open a file in, so advertising the tool there
/// would buy a promise nobody can keep. It is declared over the socket by the
/// clients that can, and the line is omitted for the ones that cannot.
pub fn contract(may_change: bool, may_open: bool) -> String {
    let base = change_contract(may_change);
    match may_open {
        true => format!("{base}{OPEN_CLAUSE}"),
        false => base.to_string(),
    }
}

/// Showing a file is not reading one, and models reach for `read_file` for
/// both. Asked to "open the config", one reads it and describes it — which is
/// an answer to a question the user did not ask, with their editor still
/// showing whatever it showed before.
const OPEN_CLAUSE: &str = "\n\nThe user has an editor open in front of them, so you can also put a \
     file on their screen:\n\
     <tool_call name=\"open_file\"><path>src/main.rs</path></tool_call>\n\
     Opening is not reading. When they ask you to open, show, pull up or bring up a file, \
     call open_file — reading it to yourself and then describing it is not what they asked \
     for. read_file is for when *you* need the contents in order to answer. Only say a file \
     is open after you have been told it was.";

fn change_contract(may_change: bool) -> &'static str {
    if may_change {
        concat!(
            "\n\nYou can look at the project, and you can change it — but a change is asked \
             for out loud and the user has to agree before it happens. To use a tool, reply \
             with ONE tool call and nothing else; it is not read aloud:\n\
             <tool_call name=\"read_file\"><path>README.md</path></tool_call>\n\
             <tool_call name=\"list_directory\"><path>src</path></tool_call>\n\
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
     <tool_call name=\"list_directory\"><path>src</path></tool_call>\n\
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

/// The paths a turn asked the *client* to show, in the order it asked.
///
/// Deliberately not a [`ToolCall`]: every variant of that enum is something
/// [`crate::tool_executor::ToolExecutor`] can run against the filesystem, and
/// opening an editor tab is not — there is no file operation to perform. It is
/// parsed here and dispatched over the socket to whoever owns a window, so the
/// executor's contract stays "things this process can do".
///
/// Both dialects models actually emit are accepted: the canonical
/// `<tool_call name="open_file">` the contract teaches, and the bare
/// `<open_file>` element they fall into when copying the shape of the tag
/// around it.
pub fn parse_open_file(text: &str) -> Vec<String> {
    open_file_re()
        .captures_iter(text)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Compiled once. A `Regex::new` on a per-turn path is this codebase's most
/// expensive recurring mistake — `strip_thinking` cost 27 MB per streamed chunk
/// before its patterns were hoisted.
fn open_file_re() -> &'static regex::Regex {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r#"(?s)<(?:tool_call\s+name="open_file"\s*|open_file\s*)>\s*<path>(.*?)</path>"#,
        )
        .expect("open_file pattern is a literal and is covered by unit tests")
    });
    &RE
}

/// Trim a tool result to something an answer can be built from.
pub fn clamp_result(s: &str) -> String {
    match s.char_indices().nth(MAX_RESULT_CHARS) {
        Some((byte, _)) => format!("{}\n…(truncated)", &s[..byte]),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod pipeline_tests {
    /// The pipeline the daemon actually runs between the provider and the
    /// speaker: reasoning filter, then tool gate.
    ///
    /// It has to be tested as a pipeline, because the defect lived in neither
    /// half. `StreamFilter` suppresses `<tool_call>` — correctly, for the agent
    /// console, which renders tool lines separately — and it runs *before* the
    /// gate. So a model following the contract in [`contract`] had its call
    /// eaten before `ToolGate` could see it: `tool_text()` was `None`, no tool
    /// ever ran, and the turn ended on "the model produced only reasoning or
    /// tool calls and never answered".
    fn run(chunks: &[&str]) -> (String, Option<String>) {
        let mut filter = crate::agent_stream_filter::StreamFilter::reasoning_only();
        let mut gate = super::ToolGate::default();
        let mut spoken = String::new();
        for c in chunks {
            let t = filter.push(c);
            if t.is_empty() {
                continue;
            }
            spoken.push_str(&gate.push(&t));
        }
        let tail = filter.finish();
        spoken.push_str(&gate.push(&tail));
        spoken.push_str(&gate.finish());
        (spoken, gate.tool_text().map(str::to_string))
    }

    #[test]
    fn a_tool_call_survives_the_reasoning_filter_and_reaches_the_gate() {
        let (spoken, tool) = run(&[
            "<tool_call name=\"list_directory\">",
            "<path>examples</path>",
            "</tool_call>",
        ]);
        assert_eq!(spoken, "", "a tool call is never read aloud");
        let tool = tool.expect("the gate must see the call the contract asked for");
        assert!(
            !vibe_ai::tools::parse_tool_calls(&tool).is_empty(),
            "the gate's text must still parse as a call: {tool:?}"
        );
    }

    #[test]
    fn reasoning_before_a_tool_call_is_still_dropped() {
        // Both halves at once — the reason the filter cannot simply be removed.
        let (spoken, tool) = run(&[
            "<thinking>",
            "The user wants the examples folder. I should list it.",
            "</thinking>\n",
            "<tool_call name=\"list_directory\"><path>examples</path></tool_call>",
        ]);
        assert_eq!(spoken, "");
        let tool = tool.expect("the call after the reasoning block must survive");
        assert!(!tool.contains("The user wants"), "reasoning leaked: {tool:?}");
        assert_eq!(
            vibe_ai::tools::parse_tool_calls(&tool).len(),
            1,
            "expected exactly one call from {tool:?}"
        );
    }

    /// Every call the contract shows the model must actually parse.
    ///
    /// The contract advertised `list_dir`; `parse_tool_calls` knows
    /// `list_directory` and nothing else. So a model that did exactly what it
    /// was told produced a call that parsed to *zero* calls, the tool branch
    /// was skipped, the turn ended with nothing spoken, and the user was told
    /// the model "never answered" — for following the instructions.
    ///
    /// A prompt is an interface. This is its conformance test: the examples in
    /// the prompt are the specification, and they are checked against the
    /// implementation that has to honour them.
    #[test]
    fn every_example_in_the_contract_parses_as_a_call() {
        for (label, text) in [
            ("read-only", super::read_only_contract().to_string()),
            ("read-write", super::contract(true, false)),
            // The `open_file` clause is part of a shipped contract too, on the
            // clients that have an editor — so it is held to the same rule.
            ("read-write + open", super::contract(true, true)),
        ] {
            let text = text.as_str();
            let examples: Vec<&str> = text
                .match_indices("<tool_call")
                .filter_map(|(start, _)| {
                    text[start..]
                        .find("</tool_call>")
                        .map(|end| &text[start..start + end + "</tool_call>".len()])
                })
                .collect();
            assert!(
                examples.len() >= 3,
                "{label}: found {} examples — the scan is not reading the contract",
                examples.len()
            );
            for ex in examples {
                // `open_file` is answered by the client, not the executor, so
                // it is parsed here rather than by `parse_tool_calls` — but the
                // rule it is being held to is the same one: the example in the
                // prompt must reach the code that acts on it.
                let parsed = match ex.contains("open_file") {
                    true => super::parse_open_file(ex).len(),
                    false => vibe_ai::tools::parse_tool_calls(ex).len(),
                };
                assert_eq!(
                    parsed, 1,
                    "{label}: the contract shows the model a call the parser does not \
                     accept, so following it produces nothing: {ex:?}"
                );
            }
        }
    }

    #[test]
    fn an_ordinary_answer_is_still_spoken_and_is_not_a_tool_turn() {
        let (spoken, tool) = run(&[
            "<thinking>short</thinking>",
            "There are three files in the examples folder.",
        ]);
        assert_eq!(spoken, "There are three files in the examples folder.");
        assert!(tool.is_none());
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

    /// The canonical dialect the contract teaches.
    #[test]
    fn an_open_call_yields_its_path() {
        assert_eq!(
            parse_open_file("<tool_call name=\"open_file\"><path>src/main.rs</path></tool_call>"),
            vec!["src/main.rs".to_string()]
        );
    }

    /// The one models fall into by copying the shape of the surrounding tag.
    /// Accepting it costs a branch; rejecting it costs the user a file that
    /// does not open and an assistant that says it did.
    #[test]
    fn the_element_dialect_is_accepted_too() {
        assert_eq!(
            parse_open_file("<open_file><path>docs/README.md</path></open_file>"),
            vec!["docs/README.md".to_string()]
        );
    }

    /// Reading is not opening, in this direction as well: a `read_file` turn
    /// must not put a tab on screen the user did not ask for.
    #[test]
    fn a_read_is_not_an_open() {
        assert!(parse_open_file(
            "<tool_call name=\"read_file\"><path>src/main.rs</path></tool_call>"
        )
        .is_empty());
    }

    /// Two files in one turn are both honoured, in order — the cap on how many
    /// are acted upon belongs to the caller, not the parser.
    #[test]
    fn several_opens_keep_their_order() {
        let calls = "<tool_call name=\"open_file\"><path>a.rs</path></tool_call>\n\
                     <tool_call name=\"open_file\"><path>b.rs</path></tool_call>";
        assert_eq!(parse_open_file(calls), vec!["a.rs", "b.rs"]);
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
