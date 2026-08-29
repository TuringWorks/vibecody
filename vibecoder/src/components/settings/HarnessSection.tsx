/**
 * HarnessSection — per-(provider, model) harness tuning.
 *
 * The harness is everything a model is *given* that is not the conversation:
 * whether tool schemas ride on the wire or are described in prose, which system
 * prompt is paired with that choice, the output cap, the reasoning budget, and
 * any per-model instructions.
 *
 * Every one of those was a single global before, which cost the strongest
 * models the most. This panel is the tuning half: the daemon ships defaults it
 * can defend, and anything it cannot honestly assert about a vendor's product
 * — output caps, context windows — is left absent for whoever measured it.
 *
 * ## Why the panel shows two values per field
 *
 * A field renders what the harness will actually use, and separately whether
 * that came from this build or from the user. Without the distinction "reset"
 * has nothing to reset to, and a default that later improves looks like a bug.
 * The daemon returns both — `effective` and `builtin` — from one request.
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { RotateCcw, Loader2, AlertCircle, Check } from "lucide-react";
import { daemonFetch } from "../../lib/daemonFetch";
import {
  ALL_PROVIDERS,
  PROVIDER_DEFAULT_MODEL,
  useModelRegistry,
} from "../../hooks/useModelRegistry";

const DAEMON = "http://localhost:7878";

/** Addresses every model a provider serves. */
const ALL_MODELS = "*";

export type ToolTransport = "native" | "prose";
export type PromptDialect = "full" | "compact";

export interface EffortBudgets {
  low?: number;
  medium?: number;
  high?: number;
  xhigh?: number;
}

/** The resolved settings for one pair. Mirrors `vibe_ai::harness::ModelProfile`. */
export interface ModelProfile {
  tool_transport: ToolTransport;
  prompt_dialect: PromptDialect;
  max_output_tokens?: number;
  temperature?: number;
  parallel_tool_calls?: boolean;
  thinking_budgets?: EffortBudgets;
  prompt_cache: boolean;
  context_window_fallback?: number;
  system_prompt_suffix?: string;
}

/** A patch — only the fields the user changed. */
export type ProfileOverride = Partial<Omit<ModelProfile, "prompt_cache">> & {
  prompt_cache?: boolean;
};

export interface ResolvedProfile {
  provider: string;
  model: string;
  effective: ModelProfile;
  builtin: ModelProfile;
  provider_override?: ProfileOverride;
  model_override?: ProfileOverride;
}

/**
 * `unknown` from the wire, narrowed here rather than cast.
 *
 * A cast tells the compiler a shape it never checked, and the crash lands far
 * from the response that caused it. Anything that fails these checks is
 * reported as an unreadable response, which is what it is.
 */
function asResolvedProfile(value: unknown): ResolvedProfile | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  const okProfile = (p: unknown): p is ModelProfile =>
    typeof p === "object" &&
    p !== null &&
    typeof (p as Record<string, unknown>).tool_transport === "string" &&
    typeof (p as Record<string, unknown>).prompt_dialect === "string";
  if (typeof v.provider !== "string" || typeof v.model !== "string") return null;
  if (!okProfile(v.effective) || !okProfile(v.builtin)) return null;
  return {
    provider: v.provider,
    model: v.model,
    effective: v.effective,
    builtin: v.builtin,
    provider_override: v.provider_override as ProfileOverride | undefined,
    model_override: v.model_override as ProfileOverride | undefined,
  };
}

/** What this panel is doing, as one value rather than parallel flags. */
type Status =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

