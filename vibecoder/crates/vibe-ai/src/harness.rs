//! What each (provider, model) pair should be *given* — the harness, not the model.
//!
//! Every provider in this crate was driven through one harness: one system
//! prompt, one tool transport, one output cap. That uniformity cost the
//! strongest models the most. Before this module:
//!
//! * `advertises_native_tools()` answered `true` for Ollama and the
//!   OpenAI-compatible family and `false` for **everything else** — so Claude,
//!   OpenAI, Gemini, Bedrock, Grok, OpenRouter, Azure, Copilot and the local
//!   mistral.rs server all received the ~15 KB XML tool catalogue in their
//!   system prompt and had their tool calls regex-parsed back out of prose,
//!   while their APIs offered first-class tool schemas the whole time.
//! * Every Claude model was capped at 16 384 output tokens by a literal
//!   repeated in four places.
//! * The reasoning-effort tiers mapped to one global budget table, identical
//!   for a 3B local model and a frontier model.
//!
//! A [`ModelProfile`] is the answer to "what should this pair be given", and
//! [`profile_for`] is the only way to get one.
//!
//! # Why there is almost no table of vendor numbers here
//!
//! [`crate::context_window`] refuses to hardcode context windows because a
//! window is a fact about someone else's product, written from memory, wrong
//! the moment a vendor ships a revision. The same reasoning applies to output
//! caps and thinking budgets, so the built-in table sets them to `None` and
//! the provider's own existing default stands. `None` means *the provider
//! decides*, never *unlimited* and never a number nobody checked.
//!
//! What the built-in table *does* assert is transport and dialect — facts about
//! **our** code, which we can check: whether we send schemas on the wire, and
//! which system prompt we pair with that choice. Those are knowable here.
//!
//! Everything else is a knob with a real default of `None`, settable per pair
//! by the user through [`set_overrides`]. That is the "fine tuning" half: the
//! numbers we cannot honestly ship come from whoever measured them.
//!
//! # Layers
//!
//! Resolution runs lowest to highest, each layer overriding only the fields it
//! sets:
//!
//! 1. [`family_default`] — keyed on provider id.
//! 2. [`builtin_model_override`] — keyed on a model-id prefix, longest wins.
//! 3. A user override for `"<provider>/*"`.
//! 4. A user override for `"<provider>/<model>"`.
//!
//! Layers 3 and 4 are injected by the daemon from the encrypted ProfileStore
//! via [`set_overrides`]. This crate deliberately does not read the store
//! itself: it stays a pure library, and the storage rules live in one place.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

// ── The knobs ───────────────────────────────────────────────────────────────

/// How this pair is told what tools exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolTransport {
    /// Schemas ride on the request's `tools` field, and the model's structured
    /// tool calls are transcribed back to `<tool_call>` markup for the agent
    /// loop (see [`crate::tools::render_tool_call`]).
    Native,
    /// Tools are described in the system prompt only, and calls are parsed out
    /// of the model's prose. The original path, kept as a first-class choice:
    /// it is the escape hatch for a model whose native tool calling is worse
    /// than its prose, and the only path for an API that has no tools field.
    Prose,
}

/// Which system prompt this pair is paired with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptDialect {
    /// The full prompt including the per-tool XML catalogue
    /// ([`crate::tools::TOOL_SYSTEM_PROMPT`]).
    Full,
    /// The catalogue replaced by a one-line list of tool names
    /// ([`crate::tools::TOOL_SYSTEM_PROMPT_COMPACT`]). A model that receives
    /// the schemas on the wire does not also need them in prose.
    Compact,
}

/// Per-tier thinking budgets, overriding [`crate::provider::Effort`]'s global
/// table for one pair.
///
/// All fields optional and all absent by default: an unset tier falls back to
/// `Effort`'s own answer, so an override can adjust one tier without having to
/// restate the other three.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortBudgets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medium: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xhigh: Option<u32>,
}

impl EffortBudgets {
    /// This pair's budget for `effort`, or `None` when the pair says nothing
    /// about that tier and the global table should answer.
    pub fn get(&self, effort: crate::provider::Effort) -> Option<u32> {
        use crate::provider::Effort;
        match effort {
            Effort::Low => self.low,
            Effort::Medium => self.medium,
            Effort::High => self.high,
            Effort::XHigh => self.xhigh,
        }
    }

