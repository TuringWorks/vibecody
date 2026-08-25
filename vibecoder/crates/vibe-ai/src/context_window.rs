//! What each model can actually hold — asked, not assumed.
//!
//! The agent loop prunes conversation history to a token budget. That budget
//! was one constant, 200 000 tokens, for every model on every provider,
//! because nothing in the product ever called `AgentLoop::with_context_limit`.
//! Both directions of that error are user-visible:
//!
//! * On a model with a **larger** window the loop compacts history it did not
//!   need to lose.
//! * On a **smaller** one the prompt overflows and the provider deals with it
//!   silently. Ollama drops the *front* — the system prompt and the tool
//!   contract go first — so the symptom is not an error, it is a model that
//!   appears to forget how to call tools and starts emitting malformed markup.
//!
//! # Why nothing here is a table of numbers
//!
//! A hardcoded window per model id is a fact about someone else's product
//! written from memory, and it is wrong the moment a vendor ships a revision.
//! Every value here is read from the provider's own API:
//!
//! | source                        | field                                    |
//! |-------------------------------|------------------------------------------|
//! | OpenAI-compatible `/models`   | `context_window` / `context_length` / …  |
//! | Ollama `/api/show`            | `model_info["<arch>.context_length"]`    |
//! | Gemini `/v1beta/models/{id}`  | `inputTokenLimit`                        |
//!
//! Providers whose API does not report it — Anthropic and OpenAI among them —
//! answer `None`, and `None` means *unknown*. It is never quietly turned into
//! a number: the caller keeps its own documented default and says so.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Tokens held back for the model's reply, as a fraction of the window.
///
/// The window covers prompt *and* completion. Budgeting the whole thing for
/// history leaves no room to answer, and the provider rejects the request or
/// truncates the reply — which is the same class of failure this module
/// exists to remove, arrived at from the other side.
const OUTPUT_RESERVE_FRACTION: f64 = 0.25;

/// Floor on the reserve, for windows small enough that a fraction of them is
/// not a usable reply.
const MIN_OUTPUT_RESERVE: usize = 4_096;

/// How much of a `window`-token context may hold conversation history.
///
/// Saturates at zero rather than underflowing: a window smaller than the
/// reserve has no room for history at all, and saying so is more useful than
/// wrapping to `usize::MAX`.
pub fn budget_for(window: usize) -> usize {
    let reserve = MIN_OUTPUT_RESERVE.max((window as f64 * OUTPUT_RESERVE_FRACTION) as usize);
    window.saturating_sub(reserve)
}

/// Field names vendors use for the same number in an OpenAI-compatible
/// `/models` listing. Groq says `context_window`, OpenRouter and Together say
/// `context_length`, Mistral says `max_context_length`, vLLM says
/// `max_model_len`. Reading only one of them silently answers `None` for the
/// rest, which is indistinguishable from a provider that does not report it.
const WINDOW_KEYS: &[&str] = &[
    "context_window",
    "context_length",
    "max_context_length",
    "max_model_len",
    "max_context_window_tokens",
];

/// Pull `model`'s context window out of an OpenAI-compatible `/models` body.
///
/// Accepts both the wrapped (`{"data": [...]}`) and bare-array shapes, and
/// matches the model id exactly — a prefix match would hand `gpt-4` the window
/// of `gpt-4-32k`.
pub fn from_models_list(body: &serde_json::Value, model: &str) -> Option<usize> {
    let entries = body
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| body.as_array())?;
    let entry = entries.iter().find(|e| {
        e.get("id").and_then(|i| i.as_str()) == Some(model)
            || e.get("name").and_then(|i| i.as_str()) == Some(model)
    })?;
    WINDOW_KEYS
        .iter()
        .find_map(|k| entry.get(*k))
        .and_then(as_positive_usize)
}

/// Pull the context length out of an Ollama `/api/show` body.
///
/// The key is namespaced by architecture — `llama.context_length`,
/// `qwen3.context_length`, `gemma3.context_length` — so it is found by suffix
/// rather than by guessing the architecture from the model name.
pub fn from_ollama_show(body: &serde_json::Value) -> Option<usize> {
    body.get("model_info")?
        .as_object()?
        .iter()
        .find(|(k, _)| k.ends_with(".context_length"))
        .and_then(|(_, v)| as_positive_usize(v))
}

/// Pull `inputTokenLimit` out of a Gemini `models/{id}` body.
pub fn from_gemini_model(body: &serde_json::Value) -> Option<usize> {
    body.get("inputTokenLimit").and_then(as_positive_usize)
}

/// A window is a count, so zero and negatives are absent data, not values.
fn as_positive_usize(v: &serde_json::Value) -> Option<usize> {
    match v {
        serde_json::Value::Number(n) => n.as_u64().filter(|n| *n > 0).map(|n| n as usize),
        // Some gateways serialise the number as a string.
        serde_json::Value::String(s) => s.trim().parse::<usize>().ok().filter(|n| *n > 0),
        _ => None,
    }
}

/// Answers already obtained, keyed by `provider/model`.
///
/// Process-wide because providers are rebuilt per request — a per-instance
/// cache would probe the network on every turn. `None` is cached too: a
/// provider that does not report a window will not start doing so mid-session,
/// and re-asking it every turn is a request per turn for a known answer.
static CACHE: LazyLock<Mutex<HashMap<String, Option<usize>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Look up `provider`/`model`'s window, running `probe` at most once for it.
///
/// Two callers racing on a cold entry may both probe; that is one duplicate
/// request, and it is preferable to holding a lock across an await.
pub async fn cached<F, Fut>(provider: &str, model: &str, probe: F) -> Option<usize>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Option<usize>>,
{
    let key = format!("{provider}/{model}");
    if let Some(hit) = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .copied()
    {
        return hit;
    }
    let found = probe().await;
    match found {
        Some(window) => tracing::info!(provider, model, window, "Model context window resolved"),
        None => tracing::debug!(
            provider,
            model,
            "Provider does not report a context window; the caller's default applies"
        ),
    }
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, found);
    found
}

