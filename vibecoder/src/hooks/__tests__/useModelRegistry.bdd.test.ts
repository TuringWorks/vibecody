import { describe, it, expect } from "vitest";
import { STATIC_MODELS, PROVIDER_DEFAULT_MODEL } from "../useModelRegistry";
import { OLLAMA_CHAT_MODELS, OLLAMA_CLOUD_MODELS } from "../../constants/ollamaModels";

/**
 * Registry integrity — the guard for R1.
 *
 * On 2026-05-19 Google announced Gemini 3.5 Pro; a refresh wrote it into this
 * registry as both a listed model and the Gemini *default* on the strength of
 * a projected GA date. It never shipped — three delays, and as of August 2026
 * still a limited Vertex AI preview. For weeks every user who selected the
 * Gemini provider got a model id the API rejects on their first call: a
 * Zero-Config First violation (AGENTS.md) caused by a forecast in the code.
 *
 * It was found twice, filed as "fix first — one line" twice, and survived both
 * times. A one-line fix with no owner and no test is not scheduled work, so
 * the close is the fix *plus* this file.
 *
 * These tests pin structure, not taste: they cannot know whether a model id is
 * real, but they can guarantee the registry never offers a default it does not
 * also list — which is the exact shape the phantom took.
 */
describe("model registry integrity", () => {
  /**
   * Providers that ship no static model list at all. Kept as an explicit
   * roster rather than a blanket skip so the set can only shrink by decision:
   * adding a new empty provider fails here, and fixing one of these also fails
   * here (delete the entry — that failure is the good kind).
   *
   * `ollama` is deliberately *not* here: it ships a real static list
   * (`OLLAMA_CHAT_MODELS`) that the daemon extends at runtime, so it satisfies
   * both checks without an exemption.
   *
   * - `vercel_ai` — NOT intentional. `STATIC_MODELS.vercel_ai` is `[]` and its
   *               default is `""`, with no runtime fetch anywhere: the picker
   *               offers a provider that can produce no model. Found by this
   *               test on 2026-08-10. Left listed rather than "fixed", because
   *               inventing model ids is exactly the failure this file exists
   *               to prevent — it needs someone who knows the Vercel AI
   *               Gateway catalogue, or removal from the registry.
   */
  const NO_STATIC_LIST = new Set(["vercel_ai"]);

  it("every provider that lists models defaults to one of them", () => {
    const offenders = Object.entries(PROVIDER_DEFAULT_MODEL)
      .filter(([provider]) => !NO_STATIC_LIST.has(provider))
      .filter(([provider, def]) => !(STATIC_MODELS[provider] ?? []).includes(def))
      .map(([provider, def]) => `${provider} → "${def}"`);

    expect(
      offenders,
      `A provider defaults to a model it does not list, so selecting that ` +
        `provider fails on first call. Offenders: ${offenders.join(", ")}`,
    ).toEqual([]);
  });

  it("the set of providers without a model list has not grown", () => {
    const actual = Object.keys(PROVIDER_DEFAULT_MODEL)
      .filter((p) => !STATIC_MODELS[p]?.length)
      .sort();

    expect(
      actual,
      `A provider offering no models is unusable once selected. If you fixed ` +
        `one, remove it from NO_STATIC_LIST above; if you added one, give it models.`,
    ).toEqual([...NO_STATIC_LIST].sort());
  });

  it("the ollama default is a known local model", () => {
    // Ollama passes the checks above on its static list, but that list is
    // extended at runtime from the local daemon — so also pin the
    // *pre-selected* value against the full known set, cloud names included.
    const known = [...OLLAMA_CHAT_MODELS, ...OLLAMA_CLOUD_MODELS];
    expect(known).toContain(PROVIDER_DEFAULT_MODEL.ollama);
  });

  it("no model id is listed twice within a provider", () => {
    const dupes = Object.entries(STATIC_MODELS)
      .map(([provider, models]) => {
        const seen = new Set<string>();
        const repeated = models.filter((m) => seen.size === seen.add(m).size);
        return repeated.length ? `${provider}: ${repeated.join(", ")}` : null;
      })
      .filter((x): x is string => x !== null);

    expect(dupes, `Duplicate model ids: ${dupes.join(" | ")}`).toEqual([]);
  });

  it("does not offer gemini-3.5-pro, which has never shipped", () => {
    // Named explicitly rather than left to the structural checks above: this
    // model was re-added once already after being identified as absent, and a
    // structural test cannot catch a phantom that is *listed* consistently.
    // Delete this case on the day Google actually ships it.
    const everywhere = Object.values(STATIC_MODELS).flat();
    expect(everywhere).not.toContain("gemini-3.5-pro");
    expect(Object.values(PROVIDER_DEFAULT_MODEL)).not.toContain("gemini-3.5-pro");
  });
});
