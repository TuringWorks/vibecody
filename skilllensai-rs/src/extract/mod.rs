//! Skill extraction — distil an experience pool into candidate skills.
//!
//! [`sequential`] is the single-pass baseline; [`parallel`] is the primary
//! method (per-trajectory mode extraction + hierarchical merge, semaphore
//! fan-out). Both require a [`crate::llm::SkillLlm`] and are gated on the
//! `llm` feature.
//!
//! The LLM is asked to emit a VibeCody-style skill document (YAML frontmatter
//! `triggers` / `tools_allowed` / `category` + a numbered-step body); the
//! response is parsed with [`crate::model::skill::Skill::from_str_named`].

#[cfg(feature = "llm")]
pub mod parallel;
#[cfg(feature = "llm")]
pub mod sequential;

#[cfg(feature = "llm")]
pub use parallel::ParallelExtractor;
#[cfg(feature = "llm")]
pub use sequential::SequentialExtractor;

#[cfg(feature = "llm")]
use crate::llm::SkillLlm;
#[cfg(feature = "llm")]
use crate::model::experience::ExperiencePool;
#[cfg(feature = "llm")]
use crate::model::skill::Skill;

/// The shared extraction contract. Both extractors produce one or more
/// candidate [`Skill`]s from an [`ExperiencePool`].
#[cfg(feature = "llm")]
#[async_trait::async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(
        &self,
        pool: &ExperiencePool,
        llm: &dyn SkillLlm,
    ) -> anyhow::Result<Vec<Skill>>;
}

/// Shared system prompt used by both extractors — instructs the model to emit a
/// single skill document with VibeCody frontmatter + numbered steps.
#[cfg(feature = "llm")]
const EXTRACT_SYSTEM: &str = "\
You are a skill-mining engineer. Distil the agent behaviour shown into ONE reusable \
agent-skill document in VibeCody format. Output ONLY the skill markdown, starting with \
a YAML frontmatter block delimited by `---` and containing `triggers` (JSON string \
array of intent phrases), `tools_allowed` (JSON string array), and `category`. \
Then a concise numbered-step body (markdown). No commentary.";

/// Parse the model's response into a [`Skill`], using `fallback_name` when
/// the response carries no usable identifier.
#[cfg(feature = "llm")]
pub(crate) fn parse_skill_response(raw: &str, fallback_name: &str) -> Skill {
    // The model may wrap output in fenced ```markdown blocks; strip a single
    // wrapping fence pair so the frontmatter parser sees a leading `---`.
    let trimmed = strip_code_fence(raw);
    let name = derive_name(trimmed).unwrap_or_else(|| fallback_name.to_string());
    Skill::from_str_named(&name, trimmed)
}

#[cfg(feature = "llm")]
fn strip_code_fence(raw: &str) -> &str {
    let t = raw.trim_start();
    if let Some(rest) = t.strip_prefix("```") {
        // drop the language tag line up to the first newline
        let rest = rest.trim_start_matches(|c: char| c.is_alphanumeric() || c == '-');
        let rest = rest.trim_start_matches('\n');
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
        return rest.trim();
    }
    raw.trim()
}

/// Derive a skill name from the first markdown heading, slugified; `None` if
/// no heading is present.
#[cfg(feature = "llm")]
fn derive_name(s: &str) -> Option<String> {
    let line = s.lines().find(|l| l.trim_start().starts_with('#'))?;
    let heading = line.trim_start_matches('#').trim();
    if heading.is_empty() {
        return None;
    }
    let slug: String = heading
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    (!slug.is_empty()).then_some(slug)
}

#[cfg(all(feature = "llm", test))]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_skill_response() {
        let raw = "```markdown\n---\ntriggers: [\"x\"]\ncategory: c\n---\n# My Skill\n1. step\n```";
        let s = parse_skill_response(raw, "fallback");
        assert_eq!(s.name, "my-skill");
        assert_eq!(s.triggers, vec!["x"]);
        assert_eq!(s.category, "c");
        assert!(s.body.contains("step"));
    }

    #[test]
    fn parses_unfenced_response_with_heading_name() {
        let raw = "---\ntriggers: [\"a\"]\ncategory: c\n---\n# Formal Verification\nbody";
        let s = parse_skill_response(raw, "fb");
        assert_eq!(s.name, "formal-verification");
    }

    #[test]
    fn falls_back_when_no_heading() {
        let raw = "---\ntriggers: [\"a\"]\ncategory: c\n---\nbody only";
        let s = parse_skill_response(raw, "fallback");
        assert_eq!(s.name, "fallback");
    }
}
