/**
 * Per-request reasoning / compute effort tier (gap C5).
 *
 * Provider-agnostic: the backend maps the selected tier onto each provider's
 * native knob (Claude/Gemini extended-thinking budget, OpenAI reasoning_effort,
 * or an output-token cap for open models). The toolbar selector in App.tsx is
 * the single source of truth; any panel that calls an LLM reads the current
 * selection via `getSelectedEffort()` and forwards it as the `effort` argument
 * to effort-aware Tauri commands (e.g. `ai_chat_with_effort`).
 */
export const EFFORT_LEVELS = ["low", "medium", "high", "xhigh"] as const;
export type EffortLevel = (typeof EFFORT_LEVELS)[number];

/** Default tier — matches the C5 spec and the Rust `Effort::default()`. */
export const DEFAULT_EFFORT: EffortLevel = "high";

const STORAGE_KEY = "vibecody:selected-effort";

/** Read the current effort selection (synchronous; safe in render). */
export function getSelectedEffort(): EffortLevel {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v && (EFFORT_LEVELS as readonly string[]).includes(v)) {
      return v as EffortLevel;
    }
  } catch {
    // localStorage unavailable
  }
  return DEFAULT_EFFORT;
}

/** Persist the effort selection. */
export function setSelectedEffort(level: EffortLevel): void {
  try {
    localStorage.setItem(STORAGE_KEY, level);
  } catch {
    // localStorage unavailable — selection is session-only
  }
}

/** Human-friendly label for a tier (for tooltips / menus). */
export function effortLabel(level: EffortLevel): string {
  switch (level) {
    case "low":
      return "Low — fastest, least reasoning";
    case "medium":
      return "Medium — balanced";
    case "high":
      return "High — deeper reasoning (default)";
    case "xhigh":
      return "X-High — maximum compute budget";
  }
}