/// Drop every cached answer. Tests only — the cache is process-wide, so a test
/// that primes it would otherwise decide the result of the next one.
#[cfg(test)]
pub(crate) fn clear_cache() {
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── budget_for ───────────────────────────────────────────────────────────

    #[test]
    fn a_large_window_reserves_a_quarter_for_the_reply() {
        assert_eq!(budget_for(200_000), 150_000);
    }

    #[test]
    fn a_small_window_reserves_the_floor_not_a_fraction() {
        // 25% of 8k is 2k, which is not a usable reply.
        assert_eq!(budget_for(8_192), 8_192 - MIN_OUTPUT_RESERVE);
    }

    /// A window under the reserve has no room for history. Saying "zero" is
    /// honest; wrapping to `usize::MAX` would be catastrophic.
    #[test]
    fn a_window_smaller_than_the_reserve_leaves_no_history_budget() {
        assert_eq!(budget_for(2_048), 0);
    }

    // ── OpenAI-compatible /models ────────────────────────────────────────────

    #[test]
    fn groq_spelling_is_read() {
        let body = json!({"data": [{"id": "llama-3.3-70b", "context_window": 131072}]});
        assert_eq!(from_models_list(&body, "llama-3.3-70b"), Some(131_072));
    }

    #[test]
    fn openrouter_and_together_spelling_is_read() {
        let body = json!({"data": [{"id": "qwen/qwen3", "context_length": 262144}]});
        assert_eq!(from_models_list(&body, "qwen/qwen3"), Some(262_144));
    }

    #[test]
    fn mistral_spelling_is_read() {
        let body = json!({"data": [{"id": "mistral-large-latest", "max_context_length": 128000}]});
        assert_eq!(from_models_list(&body, "mistral-large-latest"), Some(128_000));
    }

    #[test]
    fn a_bare_array_body_is_accepted() {
        let body = json!([{"id": "m", "context_length": 4096}]);
        assert_eq!(from_models_list(&body, "m"), Some(4_096));
    }

    /// A prefix match would hand `gpt-4` the window of `gpt-4-32k`.
    #[test]
    fn the_model_id_must_match_exactly() {
        let body = json!({"data": [{"id": "gpt-4-32k", "context_length": 32768}]});
        assert_eq!(from_models_list(&body, "gpt-4"), None);
    }

    #[test]
    fn a_listing_without_the_field_is_unknown_not_zero() {
        let body = json!({"data": [{"id": "gpt-4o", "object": "model"}]});
        assert_eq!(from_models_list(&body, "gpt-4o"), None);
    }

    #[test]
    fn a_model_absent_from_the_listing_is_unknown() {
        let body = json!({"data": [{"id": "other", "context_length": 8192}]});
        assert_eq!(from_models_list(&body, "mine"), None);
    }

    #[test]
    fn a_stringly_typed_number_is_still_a_number() {
        let body = json!({"data": [{"id": "m", "context_length": "32768"}]});
        assert_eq!(from_models_list(&body, "m"), Some(32_768));
    }

    #[test]
    fn zero_is_absent_data_not_a_window() {
        let body = json!({"data": [{"id": "m", "context_length": 0}]});
        assert_eq!(from_models_list(&body, "m"), None);
    }

    #[test]
    fn a_garbage_body_yields_no_window() {
        assert_eq!(from_models_list(&json!({"error": "nope"}), "m"), None);
        assert_eq!(from_models_list(&json!("not json we expect"), "m"), None);
    }

    // ── Ollama /api/show ─────────────────────────────────────────────────────

    #[test]
    fn ollama_context_length_is_found_under_its_architecture_prefix() {
        let body = json!({
            "model_info": {
                "general.architecture": "qwen3",
                "qwen3.context_length": 40960,
                "qwen3.embedding_length": 5120
            }
        });
        assert_eq!(from_ollama_show(&body), Some(40_960));
    }

    #[test]
    fn a_different_architecture_needs_no_new_code() {
        let body = json!({"model_info": {"gemma3.context_length": 8192}});
        assert_eq!(from_ollama_show(&body), Some(8_192));
    }

    #[test]
    fn an_ollama_body_without_model_info_is_unknown() {
        assert_eq!(from_ollama_show(&json!({"license": "MIT"})), None);
    }

    // ── Gemini ───────────────────────────────────────────────────────────────

    #[test]
    fn gemini_input_token_limit_is_read() {
        let body = json!({"name": "models/gemini-3.6-flash", "inputTokenLimit": 1048576});
        assert_eq!(from_gemini_model(&body), Some(1_048_576));
    }

    // ── cache ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn a_probe_runs_once_per_provider_and_model() {
        clear_cache();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let probe = || async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(1_234)
        };
        assert_eq!(cached("p-once", "m", probe).await, Some(1_234));
        assert_eq!(cached("p-once", "m", probe).await, Some(1_234));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// "This provider does not report a window" is an answer, and re-asking it
    /// every turn is a network request per turn for a result already known.
    #[tokio::test]
    async fn a_negative_answer_is_cached_too() {
        clear_cache();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let probe = || async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            None
        };
        assert_eq!(cached("p-neg", "m", probe).await, None);
        assert_eq!(cached("p-neg", "m", probe).await, None);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn different_models_are_cached_apart() {
        clear_cache();
        assert_eq!(cached("p-sep", "a", || async { Some(1) }).await, Some(1));
        assert_eq!(cached("p-sep", "b", || async { Some(2) }).await, Some(2));
    }
}