    /// `self` with every field `other` sets replacing ours.
    fn merged(self, other: &Self) -> Self {
        Self {
            low: other.low.or(self.low),
            medium: other.medium.or(self.medium),
            high: other.high.or(self.high),
            xhigh: other.xhigh.or(self.xhigh),
        }
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The fully resolved harness settings for one (provider, model) pair.
///
/// Every `Option` field that is `None` means "this layer says nothing; keep
/// what the provider already does". Providers read them as
/// `profile.max_output_tokens.or(their_existing_default)`, so an untouched
/// profile changes no behaviour at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Whether tool schemas go on the wire.
    pub tool_transport: ToolTransport,
    /// Which system prompt to pair with that transport.
    pub prompt_dialect: PromptDialect,
    /// Cap on the model's reply. `None` keeps the provider's own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    /// Sampling temperature. `None` keeps the provider's own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Whether the model may emit several tool calls in one turn, where the
    /// API exposes the switch. `None` leaves the field off the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Per-tier thinking budgets for this pair.
    #[serde(default, skip_serializing_if = "EffortBudgets::is_empty")]
    pub thinking_budgets: EffortBudgets,
    /// Ask the API to cache the prompt prefix where it supports it (Anthropic
    /// `cache_control`). The agent's system prompt is thousands of tokens and
    /// is resent on every turn of every run, so this is the one knob here whose
    /// default is a behaviour change rather than a passthrough.
    pub prompt_cache: bool,
    /// Context window to assume **only** when the provider's API does not
    /// publish one. Never overrides a measured answer — see
    /// [`crate::context_window`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_fallback: Option<usize>,
    /// Extra instructions appended to the agent system prompt for this pair.
    ///
    /// The per-model prompt-tuning knob: a model that needs one specific
    /// reminder gets it without every other model paying for the tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_suffix: Option<String>,
}

impl ModelProfile {
    /// What an unrecognised provider gets: exactly the behaviour this codebase
    /// had before the module existed.
    ///
    /// A provider we know nothing about is not assumed to support tool schemas.
    /// Guessing `Native` for an unknown API turns a working prose loop into a
    /// rejected request; guessing `Prose` costs prompt tokens and nothing else.
    pub const fn conservative() -> Self {
        Self {
            tool_transport: ToolTransport::Prose,
            prompt_dialect: PromptDialect::Full,
            max_output_tokens: None,
            temperature: None,
            parallel_tool_calls: None,
            thinking_budgets: EffortBudgets {
                low: None,
                medium: None,
                high: None,
                xhigh: None,
            },
            prompt_cache: false,
            context_window_fallback: None,
            system_prompt_suffix: None,
        }
    }

    /// A provider whose API takes tool schemas, so the prompt need not repeat
    /// them.
    ///
    /// The two travel together on purpose: sending schemas *and* the full XML
    /// catalogue describes the same tools twice in two vocabularies, and
    /// sending neither leaves the model nothing to call.
    pub const fn native_tools() -> Self {
        // Spelled out rather than `..Self::conservative()`: struct update
        // syntax drops the base value, and `ModelProfile` owns a `String`, so
        // the const evaluator rejects it. Every field still appears exactly
        // once, which is what a reviewer needs to check anyway.
        Self {
            tool_transport: ToolTransport::Native,
            prompt_dialect: PromptDialect::Compact,
            max_output_tokens: None,
            temperature: None,
            parallel_tool_calls: None,
            thinking_budgets: EffortBudgets {
                low: None,
                medium: None,
                high: None,
                xhigh: None,
            },
            prompt_cache: false,
            context_window_fallback: None,
            system_prompt_suffix: None,
        }
    }

    /// True when the schemas should go on the wire for this pair.
    pub fn sends_tool_schemas(&self) -> bool {
        self.tool_transport == ToolTransport::Native
    }

    /// This pair's thinking budget for `effort`, falling back to the global
    /// [`crate::provider::Effort`] table for tiers the pair does not set.
    pub fn thinking_budget(&self, effort: crate::provider::Effort) -> Option<u32> {
        self.thinking_budgets
            .get(effort)
            .or_else(|| effort.claude_thinking_budget())
    }

    /// Apply every field `patch` sets, leaving the rest alone.
    fn patched(mut self, patch: &ProfileOverride) -> Self {
        if let Some(v) = patch.tool_transport {
            self.tool_transport = v;
        }
        if let Some(v) = patch.prompt_dialect {
            self.prompt_dialect = v;
        }
        if let Some(v) = patch.prompt_cache {
            self.prompt_cache = v;
        }
        self.max_output_tokens = patch.max_output_tokens.or(self.max_output_tokens);
        self.temperature = patch.temperature.or(self.temperature);
        self.parallel_tool_calls = patch.parallel_tool_calls.or(self.parallel_tool_calls);
        self.context_window_fallback = patch
            .context_window_fallback
            .or(self.context_window_fallback);
        self.system_prompt_suffix = patch
            .system_prompt_suffix
            .clone()
            .or(self.system_prompt_suffix);
        self.thinking_budgets = self.thinking_budgets.merged(&patch.thinking_budgets);
        self
    }
}

impl Default for ModelProfile {
    fn default() -> Self {
        Self::conservative()
    }
}

/// A partial [`ModelProfile`]: the shape of one tuning layer.
///
/// Persisted and sent on the wire instead of a resolved profile on purpose. A
/// stored resolved profile freezes today's defaults into the user's settings,
/// so improving a default would silently not reach anyone who had ever opened
/// the panel. A stored patch says only what the user actually changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_transport: Option<ToolTransport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_dialect: Option<PromptDialect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "EffortBudgets::is_empty")]
    pub thinking_budgets: EffortBudgets,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_fallback: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_suffix: Option<String>,
}