export function HarnessSection() {
  const registry = useModelRegistry();
  const [provider, setProvider] = useState<string>("claude");
  const [model, setModel] = useState<string>(ALL_MODELS);
  const [resolved, setResolved] = useState<ResolvedProfile | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  const models = useMemo(() => {
    const list = registry.modelsForProvider(provider);
    const preferred = PROVIDER_DEFAULT_MODEL[provider];
    // The provider-wide entry first: it is the setting most people want, and
    // it is the only one that applies to a model this build has never listed.
    return [ALL_MODELS, ...(preferred && !list.includes(preferred) ? [preferred] : []), ...list];
  }, [registry, provider]);

  const load = useCallback(async (p: string, m: string) => {
    setStatus({ kind: "loading" });
    try {
      const params = new URLSearchParams({ provider: p, model: m });
      const res = await daemonFetch(`${DAEMON}/harness/profile?${params}`);
      if (!res.ok) {
        setStatus({ kind: "error", message: `Daemon returned ${res.status}` });
        return;
      }
      const parsed = asResolvedProfile(await res.json());
      if (!parsed) {
        setStatus({ kind: "error", message: "Unreadable response from the daemon" });
        return;
      }
      setResolved(parsed);
      setStatus({ kind: "idle" });
    } catch {
      // No daemon is the ordinary case before autostart finishes. Say so
      // rather than rendering defaults that are not what anything will use.
      setStatus({ kind: "error", message: "Could not reach the VibeCLI daemon" });
    }
  }, []);

  useEffect(() => {
    void load(provider, model);
  }, [provider, model, load]);

  /** The user's patch for whatever is selected. */
  const override: ProfileOverride = useMemo(
    () => (model === ALL_MODELS ? resolved?.provider_override : resolved?.model_override) ?? {},
    [resolved, model]
  );

  const write = useCallback(
    async (patch: ProfileOverride) => {
      setStatus({ kind: "saving" });
      try {
        const params = new URLSearchParams({ provider, model });
        const empty = Object.values(patch).every((v) => v === undefined);
        const res = await daemonFetch(`${DAEMON}/harness/profile?${params}`, {
          // An empty patch is a delete, so the pair returns to whatever this
          // build ships — including a default that improves later.
          method: empty ? "DELETE" : "PUT",
          headers: { "content-type": "application/json" },
          ...(empty ? {} : { body: JSON.stringify(patch) }),
        });
        if (!res.ok) {
          setStatus({ kind: "error", message: `Daemon returned ${res.status}` });
          return;
        }
        const parsed = asResolvedProfile(await res.json());
        if (parsed) setResolved(parsed);
        setStatus({ kind: "saved" });
      } catch {
        setStatus({ kind: "error", message: "Could not reach the VibeCLI daemon" });
      }
    },
    [provider, model]
  );

  const set = useCallback(
    <K extends keyof ProfileOverride>(key: K, value: ProfileOverride[K]) =>
      void write({ ...override, [key]: value }),
    [override, write]
  );

  const effective = resolved?.effective;
  const overridden = (key: keyof ModelProfile) => override[key as keyof ProfileOverride] !== undefined;

  return (
    <div style={{ maxWidth: 720 }}>
      <h3 style={{ marginTop: 0, color: "var(--text-primary)" }}>Model Harness</h3>
      <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", lineHeight: 1.6 }}>
        What a model is <em>given</em> — tool schemas or a prose catalogue, which system
        prompt goes with that, the output cap and the reasoning budget. Defaults are per
        provider; anything you set here applies to the selected pair only.
      </p>

      {/* Pair selector */}
      <div style={{ display: "flex", gap: 12, margin: "16px 0" }}>
        <label style={labelStyle}>
          Provider
          <select
            className="panel-input"
            value={provider}
            onChange={(e) => {
              setProvider(e.target.value);
              setModel(ALL_MODELS);
            }}
            style={inputStyle}
          >
            {ALL_PROVIDERS.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>
        <label style={labelStyle}>
          Model
          <select
            className="panel-input"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            style={inputStyle}
          >
            {models.map((m) => (
              <option key={m} value={m}>
                {m === ALL_MODELS ? "All models (provider default)" : m}
              </option>
            ))}
          </select>
        </label>
      </div>

      <StatusLine status={status} />

      {effective && (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <Row
            label="Tool transport"
            help="Native sends the tool schemas on the wire. Prose describes them in the system prompt and parses calls out of the reply — the escape hatch for a model whose native tool calling is worse than its prose."
            overridden={overridden("tool_transport")}
            onReset={() => set("tool_transport", undefined)}
          >
            <select
              className="panel-input"
              value={effective.tool_transport}
              onChange={(e) => set("tool_transport", e.target.value as ToolTransport)}
              style={inputStyle}
            >
              <option value="native">Native schemas</option>
              <option value="prose">Prose catalogue</option>
            </select>
          </Row>

          <Row
            label="System prompt"
            help="Compact drops the per-tool XML catalogue — about 4,000 tokens on every turn — which a model receiving the schemas does not also need."
            overridden={overridden("prompt_dialect")}
            onReset={() => set("prompt_dialect", undefined)}
          >
            <select
              className="panel-input"
              value={effective.prompt_dialect}
              onChange={(e) => set("prompt_dialect", e.target.value as PromptDialect)}
              style={inputStyle}
            >
              <option value="compact">Compact</option>
              <option value="full">Full catalogue</option>
            </select>
          </Row>

          <Row
            label="Prompt caching"
            help="Asks the API to cache the system prefix where it supports it. The agent's system prompt is thousands of tokens and is resent on every turn."
            overridden={overridden("prompt_cache")}
            onReset={() => set("prompt_cache", undefined)}
          >
            <input
              type="checkbox"
              checked={effective.prompt_cache}
              onChange={(e) => set("prompt_cache", e.target.checked)}
            />
          </Row>

          <Row
            label="Max output tokens"
            help="Left empty on purpose: a cap written from memory is a claim about someone else's product. Empty means the provider's own default stands."
            overridden={overridden("max_output_tokens")}
            onReset={() => set("max_output_tokens", undefined)}
          >
            <NumberInput
              label="Max output tokens"
              value={effective.max_output_tokens}
              placeholder="provider default"
              onCommit={(n) => set("max_output_tokens", n)}
            />
          </Row>

          <Row
            label="Context window fallback"
            help="Used only when the provider's API does not publish this model's window. It never overrides a number the API actually reported."
            overridden={overridden("context_window_fallback")}
            onReset={() => set("context_window_fallback", undefined)}
          >
            <NumberInput
              label="Context window fallback"
              value={effective.context_window_fallback}
              placeholder="ask the provider"
              onCommit={(n) => set("context_window_fallback", n)}
            />
          </Row>

          <Row
            label="Temperature"
            help="Empty means the provider's own default."
            overridden={overridden("temperature")}
            onReset={() => set("temperature", undefined)}
          >
            <NumberInput
              label="Temperature"
              value={effective.temperature}
              placeholder="provider default"
              step={0.1}
              onCommit={(n) => set("temperature", n)}
            />
          </Row>

          <Row
            label="Model instructions"
            help="Appended to the agent system prompt for this pair only, after everything else — so a per-model reminder can correct the general prompt rather than be corrected by it."
            overridden={overridden("system_prompt_suffix")}
            onReset={() => set("system_prompt_suffix", undefined)}
          >
            <textarea
              className="panel-input"
              rows={3}
              defaultValue={effective.system_prompt_suffix ?? ""}
              placeholder="none"
              onBlur={(e) => {
                const text = e.target.value.trim();
                if (text === (effective.system_prompt_suffix ?? "")) return;
                set("system_prompt_suffix", text === "" ? undefined : text);
              }}
              style={{ ...inputStyle, width: 320, resize: "vertical" }}
            />
          </Row>
        </div>
      )}
    </div>
  );
}

function StatusLine({ status }: { status: Status }) {
  switch (status.kind) {
    case "loading":
      return (
        <p style={noteStyle}>
          <Loader2 size={14} className="spin" /> Reading the profile…
        </p>
      );
    case "saving":
      return (
        <p style={noteStyle}>
          <Loader2 size={14} className="spin" /> Saving…
        </p>
      );
    case "saved":
      return (
        <p style={{ ...noteStyle, color: "var(--success-color, #3fb950)" }}>
          <Check size={14} /> Saved
        </p>
      );
    case "error":
      return (
        <p style={{ ...noteStyle, color: "var(--error-color, #f85149)" }}>
          <AlertCircle size={14} /> {status.message}
        </p>
      );
    case "idle":
      return null;
    default: {
      // Exhaustive: a new status has to be handled here, not fall through as
      // a blank line.
      const never: never = status;
      return never;
    }
  }
}

function Row({
  label,
  help,
  overridden,
  onReset,
  children,
}: {
  label: string;
  help: string;
  overridden: boolean;
  onReset: () => void;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "10px 0",
        borderBottom: "1px solid var(--border-color)",
      }}
    >
      <div style={{ flex: 1 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ color: "var(--text-primary)", fontWeight: 600 }}>{label}</span>
          {overridden && (
            <span
              style={{
                fontSize: "var(--font-size-xs)",
                color: "var(--accent-color)",
                border: "1px solid var(--accent-color)",
                borderRadius: "var(--radius-sm)",
                padding: "0 6px",
              }}
            >
              changed
            </span>
          )}
        </div>
        <div
          style={{
            color: "var(--text-secondary)",
            fontSize: "var(--font-size-xs)",
            lineHeight: 1.5,
            marginTop: 2,
          }}
        >
          {help}
        </div>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        {children}
        <button
          className="panel-btn"
          onClick={onReset}
          disabled={!overridden}
          title={overridden ? "Reset to the shipped default" : "Already the shipped default"}
          aria-label={`Reset ${label}`}
          style={{
            background: "none",
            border: "none",
            cursor: overridden ? "pointer" : "default",
            opacity: overridden ? 1 : 0.25,
            color: "var(--text-secondary)",
          }}
        >
          <RotateCcw size={14} />
        </button>
      </div>
    </div>
  );
}

