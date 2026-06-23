//! Design Mode → diffcomplete-into-DOM (gap A7) — §18.A7 cleared shape.
//!
//! Cursor shipped Design Mode GA (point / draw / narrate UI changes in the
//! browser; the agent edits the code underneath). The VibeCody shape is
//! deliberately distant from that surface, honoring the [§18](../../docs/FIT-GAP-ANALYSIS.md)
//! patent-distance principles:
//!
//! * **No agent-controlled browser, no live DOM mutation** (#7). The user
//!   attaches *their own* browser via CDP they authorized, clicks an element,
//!   and types an instruction. This module turns that into a **CSS/HTML unified
//!   diff against the source file**, which the user reviews and applies through
//!   the existing `DiffReviewPanel` / diffcomplete (⌘.) mechanism — the same
//!   claim-element posture as the inline-completion Path D.
//! * **No closed-loop hidden iteration** (#8): one explicit user chord per
//!   refinement; this produces a single diff, never a screenshot-diff retry loop.
//! * **No hidden RAG / cross-file taint** (#9): the prompt sees only the element
//!   the user selected and the source snippet, nothing auto-gathered.
//!
//! This is the pure transform — selection + instruction → prompt, and LLM output
//! → a reviewable unified diff. The daemon/UI owns the CDP attach and the diff
//! application; this layer is testable without a browser or provider.

use serde::{Deserialize, Serialize};

/// An element the user clicked in their own (CDP-attached) browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectedElement {
    /// CSS selector / DOM path of the clicked element.
    pub selector: String,
    /// The source file backing the element (the diff target).
    pub source_file: String,
    /// The current source snippet for that element (HTML/JSX/CSS).
    pub snippet: String,
}

/// The result of a design edit: a unified diff for `DiffReviewPanel`. Crucially
/// it is a *diff against the source file*, never a DOM-mutation instruction —
/// the §18.A7 invariant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignDiff {
    pub source_file: String,
    /// Unified-diff text (the same format diffcomplete / DiffReviewPanel consume).
    pub unified_diff: String,
}

/// Build the provider-agnostic prompt for a design edit. The model is told to
/// emit a unified diff against the source — never DOM-mutation JS — which keeps
/// the output on the cleared-shape rails regardless of provider.
pub fn build_design_prompt(element: &SelectedElement, instruction: &str) -> String {
    format!(
        "You are editing UI source code. The user selected this element \
         (selector `{selector}`) in file `{file}` and asked:\n\n\"{instruction}\"\n\n\
         Current source for the element:\n```\n{snippet}\n```\n\n\
         Return ONLY a unified diff (`--- a/{file}` / `+++ b/{file}` with @@ hunks) \
         that makes the requested change to the SOURCE. Do NOT return JavaScript \
         that mutates the live DOM, and do not change anything outside the selected \
         element. If no change is needed, return an empty diff.",
        selector = element.selector,
        file = element.source_file,
        instruction = instruction.trim(),
        snippet = element.snippet,
    )
}

/// Extract the unified diff from the model's reply into a [`DesignDiff`].
///
/// Tolerant of a fenced ```diff block or a bare diff body. Enforces the §18.A7
/// invariant: any reply that looks like live-DOM mutation (e.g. `document.`,
/// `.style.`, `.innerHTML`) instead of a diff is **rejected**, so the agent can
/// never smuggle a DOM-mutation payload past the cleared shape.
pub fn parse_design_diff(element: &SelectedElement, reply: &str) -> Result<DesignDiff, String> {
    let body = extract_diff_block(reply);
    let trimmed = body.trim();

    if trimmed.is_empty() {
        // "No change needed" is a valid, empty diff.
        return Ok(DesignDiff {
            source_file: element.source_file.clone(),
            unified_diff: String::new(),
        });
    }

    // Reject DOM-mutation payloads outright (§18.A7 #7).
    let lowered = trimmed.to_lowercase();
    let looks_like_dom_mutation = (lowered.contains("document.")
        || lowered.contains(".innerhtml")
        || lowered.contains(".style.")
        || lowered.contains("queryselector"))
        && !is_unified_diff(trimmed);
    if looks_like_dom_mutation {
        return Err(
            "rejected: reply looks like live-DOM mutation, not a source diff (§18.A7)".to_string(),
        );
    }

    if !is_unified_diff(trimmed) {
        return Err("reply is not a unified diff".to_string());
    }

    Ok(DesignDiff {
        source_file: element.source_file.clone(),
        unified_diff: trimmed.to_string(),
    })
}

/// Pull a fenced ```diff / ```patch block if present, else return the whole reply.
fn extract_diff_block(reply: &str) -> String {
    for fence in ["```diff", "```patch", "```"] {
        if let Some(start) = reply.find(fence) {
            let after = &reply[start + fence.len()..];
            if let Some(end) = after.find("```") {
                return after[..end].trim_start_matches('\n').to_string();
            }
        }
    }
    reply.to_string()
}

/// Minimal unified-diff recognizer: has file headers or at least one hunk header.
fn is_unified_diff(s: &str) -> bool {
    let has_headers = s.contains("--- ") && s.contains("+++ ");
    let has_hunk = s.lines().any(|l| l.starts_with("@@") && l.contains("@@"));
    has_headers || has_hunk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elem() -> SelectedElement {
        SelectedElement {
            selector: ".cta-button".into(),
            source_file: "src/App.tsx".into(),
            snippet: "<button className=\"cta-button\">Buy</button>".into(),
        }
    }

    #[test]
    fn prompt_demands_a_diff_not_dom_js() {
        let p = build_design_prompt(&elem(), "make it green");
        assert!(p.contains("unified diff"));
        assert!(p.to_lowercase().contains("do not return javascript"));
        assert!(p.contains(".cta-button"));
    }

    #[test]
    fn parses_fenced_unified_diff() {
        let reply = "Here's the change:\n```diff\n--- a/src/App.tsx\n+++ b/src/App.tsx\n@@ -1 +1 @@\n-<button className=\"cta-button\">Buy</button>\n+<button className=\"cta-button green\">Buy</button>\n```";
        let d = parse_design_diff(&elem(), reply).unwrap();
        assert_eq!(d.source_file, "src/App.tsx");
        assert!(d.unified_diff.contains("@@"));
        assert!(d.unified_diff.contains("green"));
    }

    #[test]
    fn rejects_live_dom_mutation() {
        let reply = "document.querySelector('.cta-button').style.color = 'green';";
        let err = parse_design_diff(&elem(), reply).unwrap_err();
        assert!(err.contains("§18.A7") || err.contains("live-DOM"));
    }

    #[test]
    fn empty_reply_is_empty_diff() {
        let d = parse_design_diff(&elem(), "   ").unwrap();
        assert!(d.unified_diff.is_empty());
    }

    #[test]
    fn non_diff_prose_is_rejected() {
        let err = parse_design_diff(&elem(), "Sure, I changed the button to green.").unwrap_err();
        assert!(err.contains("not a unified diff"));
    }
}
