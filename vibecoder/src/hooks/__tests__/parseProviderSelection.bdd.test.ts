import { describe, it, expect } from "vitest";
import { parseProviderSelection, PROVIDER_DEFAULT_MODEL } from "../useModelRegistry";

/**
 * The toolbar's provider dropdown is filled from `get_available_ai_providers`,
 * which returns the chat engine's *display names* — `"<Label> (<model>)"`.
 * Panels, meanwhile, key off provider ids (`"ollama"`), because that is what
 * the backend's `build_temp_provider` matches on.
 *
 * Nothing bridged the two. `PROVIDER_DEFAULT_MODEL["Ollama (gpt-oss:120b-cloud)"]`
 * missed, the miss produced an empty model, and an empty model is how every
 * LLM surface spells "nothing selected" — so ghost text answered ⌥\ with
 * "Select a provider and model in the toolbar first" while the toolbar showed
 * a provider *and* a model. These tests pin the bridge.
 */
describe("parseProviderSelection", () => {
  it("keeps the model the user picked, not the registry default", () => {
    expect(parseProviderSelection("Ollama (gpt-oss:120b-cloud)")).toEqual({
      provider: "ollama",
      model: "gpt-oss:120b-cloud",
    });
    // The whole point: the picked model is not the default one.
    expect(PROVIDER_DEFAULT_MODEL.ollama).not.toBe("gpt-oss:120b-cloud");
  });

  it("maps every multi-word and cased label the engine can emit", () => {
    const cases: Array<[string, string]> = [
      ["Claude (claude-opus-5)", "claude"],
      ["OpenAI (gpt-4o)", "openai"],
      ["OpenRouter (anthropic/claude-opus-5)", "openrouter"],
      ["AzureOpenAI (gpt-4o)", "azure_openai"],
      ["Fireworks AI (accounts/fireworks/models/x)", "fireworks"],
      ["Together AI (moonshotai/Kimi-K2.7-Code)", "together"],
      ["VibeCLI mistralrs (meta-llama/Llama-3.1-8B-Instruct)", "vibecli-mistralrs"],
      ["VercelAI (gpt-4o)", "vercel_ai"],
      ["SambaNova (Meta-Llama-3.3-70B-Instruct)", "sambanova"],
      ["MiniMax (MiniMax-M3)", "minimax"],
    ];
    for (const [display, provider] of cases) {
      expect(parseProviderSelection(display).provider).toBe(provider);
    }
  });

  it("resolves a bare provider id through the registry default", () => {
    expect(parseProviderSelection("claude")).toEqual({
      provider: "claude",
      model: PROVIDER_DEFAULT_MODEL.claude,
    });
  });

  it("reports an empty selection as empty so callers keep their empty state", () => {
    expect(parseProviderSelection("")).toEqual({ provider: "", model: "" });
    expect(parseProviderSelection("   ")).toEqual({ provider: "", model: "" });
  });

  it("passes an unrecognised label through instead of guessing a provider", () => {
    // A wrong provider would silently send the user's code to a vendor they
    // did not choose; an unresolvable one surfaces as the empty state.
    expect(parseProviderSelection("Nonesuch (some-model)")).toEqual({
      provider: "Nonesuch (some-model)",
      model: "",
    });
  });

  it("falls back to the registry default when a label carries no model", () => {
    expect(parseProviderSelection("Ollama ()")).toEqual({
      provider: "ollama",
      model: PROVIDER_DEFAULT_MODEL.ollama,
    });
  });
});
