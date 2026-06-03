import { useState } from "react";
import { X, KeyRound, Palette, UserCircle, Check, Trash2 } from "lucide-react";
import { useProviderSettings, KEYED_PROVIDERS, LOCAL_PROVIDERS } from "../hooks/useProviderSettings";
import { useTheme, type ThemeMode } from "../hooks/useTheme";
import { AccountSection } from "./AccountSection";

interface SettingsViewProps {
  onClose: () => void;
}

type Tab = "providers" | "appearance" | "account";

const TABS: { id: Tab; label: string; icon: typeof KeyRound }[] = [
  { id: "providers", label: "Providers", icon: KeyRound },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "account", label: "Account", icon: UserCircle },
];

/**
 * Settings, carried over from VibeUI by sharing the encrypted ProfileStore:
 * Providers (API keys + per-provider config + default selection), Appearance
 * (theme), and Account (identity / OAuth). Opens as a center overlay.
 */
export function SettingsView({ onClose }: SettingsViewProps) {
  const [tab, setTab] = useState<Tab>("providers");

  return (
    <div className="vx-settings">
      <div className="vx-settings__head">
        <span>Settings</span>
        <button className="vx-icon-btn" aria-label="Close settings" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="vx-settings__body">
        <nav className="vx-settings__tabs">
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`vx-settings__tab${tab === id ? " is-active" : ""}`}
              onClick={() => setTab(id)}
            >
              <Icon size={15} /> <span>{label}</span>
            </button>
          ))}
        </nav>
        <div className="vx-settings__panel">
          {tab === "providers" && <ProvidersSection />}
          {tab === "appearance" && <AppearanceSection />}
          {tab === "account" && <AccountSection />}
        </div>
      </div>
    </div>
  );
}

function ProvidersSection() {
  const { configured, defaultProvider, setKey, deleteKey, setProviderUrl, setDefaultProvider } =
    useProviderSettings();
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [urls, setUrls] = useState<Record<string, string>>({});

  async function save(id: string) {
    const v = (drafts[id] ?? "").trim();
    if (!v) return;
    await setKey(id, v);
    if (urls[id]?.trim()) await setProviderUrl(id, urls[id].trim());
    setDrafts((d) => ({ ...d, [id]: "" }));
  }

  const providerOptions = [...LOCAL_PROVIDERS, ...KEYED_PROVIDERS.map((p) => p.id)];

  return (
    <div className="vx-set-section">
      <h3 className="vx-set-h">Default provider</h3>
      <p className="vx-set-hint">Used when a task doesn't pick a model. Local providers need no key.</p>
      <select
        className="vx-set-select"
        value={defaultProvider}
        onChange={(e) => setDefaultProvider(e.target.value)}
      >
        {providerOptions.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>

      <h3 className="vx-set-h">API keys</h3>
      <p className="vx-set-hint">
        Stored encrypted in the shared ProfileStore (~/.vibecli) — the same keys VibeUI uses.
      </p>
      <ul className="vx-set-keys">
        {KEYED_PROVIDERS.map((p) => {
          const has = configured.has(p.id);
          return (
            <li key={p.id} className="vx-set-key">
              <div className="vx-set-key__label">
                {p.label}
                {has && <span className="vx-set-key__badge"><Check size={11} /> configured</span>}
              </div>
              <div className="vx-set-key__row">
                <input
                  className="vx-set-input"
                  type="password"
                  placeholder={has ? "•••••••• (set — type to replace)" : `${p.id} API key`}
                  value={drafts[p.id] ?? ""}
                  onChange={(e) => setDrafts((d) => ({ ...d, [p.id]: e.target.value }))}
                />
                {p.needsUrl && (
                  <input
                    className="vx-set-input"
                    placeholder="endpoint URL"
                    value={urls[p.id] ?? ""}
                    onChange={(e) => setUrls((u) => ({ ...u, [p.id]: e.target.value }))}
                  />
                )}
                <button className="panel-btn panel-btn-primary" onClick={() => save(p.id)} disabled={!drafts[p.id]?.trim()}>
                  Save
                </button>
                {has && (
                  <button className="vx-icon-btn" aria-label={`Delete ${p.id} key`} onClick={() => deleteKey(p.id)}>
                    <Trash2 size={14} />
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function AppearanceSection() {
  const { mode, setTheme, themeId, setThemeId, themes } = useTheme();
  const modes: { id: ThemeMode; label: string }[] = [
    { id: "dark", label: "Dark" },
    { id: "light", label: "Light" },
  ];
  // Show themes of the currently-selected mode, grouped by category. The
  // category labels mirror the VibeUI Settings → Appearance ordering so
  // a user moving between apps doesn't have to relearn the layout.
  const CATEGORY_ORDER = ["standard", "high-contrast", "color-blind", "supercar"] as const;
  const CATEGORY_LABEL: Record<(typeof CATEGORY_ORDER)[number], string> = {
    standard: "Standard",
    "high-contrast": "High contrast",
    "color-blind": "Color-blind friendly",
    supercar: "Supercar",
  };
  const visible = themes.filter((t) => t.mode === mode);
  return (
    <div className="vx-set-section">
      <h3 className="vx-set-h">Theme</h3>
      <p className="vx-set-hint">Uses the shared VibeCody design tokens, so VibeX matches VibeUI.</p>

      {/* Mode toggle — flips within the current pair, so "Charcoal dark → light"
          stays on Charcoal instead of jumping back to Default. */}
      <div className="vx-set-themes" role="radiogroup" aria-label="Theme mode">
        {modes.map((m) => (
          <button
            key={m.id}
            role="radio"
            aria-checked={mode === m.id}
            className={`vx-set-theme${mode === m.id ? " is-active" : ""}`}
            onClick={() => setTheme(m.id)}
            data-theme-preview={m.id}
          >
            <span className="vx-set-theme__swatch" data-theme={m.id} />
            <span>{m.label}</span>
            {mode === m.id && <Check size={14} />}
          </button>
        ))}
      </div>

      {/* Full theme picker — grouped by category. The swatch is rendered from
          each theme's own `preview` colors (bg + fg + accent + secondary) so
          you can compare palettes without applying them. */}
      {CATEGORY_ORDER.map((cat) => {
        const inCat = visible.filter((t) => t.category === cat);
        if (inCat.length === 0) return null;
        return (
          <div key={cat} className="vx-set-theme-group">
            <h4 className="vx-set-theme-group__h">{CATEGORY_LABEL[cat]}</h4>
            <div className="vx-set-theme-grid" role="radiogroup" aria-label={`${CATEGORY_LABEL[cat]} themes`}>
              {inCat.map((t) => {
                const active = themeId === t.id;
                return (
                  <button
                    key={t.id}
                    role="radio"
                    aria-checked={active}
                    className={`vx-set-theme-card${active ? " is-active" : ""}`}
                    onClick={() => setThemeId(t.id)}
                    title={t.name}
                  >
                    <span
                      className="vx-set-theme-card__swatch"
                      style={{
                        background: t.preview.bg,
                        color: t.preview.fg,
                        boxShadow: `inset 0 0 0 1px var(--border-color)`,
                      }}
                    >
                      <span
                        className="vx-set-theme-card__dot"
                        style={{ background: t.preview.accent }}
                      />
                      <span
                        className="vx-set-theme-card__dot"
                        style={{ background: t.preview.secondary }}
                      />
                    </span>
                    <span className="vx-set-theme-card__name">{t.name}</span>
                    {active && <Check size={12} className="vx-set-theme-card__check" />}
                  </button>
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}
