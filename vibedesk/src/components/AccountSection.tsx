import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check } from "lucide-react";

/** Identity providers VibeDesk can be configured against (mirrors VibeUI's set). */
const IDENTITY_PROVIDERS: { id: string; label: string }[] = [
  { id: "google", label: "Google" },
  { id: "github", label: "GitHub" },
  { id: "gitlab", label: "GitLab" },
  { id: "microsoft", label: "Microsoft" },
  { id: "apple", label: "Apple" },
];

/**
 * Account / identity-provider configuration (VX identity slice). Stores each
 * provider's OAuth client credentials in the shared encrypted ProfileStore
 * (client_id + client_secret), so VibeDesk is configurable for SSO. The live
 * browser-callback sign-in flow reuses VibeUI's cloud_oauth_* path and is a
 * follow-up; here we own the durable configuration.
 */
export function AccountSection() {
  const [configured, setConfigured] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState<string | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");

  async function refresh() {
    const next = new Set<string>();
    for (const p of IDENTITY_PROVIDERS) {
      try {
        const has = await invoke<boolean>("oauth_client_has", { provider: p.id });
        if (has) next.add(p.id);
      } catch {
        /* ignore */
      }
    }
    setConfigured(next);
  }

  useEffect(() => {
    refresh();
  }, []);

  async function save(provider: string) {
    if (!clientId.trim()) return;
    await invoke("oauth_client_set", {
      provider,
      clientId: clientId.trim(),
      clientSecret: clientSecret.trim(),
    });
    setClientId("");
    setClientSecret("");
    setOpen(null);
    refresh();
  }

  return (
    <div className="vx-set-section">
      <h3 className="vx-set-h">Identity providers</h3>
      <p className="vx-set-hint">
        Configure OAuth client credentials for sign-in / account linking. Stored encrypted in the
        shared ProfileStore.
      </p>
      <ul className="vx-set-keys">
        {IDENTITY_PROVIDERS.map((p) => {
          const has = configured.has(p.id);
          const isOpen = open === p.id;
          return (
            <li key={p.id} className="vx-set-key">
              <div className="vx-set-key__label">
                {p.label}
                {has && <span className="vx-set-key__badge"><Check size={11} /> configured</span>}
                <button
                  className="vx-set-key__edit"
                  onClick={() => setOpen(isOpen ? null : p.id)}
                >
                  {isOpen ? "Cancel" : has ? "Edit" : "Configure"}
                </button>
              </div>
              {isOpen && (
                <div className="vx-set-key__row vx-set-key__row--col">
                  <input
                    className="vx-set-input"
                    placeholder="Client ID"
                    value={clientId}
                    onChange={(e) => setClientId(e.target.value)}
                  />
                  <input
                    className="vx-set-input"
                    type="password"
                    placeholder="Client secret (optional for PKCE)"
                    value={clientSecret}
                    onChange={(e) => setClientSecret(e.target.value)}
                  />
                  <button
                    className="panel-btn panel-btn-primary"
                    onClick={() => save(p.id)}
                    disabled={!clientId.trim()}
                  >
                    Save
                  </button>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