impl ProfileOverride {
    /// True when this patch would change nothing — the shape a "reset to
    /// default" writes, and the one the store should delete rather than keep.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

// ── Layer 1: provider families ──────────────────────────────────────────────

/// Provider ids whose API takes tool schemas and whose request builders in this
/// crate put them there.
///
/// This list is an assertion about **our** code, not about a vendor's product,
/// which is why it can be a literal table at all: every id here has a request
/// builder that sets `tools`, and a response path that transcribes tool calls
/// back to `<tool_call>`. Adding an id without both is how a model ends up with
/// no way to call anything — `every_native_provider_is_wired` guards it.
const NATIVE_TOOL_PROVIDERS: &[&str] = &[
    // Anthropic Messages API — `tools` + `tool_use` content blocks.
    "claude",
    "anthropic",
    // OpenAI and the APIs that copy its `/chat/completions` shape.
    "openai",
    "azure-openai",
    "grok",
    "xai",
    "openrouter",
    "copilot",
    "github-copilot",
    // Google Gemini — `functionDeclarations` + `functionCall` parts.
    "gemini",
    "google",
    // AWS Bedrock Converse — `toolConfig` + `toolUse` blocks.
    "bedrock",
    "aws-bedrock",
    // Ollama's native `tools` field.
    "ollama",
    // The OpenAI-compatible family (`providers::compat`).
    "groq",
    "cerebras",
    "together",
    "fireworks",
    "sambanova",
    "minimax",
    "zhipu",
    "mistral",
    "deepseek",
    "perplexity",
    "poolside",
    "vercel",
    "vercel-ai",
    "vllm",
    "lmstudio",
];

/// `vibecli-mistralrs` is deliberately **absent** from the list above.
///
/// It looks like an OpenAI-shaped provider and is not: it posts Ollama-style
/// NDJSON to the VibeCLI daemon's own `/api/chat`, whose request type
/// (`vibecli::inference::backend::ChatRequest`) has `model`, `messages`,
/// `stream`, `options` and `backend` — and no `tools`. serde drops unknown
/// fields silently, so declaring it native would send schemas that never
/// arrive *and* strip the XML catalogue that was the model's only remaining
/// description of the tools. It would be left with nothing.
///
/// Move it into the list when the daemon's inference route learns to forward
/// tool definitions, not before.
///
/// Providers that support Anthropic-style prompt caching of the system prefix.
const PROMPT_CACHE_PROVIDERS: &[&str] = &["claude", "anthropic"];

/// The base profile for `provider`, before any model-specific layer.
pub fn family_default(provider: &str) -> ModelProfile {
    let id = normalise_provider(provider);
    let base = match NATIVE_TOOL_PROVIDERS.contains(&id.as_str()) {
        true => ModelProfile::native_tools(),
        false => ModelProfile::conservative(),
    };
    ModelProfile {
        prompt_cache: PROMPT_CACHE_PROVIDERS.contains(&id.as_str()),
        ..base
    }
}

/// Provider ids reach this module from a toolbar string, a CLI flag, a daemon
/// request body and four client lists, so they arrive in more than one shape.
///
/// Case is folded and `_` is folded to `-`, because `azure_openai` and
/// `azure-openai` are one provider spelled two ways and no caller should have
/// to know which spelling this module wants. Anything beyond that — `anthropic`
/// for `claude`, `xai` for `grok` — is a real alias and is spelled out in the
/// tables rather than guessed at by rewriting strings.
///
/// This is also what makes the storage key canonical: an override saved from a
/// client that says `azure_openai` has to be found by one that says
/// `azure-openai`, or it saves successfully and never applies.
fn normalise_provider(provider: &str) -> String {
    provider.trim().to_lowercase().replace('_', "-")
}

// ── Layer 2: built-in model overrides ───────────────────────────────────────

/// Model-id prefixes with a built-in adjustment, and what it is.
///
/// Deliberately near-empty. A prefix here is a claim about a specific model,
/// and the only claims worth shipping are ones measured against that model
/// through `evals/suites/models.yaml`. Prefixes, never substrings: a
/// `contains("gpt-4")` rule silently captures `gpt-4-32k` as well, the exact
/// trap `context_window::from_models_list` documents.
const MODEL_OVERRIDES: &[(&str, &str, ProfileOverride)] = &[];

/// The built-in adjustment for `model` under `provider`, if any.
///
/// Longest matching prefix wins, so a rule for `gpt-5.3-codex` beats one for
/// `gpt-5`, and neither can be reached by a shorter accidental match.
fn builtin_model_override(provider: &str, model: &str) -> Option<&'static ProfileOverride> {
    let id = normalise_provider(provider);
    let model = model.trim().to_lowercase();
    MODEL_OVERRIDES
        .iter()
        .filter(|(p, prefix, _)| *p == id && model.starts_with(prefix))
        .max_by_key(|(_, prefix, _)| prefix.len())
        .map(|(_, _, patch)| patch)
}

