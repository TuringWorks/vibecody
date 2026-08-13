//! Streaming filter for agent display text.
//!
//! The agent streams raw provider text to the console. That text carries two
//! things the user must never see: reasoning blocks (`<thinking>`, `<think>`,
//! namespaced `<mm:think>`) and tool-call markup (`<tool_call name="…">…`),
//! which the renderer prints separately as a structured tool line.
//!
//! [`vibe_ai::tools::strip_thinking`] already does this — but only for a
//! *complete* response. The REPL prints chunk by chunk, and a tag routinely
//! straddles a chunk boundary (`<thin` + `king>`), so a per-chunk strip both
//! misses the tag and leaks its two halves. Hence a state machine that carries
//! its partial state across calls.
//!
//! Contract: feed every chunk to [`StreamFilter::push`] and print what it
//! returns; call [`StreamFilter::finish`] at end of turn to flush anything held
//! back. Held-back text is never silently dropped — an unclosed reasoning block
//! is discarded (it *is* reasoning), but a lone `<` that never became a tag is
//! emitted verbatim.

use regex::Regex;
use std::sync::OnceLock;

/// Elements whose entire contents are protocol, never user-facing. Compared on
/// the local name, so `<mm:think>` matches `think`.
const SUPPRESSED: &[&str] = &["think", "thinking", "tool_call"];

/// `<`, an optional `/`, a possibly-namespaced name, optional attributes, `>`.
fn tag_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?s)^<(/?)([A-Za-z][\w.-]*(?::[A-Za-z][\w.-]*)?)[^>]*>")
            .expect("hardcoded regex is valid")
    })
}

/// A `<` that could still grow into a tag: no `>` has arrived yet, and what
/// follows is either nothing (`<`, `</` — the chunk ended exactly there) or a
/// started tag name. `< b` is excluded: a space cannot begin a name, so it is
/// prose and is emitted immediately rather than stalling the stream.
fn partial_tag_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?s)^</?(?:[A-Za-z][\w.:-]*[^>]*)?$").expect("hardcoded regex is valid")
    })
}

/// Local name of a possibly-namespaced tag: `mm:think` → `think`.
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// The closing-tag regex for a suppressed element, compiled once.
///
/// This used to `format!` a pattern and call `Regex::new` on **every call**,
/// and the caller runs once per streamed chunk for as long as the model stays
/// inside a reasoning block — so a long `<thinking>` block compiled a fresh
/// regex for every chunk of its duration. `tag` is always the local name of a
/// [`SUPPRESSED`] element (see `drain`), so three regexes cover every call.
///
/// Matching is case-insensitive, which [`is_suppressed`] already is for the
/// *opening* tag. The old per-call regex was built from the observed tag and
/// so was case-*sensitive* on close: a model emitting `<Think>` … `</think>`
/// never closed the block, and the whole tail was dropped on flush. Being
/// consistent with the open tag fixes that rather than reproducing it.
fn close_re(tag: &str) -> Option<&'static Regex> {
    static RES: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    let res = RES.get_or_init(|| {
        SUPPRESSED
            .iter()
            .filter_map(|name| {
                Regex::new(&format!(
                    r"(?si)</(?:[A-Za-z][\w.-]*:)?{}\s*>",
                    regex::escape(name)
                ))
                .ok()
                .map(|re| (*name, re))
            })
            .collect()
    });
    res.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(tag))
        .map(|(_, re)| re)
}

fn is_suppressed(name: &str) -> bool {
    let local = local_name(name);
    SUPPRESSED.iter().any(|s| s.eq_ignore_ascii_case(local))
}

#[derive(Debug, Default)]
pub struct StreamFilter {
    /// Text received but not yet classified as emittable or suppressed.
    buf: String,
    /// Local tag name of the suppressed element currently being consumed.
    inside: Option<String>,
}