/**
 * A number field where empty means *absent*, not zero.
 *
 * The distinction is the whole point of these knobs: an absent cap means the
 * provider decides, and rendering that as `0` would assert a limit nobody set.
 */
function NumberInput({
  label,
  value,
  placeholder,
  step,
  onCommit,
}: {
  /** Accessible name. Two fields can legitimately share a placeholder
   *  ("provider default"), so the placeholder cannot be the name. */
  label: string;
  value?: number;
  placeholder: string;
  step?: number;
  onCommit: (value: number | undefined) => void;
}) {
  return (
    <input
      className="panel-input"
      aria-label={label}
      type="number"
      step={step}
      defaultValue={value ?? ""}
      placeholder={placeholder}
      onBlur={(e) => {
        const raw = e.target.value.trim();
        if (raw === "") {
          if (value !== undefined) onCommit(undefined);
          return;
        }
        const parsed = Number(raw);
        // A field that will not parse is left alone rather than committed as
        // NaN or silently coerced to zero.
        if (!Number.isFinite(parsed) || parsed === value) return;
        onCommit(parsed);
      }}
      style={{ ...inputStyle, width: 140 }}
    />
  );
}

const labelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 4,
  color: "var(--text-secondary)",
  fontSize: "var(--font-size-sm)",
};

const inputStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-sm)",
  padding: "6px 8px",
};

const noteStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  color: "var(--text-secondary)",
  fontSize: "var(--font-size-sm)",
  margin: "8px 0",
};

export default HarnessSection;