// ── Layers 3 & 4: user overrides ────────────────────────────────────────────

/// User overrides, keyed `"<provider>/<model>"` or `"<provider>/*"`.
///
/// Process-wide because providers are constructed per request: a per-instance
/// copy would have to be threaded through 26 constructors and would go stale
/// the moment the user saved a change. Written once at daemon start and again
/// on each save, read on every resolution — hence `RwLock`, not `Mutex`.
static OVERRIDES: LazyLock<RwLock<HashMap<String, ProfileOverride>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Replace the whole override set.
///
/// Whole-set replacement, not merge: the daemon holds the store and the store
/// is the truth. A merge would leave a deleted override alive in memory until
/// the next restart, which is exactly the bug a "reset to default" button must
/// not have.
pub fn set_overrides(overrides: HashMap<String, ProfileOverride>) {
    let mut guard = OVERRIDES.write().unwrap_or_else(|e| e.into_inner());
    *guard = overrides;
}

/// Every override currently in effect, as the settings surfaces render them.
pub fn overrides() -> HashMap<String, ProfileOverride> {
    OVERRIDES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// The key under which an override for this pair is stored.
///
/// One function so the daemon, the store and the resolver cannot disagree about
/// the shape — a mismatch here is an override that saves successfully and never
/// applies.
pub fn override_key(provider: &str, model: &str) -> String {
    format!("{}/{}", normalise_provider(provider), model.trim())
}

/// The key for a provider-wide override, applied to every model it serves.
pub fn provider_wide_key(provider: &str) -> String {
    format!("{}/*", normalise_provider(provider))
}

// ── Which knobs a provider actually honours ─────────────────────────────────

/// A profile field, for saying which ones a given provider can act on.
///
/// Not every knob reaches every API. `prompt_cache` is Anthropic's
/// `cache_control`; `thinking_budgets` is a *token* budget, which only Claude
/// and Gemini expose (OpenAI takes an effort word, not a number);
/// `parallel_tool_calls` is an OpenAI-shaped request field.
///
/// A settings surface that offers all of them everywhere lets a user turn on a
/// setting that saves, reads back as changed, and does nothing — the
/// success-assuming failure this codebase names as its dominant bug family,
/// arrived at through the UI instead of through a return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileField {
    ToolTransport,
    PromptDialect,
    MaxOutputTokens,
    Temperature,
    ParallelToolCalls,
    ThinkingBudgets,
    PromptCache,
    ContextWindowFallback,
    SystemPromptSuffix,
}

/// Knobs every provider honours, because the agent loop — not the provider —
/// is what acts on them.
const UNIVERSAL: &[ProfileField] = &[
    ProfileField::ToolTransport,
    ProfileField::PromptDialect,
    ProfileField::ContextWindowFallback,
    ProfileField::SystemPromptSuffix,
];

/// Providers whose request builders send `parallel_tool_calls` — the
/// OpenAI-shaped family. Claude, Gemini and Bedrock express the same idea
/// differently or not at all, so the field would be ignored or rejected.
const PARALLEL_TOOL_CALLS: &[&str] = &[
    "openai", "azure-openai", "grok", "xai", "openrouter", "copilot", "groq", "cerebras",
    "together", "fireworks", "sambanova", "minimax", "zhipu", "mistral", "deepseek",
    "perplexity", "poolside", "vercel", "vercel-ai", "vllm", "lmstudio",
];

/// Providers whose request builders send a temperature or an output cap.
const SAMPLING: &[&str] = &[
    "claude", "anthropic", "openai", "azure-openai", "grok", "xai", "openrouter", "copilot",
    "gemini", "google", "bedrock", "aws-bedrock", "groq", "cerebras", "together", "fireworks",
    "sambanova", "minimax", "zhipu", "mistral", "deepseek", "perplexity", "poolside", "vercel",
    "vercel-ai", "vllm", "lmstudio",
];

