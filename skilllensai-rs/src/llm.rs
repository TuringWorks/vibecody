//! The provider-agnostic LLM seam (feature = "llm").
//!
//! STRICT rule (CLAUDE.md → Provider-Agnostic Panels): this crate never
//! hard-codes a provider. It defines the [`SkillLlm`] trait; the vibecli bridge
//! (`skillforge_index.rs`) implements it over `vibe_ai::AIProvider`, honouring
//! the toolbar-selected provider+model. [`NullLlm`] lets code depend on the
//! trait while running non-LLM paths with no key configured.

use serde::{Deserialize, Serialize};

/// Which provider+model a [`SkillLlm`] is bound to (for reports / provenance).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmDescriptor {
    pub provider: String,
    pub model: String,
}

/// The single chat entry-point extraction, scoring, and the optimizer use.
///
/// Deliberately minimal (system + user → text) so any backend can implement it.
#[async_trait::async_trait]
pub trait SkillLlm: Send + Sync {
    /// The provider+model this instance dispatches to.
    fn descriptor(&self) -> LlmDescriptor;

    /// One completion. Implementations should be retry/cost aware upstream.
    async fn chat(&self, system: &str, user: &str) -> anyhow::Result<String>;
}

/// A no-op backend that errors on `chat`. Use it to satisfy the trait bound on
/// paths that don't actually call the model (parsing, static metrics, tests).
pub struct NullLlm;

#[async_trait::async_trait]
impl SkillLlm for NullLlm {
    fn descriptor(&self) -> LlmDescriptor {
        LlmDescriptor {
            provider: "null".to_string(),
            model: "null".to_string(),
        }
    }

    async fn chat(&self, _system: &str, _user: &str) -> anyhow::Result<String> {
        anyhow::bail!("NullLlm: no LLM configured — this path requires a real SkillLlm")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_llm_reports_and_errors() {
        let llm = NullLlm;
        assert_eq!(llm.descriptor().provider, "null");
        assert!(llm.chat("sys", "usr").await.is_err());
    }
}