impl StreamFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one streamed chunk. Returns the text safe to print now, which may
    /// be empty while a tag or a suppressed block is still being resolved.
    pub fn push(&mut self, chunk: &str) -> String {
        self.buf.push_str(chunk);
        self.drain(false)
    }

    /// End of turn: flush everything still held. Discards an unterminated
    /// reasoning block, emits an unterminated non-tag `<`.
    ///
    /// Also clears suppression state, so the filter is reusable. A turn that
    /// ended inside `<thinking>` must not leave the next turn suppressed —
    /// that would silently swallow a whole run's output.
    pub fn finish(&mut self) -> String {
        let out = self.drain(true);
        self.inside = None;
        self.buf.clear();
        out
    }

    fn drain(&mut self, flush: bool) -> String {
        let mut out = String::new();
        loop {
            // `take` rather than `clone`: this loop runs per streamed chunk and
            // the clone allocated a `String` on every iteration purely to end
            // the borrow. The tag is put back on the path that keeps buffering.
            match self.inside.take() {
                // Inside a suppressed element: consume until its close tag.
                Some(tag) => {
                    match self.find_close(&tag) {
                        Some(end) => {
                            self.buf.drain(..end);
                            // `inside` stays None — `take` already cleared it.
                        }
                        None => {
                            // Keep buffering; on flush the block never closed,
                            // so everything left is reasoning — drop it.
                            self.inside = Some(tag);
                            if flush {
                                self.buf.clear();
                            }
                            return out;
                        }
                    }
                }
                None => {
                    let Some(lt) = self.buf.find('<') else {
                        out.push_str(&self.buf);
                        self.buf.clear();
                        return out;
                    };
                    out.push_str(&self.buf[..lt]);

                    // Both regexes are `^`-anchored, so they can run against a
                    // borrow of the buffer. This used to `.to_string()` the
                    // entire remaining buffer on every iteration — a full copy
                    // of the rest of the response, per chunk — solely because
                    // `self.buf.drain()` is called later in the same scope.
                    // Extracting what we need first ends the borrow instead.
                    let rest = &self.buf[lt..];
                    let matched = tag_re().captures(rest).map(|caps| {
                        (
                            caps.get(0).map(|m| m.len()).unwrap_or(0),
                            !caps[1].is_empty(),
                            // One allocation, not two: the old code built the
                            // full name and then the local name separately.
                            local_name(&caps[2]).to_string(),
                        )
                    });
                    let could_still_grow = matched.is_none() && partial_tag_re().is_match(rest);

                    if let Some((whole, closing, local)) = matched {
                        if !closing && is_suppressed(&local) {
                            self.buf.drain(..lt + whole);
                            self.inside = Some(local);
                        } else {
                            // Not ours — an ordinary `<` (HTML in prose, a
                            // generic in code). Emit it and move past.
                            out.push('<');
                            self.buf.drain(..lt + 1);
                        }
                    } else if could_still_grow && !flush {
                        // Might still become a tag once more arrives.
                        self.buf.drain(..lt);
                        return out;
                    } else {
                        out.push('<');
                        self.buf.drain(..lt + 1);
                    }
                }
            }
        }
    }

    /// Byte offset just past `</tag>` (namespace-tolerant), if present.
    fn find_close(&self, tag: &str) -> Option<usize> {
        close_re(tag)?.find(&self.buf).map(|m| m.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The open tag has always been matched case-insensitively
    /// ([`is_suppressed`]), but the close tag was matched case-*sensitively*
    /// because its regex was built from the observed text. A model emitting
    /// `<Think>` … `</think>` therefore never closed the block, and everything
    /// after it was swallowed as reasoning. Now consistent in both directions.
    #[test]
    fn a_reasoning_block_closes_regardless_of_tag_case() {
        let mut f = StreamFilter::new();
        let mut out = f.push("before <Think>hidden</think> after");
        out.push_str(&f.finish());

        assert!(out.contains("before "), "{out:?}");
        assert!(
            out.contains(" after"),
            "lost the text after the block: {out:?}"
        );
        assert!(!out.contains("hidden"), "leaked reasoning: {out:?}");
    }

    #[test]
    fn a_namespaced_reasoning_block_still_closes() {
        let mut f = StreamFilter::new();
        let mut out = f.push("a <mm:think>r</mm:think> b");
        out.push_str(&f.finish());

        assert!(!out.contains('r') || out.contains("a "), "{out:?}");
        assert!(out.contains("a "), "{out:?}");
        assert!(out.contains(" b"), "{out:?}");
    }

    /// Feed a whole string one byte at a time — the worst case for a filter
    /// that assumes tags arrive intact.
    fn push_bytewise(input: &str) -> String {
        let mut f = StreamFilter::new();
        let mut out: String = input.chars().map(|c| f.push(&c.to_string())).collect();
        out.push_str(&f.finish());
        out
    }

    #[test]
    fn thinking_block_is_removed_whole() {
        let mut f = StreamFilter::new();
        let out = f.push("<thinking>Need to create the directory first.</thinking>Done.");
        assert_eq!(out + &f.finish(), "Done.");
    }

    #[test]
    fn tag_split_across_chunks_is_still_caught() {
        // The exact failure of a per-chunk strip.
        let mut f = StreamFilter::new();
        let mut out = String::new();
        for chunk in [
            "Now the db module.<thin",
            "king>secret plan</thin",
            "king>Next.",
        ] {
            out.push_str(&f.push(chunk));
        }
        out.push_str(&f.finish());
        assert_eq!(out, "Now the db module.Next.");
    }

    #[test]
    fn bytewise_streaming_matches_whole_string() {
        assert_eq!(
            push_bytewise("A<thinking>hidden</thinking>B"),
            "AB",
            "a tag split at every byte must filter identically"
        );
    }

    #[test]
    fn namespaced_reasoning_is_removed() {
        assert_eq!(push_bytewise("a<mm:think>x</mm:think>b"), "ab");
    }

    #[test]
    fn tool_call_markup_is_not_printed() {
        let out = push_bytewise(
            "Writing it now.<tool_call name=\"write_file\"><path>a.rs</path></tool_call>ok",
        );
        assert_eq!(out, "Writing it now.ok");
    }

    #[test]
    fn ordinary_angle_brackets_survive() {
        // Generics and comparisons are not tags and must reach the screen.
        assert_eq!(
            push_bytewise("fn f() -> Result<T, E> { a < b }"),
            "fn f() -> Result<T, E> { a < b }"
        );
    }

    #[test]
    fn html_in_prose_survives() {
        assert_eq!(
            push_bytewise("use <div> for layout"),
            "use <div> for layout"
        );
    }

    #[test]
    fn unterminated_reasoning_block_is_dropped() {
        // The stream ended mid-thought (cancelled, or the turn was cut off).
        // Emitting the tail would print raw reasoning.
        let mut f = StreamFilter::new();
        let out = f.push("visible<thinking>dangling reasoning");
        assert_eq!(out + &f.finish(), "visible");
    }

    #[test]
    fn lone_angle_bracket_is_flushed_not_swallowed() {
        // Held back while it might become a tag, but it must not vanish.
        let mut f = StreamFilter::new();
        let held = f.push("value <");
        assert_eq!(held, "value ");
        assert_eq!(f.finish(), "<");
    }

    #[test]
    fn a_turn_ending_mid_block_does_not_suppress_the_next_turn() {
        let mut f = StreamFilter::new();
        f.push("<thinking>cut off here");
        let _ = f.finish();
        // The observable consequence: the next turn is not swallowed.
        let out = f.push("fresh answer");
        assert_eq!(out + &f.finish(), "fresh answer");
    }

    #[test]
    fn back_to_back_blocks_are_both_removed() {
        assert_eq!(
            push_bytewise("<think>a</think>X<thinking>b</thinking>Y"),
            "XY"
        );
    }
}