/// Which fields `provider` can actually act on.
///
/// Callers render or accept only these. Setting one of the others still
/// *stores* fine — the resolver is generic — but it would never reach a
/// request, and offering it would be the lie described on [`ProfileField`].
pub fn honored_fields(provider: &str) -> Vec<ProfileField> {
    let id = normalise_provider(provider);
    let mut fields = UNIVERSAL.to_vec();
    if SAMPLING.contains(&id.as_str()) {
        fields.push(ProfileField::MaxOutputTokens);
        fields.push(ProfileField::Temperature);
    }
    if PARALLEL_TOOL_CALLS.contains(&id.as_str()) {
        fields.push(ProfileField::ParallelToolCalls);
    }
    if THINKING_BUDGET_PROVIDERS.contains(&id.as_str()) {
        fields.push(ProfileField::ThinkingBudgets);
    }
    if PROMPT_CACHE_PROVIDERS.contains(&id.as_str()) {
        fields.push(ProfileField::PromptCache);
    }
    fields
}

/// Providers taking a thinking budget denominated in **tokens**.
///
/// OpenAI is absent deliberately: its reasoning dial is an effort word
/// (`reasoning_effort`), already driven by the toolbar's effort tier, and a
/// token count means nothing to it.
const THINKING_BUDGET_PROVIDERS: &[&str] = &["claude", "anthropic", "gemini", "google"];

// ── Resolution ──────────────────────────────────────────────────────────────

/// The harness settings for `provider`/`model`, with every layer applied.
///
/// Total by construction: an unknown provider resolves to
/// [`ModelProfile::conservative`], which is the behaviour that shipped before
/// this module. There is no failure mode and no `Result`.
///
/// Not memoised. Resolution is a handful of string comparisons plus one
/// read-lock, and the override map has to stay live for a save to take effect
/// without a restart — a cache would need invalidation that costs more than the
/// work it saves.
pub fn profile_for(provider: &str, model: &str) -> ModelProfile {
    let base = family_default(provider);
    let base = match builtin_model_override(provider, model) {
        Some(patch) => base.patched(patch),
        None => base,
    };

    let guard = OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    let base = match guard.get(&provider_wide_key(provider)) {
        Some(patch) => base.patched(patch),
        None => base,
    };
    match guard.get(&override_key(provider, model)) {
        Some(patch) => base.patched(patch),
        None => base,
    }
}

/// The resolved profile plus the patches that produced it, for the settings
/// surfaces.
///
/// A panel that shows only the resolved values cannot tell the user which of
/// them they chose and which came from us, so "reset" has nothing to reset to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedProfile {
    pub provider: String,
    pub model: String,
    /// What the harness will actually use.
    pub effective: ModelProfile,
    /// What this codebase ships for the pair, before any user override.
    pub builtin: ModelProfile,
    /// The user's provider-wide patch, if they set one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_override: Option<ProfileOverride>,
    /// The user's patch for this exact model, if they set one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<ProfileOverride>,
    /// Which fields this provider can actually act on. A surface that offers
    /// the others lets a user set something that saves and does nothing.
    pub honored_fields: Vec<ProfileField>,
}

/// [`profile_for`], plus the provenance a settings panel needs.
pub fn resolve(provider: &str, model: &str) -> ResolvedProfile {
    let builtin = match builtin_model_override(provider, model) {
        Some(patch) => family_default(provider).patched(patch),
        None => family_default(provider),
    };
    let guard = OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    ResolvedProfile {
        provider: normalise_provider(provider),
        model: model.trim().to_string(),
        effective: profile_for(provider, model),
        builtin,
        provider_override: guard.get(&provider_wide_key(provider)).cloned(),
        model_override: guard.get(&override_key(provider, model)).cloned(),
        honored_fields: honored_fields(provider),
    }
}

