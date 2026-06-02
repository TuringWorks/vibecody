import { useEffect, useState } from "react";
import { ChevronUp } from "lucide-react";
import { useModels } from "../hooks/useModels";

interface ProviderPillProps {
  daemonUrl: string;
  daemonOnline: boolean;
  provider: string;
  model?: string;
  onProvider: (p: string) => void;
  onModel: (m: string | undefined) => void;
}

/**
 * VX-106 — provider/model selector (Codex screenshots 1, 7), backed by the
 * daemon's live `/models` list (the source of truth for what's installed).
 * Selecting a model sets BOTH provider and model so the daemon never falls
 * back to its picker default (which can be a flaky cloud model). The pill
 * shows the chosen model name.
 */
export function ProviderPill({ daemonUrl, daemonOnline, provider, model, onProvider, onModel }: ProviderPillProps) {
  const [open, setOpen] = useState(false);
  const models = useModels(daemonUrl, daemonOnline);

  // Auto-pick a sensible default once models load and nothing is selected yet.
  // Prefer a LOCAL model (avoids the daemon's cloud picker-default, which can
  // return transient 500s); fall back to the first model of any kind.
  useEffect(() => {
    if (model || models.length === 0) return;
    const isCloud = (n: string) => n.includes("-cloud") || n.includes("cloud");
    const pick = models.find((m) => !isCloud(m.name!)) ?? models[0];
    if (pick?.name) {
      onProvider(pick.provider);
      onModel(pick.name);
    }
  }, [models, model, onProvider, onModel]);

  // Group models by provider for a readable menu.
  const byProvider = new Map<string, { name: string }[]>();
  for (const m of models) {
    const arr = byProvider.get(m.provider) ?? [];
    arr.push({ name: m.name! });
    byProvider.set(m.provider, arr);
  }

  const label = model ? model : `${provider} (default)`;

  return (
    <div className="vx-pill-wrap">
      {open && (
        <ul className="vx-pill-menu vx-pill-menu--models" role="menu">
          {models.length === 0 && <li className="vx-pill-menu__empty">No models reported by daemon</li>}
          {[...byProvider.entries()].map(([prov, list]) => (
            <li key={prov}>
              <div className="vx-pill-menu__group">{prov}</div>
              {list.map((m) => (
                <button
                  key={`${prov}/${m.name}`}
                  role="menuitemradio"
                  aria-checked={provider === prov && model === m.name}
                  className="vx-pill-menu__item"
                  onClick={() => {
                    onProvider(prov);
                    onModel(m.name);
                    setOpen(false);
                  }}
                >
                  {m.name} {provider === prov && model === m.name && "✓"}
                </button>
              ))}
            </li>
          ))}
        </ul>
      )}
      <button className="vx-pill" onClick={() => setOpen((v) => !v)} aria-label="Provider and model" title={label}>
        <span className="vx-pill__model">{label}</span>
        <ChevronUp size={12} />
      </button>
    </div>
  );
}
