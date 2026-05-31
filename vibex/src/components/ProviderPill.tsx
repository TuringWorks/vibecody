import { useState } from "react";
import { ChevronUp } from "lucide-react";

/**
 * VX-106 (interim) — provider/model pill (Codex screenshots 1, 7).
 *
 * The canonical provider/model source is VibeUI's `useModelRegistry`
 * (vibeui/src/hooks/useModelRegistry.ts). To stay self-contained during
 * scaffolding, this lists provider keys statically; VX-106 replaces this
 * with a direct import of `ALL_PROVIDERS` + `useModelRegistry` so VibeX and
 * VibeUI never drift. Do NOT hardcode a model list beyond this stub.
 */
const PROVIDERS = [
  "ollama",
  "openai",
  "anthropic",
  "gemini",
  "grok",
  "groq",
  "vibecli-mistralrs",
];

interface ProviderPillProps {
  provider: string;
  model?: string;
  onProvider: (p: string) => void;
  onModel: (m: string | undefined) => void;
}

export function ProviderPill({ provider, onProvider, onModel }: ProviderPillProps) {
  const [open, setOpen] = useState(false);
  return (
    <div className="vx-pill-wrap">
      {open && (
        <ul className="vx-pill-menu" role="menu">
          {PROVIDERS.map((p) => (
            <li key={p}>
              <button
                role="menuitemradio"
                aria-checked={provider === p}
                className="vx-pill-menu__item"
                onClick={() => {
                  onProvider(p);
                  onModel(undefined); // reset to provider default (VX-106 fills from registry)
                  setOpen(false);
                }}
              >
                {p} {provider === p && "✓"}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button className="vx-pill" onClick={() => setOpen((v) => !v)} aria-label="Provider">
        <span>{provider}</span>
        <ChevronUp size={12} />
      </button>
    </div>
  );
}