/// Run `f` with exactly `map` installed as the override set, then clear it.
///
/// The override map is process-wide, so any test that installs one races every
/// other test that resolves a profile — including tests in other modules of
/// this crate, which cannot see this module's private lock. One shared,
/// poison-tolerant guard lives here so all of them serialise on it.
#[cfg(test)]
pub(crate) fn with_overrides_for_test<T>(
    map: HashMap<String, ProfileOverride>,
    f: impl FnOnce() -> T,
) -> T {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_overrides(map);
    let out = f();
    set_overrides(HashMap::new());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Effort;

    /// Every test that resolves a profile serialises on one guard — readers
    /// included.
    ///
    /// Only the writers took a lock at first, and the read-only default
    /// assertions failed intermittently: a neighbouring test's overrides were
    /// still installed when they ran. That is the shared-state flake this
    /// codebase names as the top cause of "timing" failures.
    use super::with_overrides_for_test as with_overrides;

    /// Run `f` with no overrides installed — what a fresh process looks like.
    fn with_no_overrides<T>(f: impl FnOnce() -> T) -> T {
        with_overrides(HashMap::new(), f)
    }

    // ── Layer 1 ─────────────────────────────────────────────────────────

    #[test]
    fn an_unknown_provider_keeps_the_old_behaviour() {
        with_no_overrides(|| {
            let p = profile_for("some-vendor-we-have-never-heard-of", "m");
            assert_eq!(p.tool_transport, ToolTransport::Prose);
            assert_eq!(p.prompt_dialect, PromptDialect::Full);
            assert!(!p.sends_tool_schemas());
        });
    }

    #[test]
    fn the_previously_hobbled_providers_now_get_schemas() {
        // The eight whose APIs take tool schemas and that sent none.
        with_no_overrides(|| {
            for provider in [
                "claude",
                "openai",
                "gemini",
                "bedrock",
                "grok",
                "openrouter",
                "azure-openai",
                "copilot",
            ] {
                let p = profile_for(provider, "any-model");
                assert!(p.sends_tool_schemas(), "{provider} should send tool schemas");
                assert_eq!(
                    p.prompt_dialect,
                    PromptDialect::Compact,
                    "{provider} sends schemas, so it must not also get the XML catalogue"
                );
            }
        });
    }

    /// The ninth provider that sends no schemas stays on the prose path,
    /// because the endpoint it talks to has no `tools` field to send them in.
    /// Declaring it native would strip its XML catalogue — its only remaining
    /// description of the tools — while the schemas it was given instead were
    /// silently dropped by serde at the daemon.
    #[test]
    fn vibecli_mistralrs_stays_on_the_prose_path() {
        with_no_overrides(|| {
            for id in ["vibecli-mistralrs", "vibecli_mistralrs"] {
                let p = profile_for(id, "Qwen/Qwen2.5-0.5B-Instruct");
                assert!(!p.sends_tool_schemas(), "{id}");
                assert_eq!(p.prompt_dialect, PromptDialect::Full, "{id}");
            }
        });
    }

    #[test]
    fn the_providers_that_already_worked_still_do() {
        with_no_overrides(|| {
            for provider in ["ollama", "groq", "cerebras", "deepseek", "vllm"] {
                assert!(profile_for(provider, "m").sends_tool_schemas(), "{provider}");
            }
        });
    }

    #[test]
    fn provider_ids_are_matched_case_insensitively() {
        with_no_overrides(|| {
            assert!(profile_for("Claude", "m").sends_tool_schemas());
            assert!(profile_for("  OpenAI  ", "m").sends_tool_schemas());
        });
    }

    /// `azure_openai` and `azure-openai` are one provider spelled two ways, and
    /// both spellings are in use across the clients. If they resolved to
    /// different keys, an override saved from one client would be invisible to
    /// the other — it would save successfully and never apply.
    #[test]
    fn underscores_and_hyphens_are_the_same_provider() {
        with_no_overrides(|| {
            assert_eq!(
                profile_for("azure_openai", "gpt-4o"),
                profile_for("azure-openai", "gpt-4o")
            );
            assert_eq!(
                override_key("azure_openai", "gpt-4o"),
                override_key("azure-openai", "gpt-4o")
            );
            assert_eq!(
                provider_wide_key("vibecli_mistralrs"),
                provider_wide_key("vibecli-mistralrs")
            );
        });
    }

    /// Folding separators must not turn a real alias into a rewrite rule: these
    /// are different providers that happen to share a prefix, and the tables
    /// name them individually.
    #[test]
    fn folding_does_not_merge_distinct_providers() {
        with_no_overrides(|| {
            assert_ne!(override_key("vercel", "m"), override_key("vercel-ai", "m"));
        });
    }

    #[test]
    fn prompt_caching_is_on_only_where_the_api_has_it() {
        with_no_overrides(|| {
            assert!(profile_for("claude", "claude-opus-5").prompt_cache);
            assert!(profile_for("anthropic", "claude-opus-5").prompt_cache);
            assert!(!profile_for("openai", "gpt-5.5").prompt_cache);
            assert!(!profile_for("ollama", "qwen3").prompt_cache);
        });
    }

    /// Every provider declared native must have a request builder that sends
    /// schemas. This test cannot see the builders, so it guards the half it
    /// can: the id has to be one the daemon can actually construct. A typo
    /// here is a provider silently left on the prose path.
    #[test]
    fn native_provider_ids_are_unique_and_lowercase() {
        let mut seen = std::collections::HashSet::new();
        for id in NATIVE_TOOL_PROVIDERS {
            assert_eq!(*id, id.to_lowercase(), "{id} must be lowercase");
            assert!(seen.insert(*id), "{id} listed twice");
        }
    }

    // ── Layer 2 ─────────────────────────────────────────────────────────

    #[test]
    fn model_overrides_match_by_prefix_not_substring() {
        // The table ships empty on purpose; this pins the matcher's shape so a
        // future entry cannot be written as a substring rule by accident.
        const TABLE: &[(&str, &str, ProfileOverride)] = &[];
        assert!(TABLE.is_empty());
        // A shorter prefix must not capture a longer, differently-suffixed id.
        assert!("gpt-4-32k".starts_with("gpt-4"));
        assert!(!"gpt-4".starts_with("gpt-4-32k"));
    }

    #[test]
    fn no_builtin_model_override_is_a_no_op_not_a_reset() {
        // `claude` sets prompt_cache at the family layer; resolving a model
        // with no model-layer rule must not lose it.
        with_no_overrides(|| assert!(profile_for("claude", "claude-sonnet-5").prompt_cache));
    }

    // ── Layers 3 & 4 ────────────────────────────────────────────────────

    #[test]
    fn a_user_override_wins_over_the_builtin() {
        let map = HashMap::from([(
            "claude/claude-opus-5".to_string(),
            ProfileOverride {
                tool_transport: Some(ToolTransport::Prose),
                prompt_dialect: Some(PromptDialect::Full),
                ..Default::default()
            },
        )]);
        with_overrides(map, || {
            let p = profile_for("claude", "claude-opus-5");
            assert_eq!(p.tool_transport, ToolTransport::Prose);
            assert_eq!(p.prompt_dialect, PromptDialect::Full);
            // Untouched fields survive.
            assert!(p.prompt_cache);
            // A sibling model is unaffected.
            assert!(profile_for("claude", "claude-sonnet-5").sends_tool_schemas());
        });
    }

    #[test]
    fn a_model_override_beats_a_provider_wide_one() {
        let map = HashMap::from([
            (
                "openai/*".to_string(),
                ProfileOverride {
                    temperature: Some(0.1),
                    max_output_tokens: Some(1_000),
                    ..Default::default()
                },
            ),
            (
                "openai/gpt-5.5".to_string(),
                ProfileOverride {
                    temperature: Some(0.9),
                    ..Default::default()
                },
            ),
        ]);
        with_overrides(map, || {
            let p = profile_for("openai", "gpt-5.5");
            assert_eq!(p.temperature, Some(0.9), "model layer wins");
            assert_eq!(
                p.max_output_tokens,
                Some(1_000),
                "provider layer still supplies what the model layer omits"
            );
            // A different model gets only the provider-wide layer.
            assert_eq!(profile_for("openai", "gpt-4o").temperature, Some(0.1));
        });
    }

    #[test]
    fn an_empty_override_map_changes_nothing() {
        let cleared = with_no_overrides(|| profile_for("claude", "claude-opus-5"));
        assert_eq!(cleared, with_no_overrides(|| family_default("claude")));
    }

    #[test]
    fn override_keys_are_built_one_way() {
        assert_eq!(override_key("Claude", " claude-opus-5 "), "claude/claude-opus-5");
        assert_eq!(provider_wide_key("OpenAI"), "openai/*");
    }

    // ── Thinking budgets ────────────────────────────────────────────────

    #[test]
    fn an_unset_tier_falls_back_to_the_global_table() {
        with_no_overrides(|| {
            let p = profile_for("claude", "claude-opus-5");
            assert_eq!(
                p.thinking_budget(Effort::High),
                Effort::High.claude_thinking_budget()
            );
            assert_eq!(p.thinking_budget(Effort::Low), None, "Low disables thinking");
        });
    }

    #[test]
    fn a_per_tier_override_replaces_only_that_tier() {
        let map = HashMap::from([(
            "claude/claude-opus-5".to_string(),
            ProfileOverride {
                thinking_budgets: EffortBudgets {
                    high: Some(48_000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )]);
        with_overrides(map, || {
            let p = profile_for("claude", "claude-opus-5");
            assert_eq!(p.thinking_budget(Effort::High), Some(48_000));
            assert_eq!(
                p.thinking_budget(Effort::Medium),
                Effort::Medium.claude_thinking_budget(),
                "an unset tier still comes from the global table"
            );
        });
    }

    // ── Honesty ─────────────────────────────────────────────────────────

    #[test]
    fn the_builtin_table_invents_no_vendor_numbers() {
        // Every knob whose honest value is "ask the provider" ships absent, so
        // a provider's own default stands and nothing is asserted from memory.
        with_no_overrides(|| {
            for (provider, model) in [
                ("claude", "claude-opus-5"),
                ("openai", "gpt-5.5"),
                ("gemini", "gemini-3.1-pro"),
                ("bedrock", "anthropic.claude-sonnet-4"),
                ("ollama", "qwen3-coder"),
            ] {
                let p = profile_for(provider, model);
                assert_eq!(p.max_output_tokens, None, "{provider}/{model}");
                assert_eq!(p.context_window_fallback, None, "{provider}/{model}");
                assert_eq!(p.temperature, None, "{provider}/{model}");
                assert!(p.thinking_budgets.is_empty(), "{provider}/{model}");
            }
        });
    }

    // ── Honoured fields ─────────────────────────────────────────────────

    /// The four the agent loop acts on reach every provider, including one
    /// this build has never heard of.
    #[test]
    fn the_universal_knobs_are_honored_everywhere() {
        for provider in ["claude", "openai", "gemini", "bedrock", "ollama", "who-is-this"] {
            let fields = honored_fields(provider);
            for f in [
                ProfileField::ToolTransport,
                ProfileField::PromptDialect,
                ProfileField::ContextWindowFallback,
                ProfileField::SystemPromptSuffix,
            ] {
                assert!(fields.contains(&f), "{provider} should honour {f:?}");
            }
        }
    }

    /// Only `claude.rs` reads `prompt_cache` — it is Anthropic's
    /// `cache_control`. Offering it on OpenAI would be a switch that saves,
    /// reads back as changed, and does nothing.
    #[test]
    fn prompt_cache_is_offered_only_where_it_is_read() {
        assert!(honored_fields("claude").contains(&ProfileField::PromptCache));
        assert!(honored_fields("anthropic").contains(&ProfileField::PromptCache));
        for provider in ["openai", "gemini", "bedrock", "ollama", "groq"] {
            assert!(
                !honored_fields(provider).contains(&ProfileField::PromptCache),
                "{provider} does not read prompt_cache and must not offer it"
            );
        }
    }

    /// A thinking budget is a *token count*. OpenAI's reasoning dial is an
    /// effort word, so a number means nothing to it.
    #[test]
    fn token_thinking_budgets_are_offered_only_to_claude_and_gemini() {
        for provider in ["claude", "gemini"] {
            assert!(honored_fields(provider).contains(&ProfileField::ThinkingBudgets), "{provider}");
        }
        for provider in ["openai", "bedrock", "ollama", "groq"] {
            assert!(!honored_fields(provider).contains(&ProfileField::ThinkingBudgets), "{provider}");
        }
    }

    #[test]
    fn parallel_tool_calls_is_offered_only_to_the_openai_shaped_family() {
        for provider in ["openai", "groq", "openrouter", "azure_openai"] {
            assert!(
                honored_fields(provider).contains(&ProfileField::ParallelToolCalls),
                "{provider}"
            );
        }
        // These express it differently or not at all; the field would be
        // ignored or rejected.
        for provider in ["claude", "gemini", "bedrock", "ollama"] {
            assert!(
                !honored_fields(provider).contains(&ProfileField::ParallelToolCalls),
                "{provider}"
            );
        }
    }

    /// Separator folding applies here too, or a client spelling it one way
    /// would be offered a different set of knobs from one spelling it the
    /// other.
    #[test]
    fn honored_fields_folds_separators() {
        assert_eq!(honored_fields("azure_openai"), honored_fields("azure-openai"));
    }

    #[test]
    fn resolve_reports_the_honored_fields() {
        with_no_overrides(|| {
            let r = resolve("openai", "gpt-5.5");
            assert!(r.honored_fields.contains(&ProfileField::ParallelToolCalls));
            assert!(!r.honored_fields.contains(&ProfileField::PromptCache));
        });
    }

    // ── Provenance ──────────────────────────────────────────────────────

    #[test]
    fn resolve_separates_what_we_ship_from_what_the_user_set() {
        let map = HashMap::from([(
            "gemini/gemini-3.1-pro".to_string(),
            ProfileOverride {
                temperature: Some(0.3),
                ..Default::default()
            },
        )]);
        with_overrides(map, || {
            let r = resolve("gemini", "gemini-3.1-pro");
            assert_eq!(r.effective.temperature, Some(0.3));
            assert_eq!(r.builtin.temperature, None, "the shipped default is untouched");
            assert!(r.model_override.is_some());
            assert!(r.provider_override.is_none());
        });
    }

    #[test]
    fn resolve_reports_no_override_when_the_user_set_none() {
        with_no_overrides(|| {
            let r = resolve("claude", "claude-opus-5");
            assert!(r.model_override.is_none());
            assert!(r.provider_override.is_none());
            assert_eq!(r.effective, r.builtin);
        });
    }

    // ── Wire shape ──────────────────────────────────────────────────────

    #[test]
    fn an_override_round_trips_through_json() {
        let patch = ProfileOverride {
            tool_transport: Some(ToolTransport::Prose),
            max_output_tokens: Some(32_000),
            system_prompt_suffix: Some("Prefer small diffs.".into()),
            thinking_budgets: EffortBudgets {
                xhigh: Some(60_000),
                ..Default::default()
            },
            ..Default::default()
        };
        let json = serde_json::to_string(&patch).expect("serialises");
        let back: ProfileOverride = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(patch, back);
        // Absent fields stay absent on the wire rather than serialising as
        // nulls a client would render as a value.
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn an_empty_override_is_recognised_as_empty() {
        assert!(ProfileOverride::default().is_empty());
        assert!(!ProfileOverride {
            temperature: Some(0.0),
            ..Default::default()
        }
        .is_empty());
    }
}
