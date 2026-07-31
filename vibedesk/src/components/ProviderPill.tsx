import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronUp } from "lucide-react";
import { useModels } from "../hooks/useModels";

interface ProviderPillProps {
  daemonUrl: string;
  daemonOnline: boolean;
  provider: string;
  model?: string;
  /** Provider and model are chosen together — a model always belongs to one. */
  onSelect: (provider: string, model: string | undefined) => void;
}

/**
 * VX-106 — provider/model selector (Codex screenshots 1, 7), backed by the
 * daemon's live `/models` list (the source of truth for what's installed).
 * Selecting a model sets BOTH provider and model so the daemon never falls
 * back to its picker default (which can be a flaky cloud model). The pill
 * shows the chosen model name.
 *
 * The list is long enough (every configured provider's catalog) that it needs a
 * filter box and a click-away close to be usable; picking a model out of a
 * hundred-row menu with no search was one of the sharper edges here.
 */
export function ProviderPill({ daemonUrl, daemonOnline, provider, model, onSelect }: ProviderPillProps) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const wrapRef = useRef<HTMLDivElement>(null);
  const models = useModels(daemonUrl, daemonOnline);

  // Auto-pick a sensible default once models load and nothing is selected yet.
  // Prefer a LOCAL model (avoids the daemon's cloud picker-default, which can
  // return transient 500s); fall back to the first model of any kind. A model
  // restored from settings counts as "selected", so this never overrides it.
  useEffect(() => {
    if (model || models.length === 0) return;
    const pick = models.find((m) => !m.name!.includes("cloud")) ?? models[0];
    if (pick?.name) onSelect(pick.provider, pick.name);
  }, [models, model, onSelect]);

  // Close on click-away / Escape — a menu that only closes by re-clicking the
  // pill traps clicks meant for the conversation behind it.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  // Group models by provider for a readable menu, honoring the filter.
  const groups = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    const byProvider = new Map<string, string[]>();
    for (const m of models) {
      const name = m.name!;
      if (needle && !`${m.provider}/${name}`.toLowerCase().includes(needle)) continue;
      byProvider.set(m.provider, [...(byProvider.get(m.provider) ?? []), name]);
    }
    return [...byProvider.entries()];
  }, [models, filter]);

  const label = model ?? `${provider} (default)`;

  return (
    <div className="vx-pill-wrap" ref={wrapRef}>
      {open && (
        <div className="vx-pill-menu vx-pill-menu--models" role="menu">
          <input
            className="vx-pill-menu__filter"
            placeholder="Filter models…"
            value={filter}
            autoFocus
            onChange={(e) => setFilter(e.target.value)}
          />
          <div className="vx-pill-menu__scroll">
            {models.length === 0 && (
              <div className="vx-pill-menu__empty">No models reported by the daemon</div>
            )}
            {models.length > 0 && groups.length === 0 && (
              <div className="vx-pill-menu__empty">No model matches “{filter}”</div>
            )}
            {groups.map(([prov, list]) => (
              <div key={prov}>
                <div className="vx-pill-menu__group">{prov}</div>
                {list.map((name) => {
                  const active = provider === prov && model === name;
                  return (
                    <button
                      key={`${prov}/${name}`}
                      role="menuitemradio"
                      aria-checked={active}
                      className={`vx-pill-menu__item${active ? " is-active" : ""}`}
                      onClick={() => {
                        onSelect(prov, name);
                        setOpen(false);
                        setFilter("");
                      }}
                    >
                      <span className="vx-pill-menu__item-name">{name}</span>
                      {active && <span aria-hidden>✓</span>}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
        </div>
      )}
      <button
        className="vx-pill"
        onClick={() => setOpen((v) => !v)}
        aria-label="Provider and model"
        aria-expanded={open}
        title={`${provider} · ${label}`}
      >
        <span className="vx-pill__model">{label}</span>
        <ChevronUp size={12} />
      </button>
    </div>
  );
}
