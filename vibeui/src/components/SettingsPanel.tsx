/**
 * SettingsPanel — Comprehensive settings panel opened via the gear icon.
 *
 * Sections:
 *   1. Profile — Display name, avatar, email, bio
 *   2. Appearance — 19 theme pairs (dark/light/high-contrast/color-blind/supercar), font size, UI density
 *   3. OAuth Login — Google, GitHub, GitLab, Bitbucket, Microsoft, Apple
 *   4. Saved Customizations — Export/import/reset workspace preferences
 *   5. API Keys — BYOK provider keys (existing functionality preserved)
 */
import React, { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  User, Palette, LogIn, Save, Key, X, Check, Upload, Download, RotateCcw,
  Sun, Moon, Eye, EyeOff, ChevronRight, CheckCircle, MinusCircle, AlertCircle,
  Loader2, Zap, Plug,
  Mail, CalendarDays, ClipboardList, MessageSquare, Search, Mic, Home, Server,
} from "lucide-react";
import { THEMES, applyThemeById, type ThemeDef } from "../theme/themes";

/* ── Types ──────────────────────────────────────────────────────────── */

type SettingsSection = "profile" | "appearance" | "oauth" | "customizations" | "apikeys" | "integrations" | "sessions";

interface SessionsSettings {
  recapOnTabClose: boolean;
  recapOnIdle: boolean;
  idleMinutes: number;
  generator: "heuristic" | "llm";
  autoResumeLast: boolean;
}

const SESSIONS_DEFAULTS: SessionsSettings = {
  recapOnTabClose: true,
  recapOnIdle: false,
  idleMinutes: 30,
  generator: "heuristic",
  autoResumeLast: false,
};

interface UserProfile {
  displayName: string;
  email: string;
  bio: string;
  avatarUrl: string;
}

interface OAuthProvider {
  id: string;
  name: string;
  icon: string;
  connected: boolean;
  email?: string;
  displayName?: string;
  expired?: boolean;
}

interface SavedCustomization {
  id: string;
  name: string;
  createdAt: string;
  theme: string;
  fontSize: number;
  density: string;
}

interface ApiKeySettings {
  anthropic_api_key: string;
  openai_api_key: string;
  gemini_api_key: string;
  grok_api_key: string;
  groq_api_key: string;
  openrouter_api_key: string;
  azure_openai_api_key: string;
  azure_openai_api_url: string;
  mistral_api_key: string;
  cerebras_api_key: string;
  deepseek_api_key: string;
  zhipu_api_key: string;
  vercel_ai_api_key: string;
  vercel_ai_api_url: string;
  minimax_api_key: string;
  perplexity_api_key: string;
  together_api_key: string;
  fireworks_api_key: string;
  sambanova_api_key: string;
  ollama_api_key: string;
  ollama_api_url: string;
  claude_model: string;
  openai_model: string;
  openrouter_model: string;
}


const OAUTH_PROVIDERS: OAuthProvider[] = [
  { id: "google", name: "Google", icon: "G", connected: false },
  { id: "github", name: "GitHub", icon: "GH", connected: false },
  { id: "gitlab", name: "GitLab", icon: "GL", connected: false },
  { id: "bitbucket", name: "Bitbucket", icon: "BB", connected: false },
  { id: "microsoft", name: "Microsoft", icon: "MS", connected: false },
  { id: "apple", name: "Apple", icon: "A", connected: false },
];

const STORAGE_KEYS = {
  profile: "vibeui-profile",
  theme: "vibeui-theme-id",
  themeMode: "vibeui-theme",
  fontSize: "vibeui-font-size",
  density: "vibeui-density",
  oauth: "vibeui-oauth",
  customizations: "vibeui-customizations",
  sessions: "vibeui-sessions",
};

/* ── Shared styles ─────────────────────────────────────────────────── */

const sectionBtnStyle = (active: boolean): React.CSSProperties => ({
  display: "flex", alignItems: "center", gap: 10, width: "100%", padding: "10px 14px",
  background: active ? "var(--accent-bg)" : "transparent", border: "none",
  borderRadius: "var(--radius-sm)", cursor: "pointer", fontSize: "var(--font-size-md)", fontWeight: active ? 600 : 400,
  color: active ? "var(--accent-color)" : "var(--text-primary)", textAlign: "left",
  transition: "var(--transition-fast)",
});

const modelsHintStyle: React.CSSProperties = { fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", margin: "4px 0 0", lineHeight: 1.4, opacity: 0.8 };

/* ── Section Components ────────────────────────────────────────────── */

function ProfileSection() {
  const [profile, setProfile] = useState<UserProfile>({ displayName: "", email: "", bio: "", avatarUrl: "" });
  const [saved, setSaved] = useState(false);
  const [googleLoading, setGoogleLoading] = useState(false);
  const [googleConnected, setGoogleConnected] = useState(false);
  const [googleError, setGoogleError] = useState<string | null>(null);
  const [googleConfiguring, setGoogleConfiguring] = useState(false);
  const [gClientId, setGClientId] = useState("");
  const [gClientSecret, setGClientSecret] = useState("");
  const [gAuthCode, setGAuthCode] = useState("");
  const [gAwaitingCode, setGAwaitingCode] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.profile);
    if (stored) setProfile(JSON.parse(stored));
    checkGoogleStatus();
  }, []);

  // Listen for OAuth callback from the temporary local server
  useEffect(() => {
    const unlisten = listen<{ provider: string; code?: string; error?: string }>("oauth-callback", (event) => {
      if (event.payload.provider !== "google") return;
      if (event.payload.code) {
        setGAuthCode(event.payload.code);
        // Auto-complete the OAuth flow
        (async () => {
          setGoogleLoading(true);
          setGoogleError(null);
          try {
            const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
            await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
              provider: "google",
              code: event.payload.code,
              clientId: config.client_id,
              clientSecret: config.client_secret || "",
              redirectUri: "http://localhost:7878/oauth/callback",
            });
            setGAwaitingCode(false);
            setGAuthCode("");
            const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
            const updated = {
              ...profile,
              displayName: gProfile.displayName || profile.displayName,
              email: gProfile.email || profile.email,
              avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
            };
            setProfile(updated);
            localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
            setGoogleConnected(true);
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
          } catch (_e) {
            setGoogleError(String(_e));
          } finally {
            setGoogleLoading(false);
          }
        })();
      } else if (event.payload.error) {
        setGoogleError(`OAuth error: ${event.payload.error}`);
        setGAwaitingCode(false);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [profile]);

  const checkGoogleStatus = async () => {
    try {
      const status = await invoke<{ connected: boolean; expired: boolean; email: string }>("cloud_oauth_status", { provider: "google" });
      setGoogleConnected(status.connected && !status.expired);
    } catch { /* not connected */ }
  };

  const save = () => {
    localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(profile));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleGoogleLogin = async () => {
    setGoogleError(null);
    // First check if already connected — just fetch profile
    try {
      const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
      if (gProfile.email) {
        const updated = {
          ...profile,
          displayName: gProfile.displayName || profile.displayName,
          email: gProfile.email || profile.email,
          avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
        };
        setProfile(updated);
        localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
        setGoogleConnected(true);
        setSaved(true);
        setTimeout(() => setSaved(false), 2000);
        return;
      }
    } catch { /* not connected yet, start flow */ }

    // Check if client config exists
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
      if (config.client_id) {
        await startGoogleOAuth(config.client_id);
        return;
      }
    } catch { /* no config */ }

    // Need to configure client credentials first
    setGoogleConfiguring(true);
    setGClientId("");
    setGClientSecret("");
  };

  const startGoogleOAuth = async (clientId: string) => {
    setGoogleLoading(true);
    setGoogleError(null);
    try {
      const redirectUri = "http://localhost:7878/oauth/callback";
      await invoke<string>("cloud_oauth_initiate", {
        provider: "google",
        clientId,
        redirectUri,
      });
      setGAwaitingCode(true);
      setGAuthCode("");
    } catch (_e) {
      setGoogleError(String(_e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const saveGoogleConfig = async () => {
    if (!gClientId.trim()) { setGoogleError("Client ID is required"); return; }
    setGoogleLoading(true);
    try {
      await invoke("cloud_oauth_save_client_config", {
        provider: "google",
        clientId: gClientId.trim(),
        clientSecret: gClientSecret.trim(),
      });
      setGoogleConfiguring(false);
      await startGoogleOAuth(gClientId.trim());
    } catch (_e) {
      setGoogleError(String(_e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const completeGoogleOAuth = async () => {
    if (!gAuthCode.trim()) { setGoogleError("Authorization code is required"); return; }
    setGoogleLoading(true);
    setGoogleError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
      await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
        provider: "google",
        code: gAuthCode.trim(),
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
        redirectUri: "http://localhost:7878/oauth/callback",
      });
      setGAwaitingCode(false);
      setGAuthCode("");

      // Now fetch the full Google profile including avatar
      const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
      const updated = {
        ...profile,
        displayName: gProfile.displayName || profile.displayName,
        email: gProfile.email || profile.email,
        avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
      };
      setProfile(updated);
      localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
      setGoogleConnected(true);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (_e) {
      setGoogleError(String(_e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const handleGoogleDisconnect = async () => {
    try {
      await invoke("cloud_oauth_disconnect", { provider: "google" });
      setGoogleConnected(false);
    } catch (_e) {
      setGoogleError(String(_e));
    }
  };

  const initials = profile.displayName.split(/\s+/).map(w => w[0]?.toUpperCase() || "").join("").slice(0, 2) || "?";

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Profile</h3>

      {/* Google Sign-In card */}
      <div style={{
        padding: 14, borderRadius: "var(--radius-md)", marginBottom: 20,
        border: googleConnected ? "1px solid var(--success-color)" : "1px solid var(--border-color)",
        background: googleConnected ? "var(--success-bg)" : "var(--bg-secondary)",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          {/* Google "G" logo */}
          <div style={{
            width: 40, height: 40, borderRadius: "var(--radius-sm)", background: "var(--bg-elevated)",
            display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0,
            border: "1px solid var(--border-color)",
          }}>
            <svg width="20" height="20" viewBox="0 0 48 48">
              <path fill="#4285F4" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"/>
              <path fill="#34A853" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z"/>
              <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z"/>
              <path fill="#EA4335" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"/>
            </svg>
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>Sign in with Google</div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
              {googleConnected
                ? `Connected as ${profile.email || "Google user"}`
                : "Auto-fill your profile with your Google account"}
            </div>
          </div>
          <div>
            {googleConnected ? (
              <div style={{ display: "flex", gap: 6 }}>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={handleGoogleLogin}>
                  Refresh
                </button>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }} onClick={handleGoogleDisconnect}>
                  Disconnect
                </button>
              </div>
            ) : (
              <button className="panel-btn"
                style={{
                  padding: "8px 20px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)",
                  background: "var(--bg-elevated)", color: "var(--text-primary)", cursor: "pointer", fontSize: "var(--font-size-md)", fontWeight: 500,
                  display: "flex", alignItems: "center", gap: 8,
                }}
                onClick={handleGoogleLogin}
                disabled={googleLoading}
              >
                {googleLoading ? "..." : "Sign in with Google"}
              </button>
            )}
          </div>
        </div>

        {googleError && (
          <div style={{ marginTop: 8, padding: "8px 12px", borderRadius: "var(--radius-xs-plus)", background: "var(--error-bg)", color: "var(--error-color)", fontSize: "var(--font-size-sm)" }}>
            {googleError}
            <button className="panel-btn" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit" }} onClick={() => setGoogleError(null)}>x</button>
          </div>
        )}

        {/* Client credential configuration */}
        {googleConfiguring && (
          <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>
              Enter your Google OAuth client credentials. Create them at{" "}
              <span style={{ color: "var(--accent-blue)" }}>Google Cloud Console &gt; APIs &amp; Services &gt; Credentials</span>.
              Set the redirect URI to <code style={{ fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", padding: "1px 4px", borderRadius: 3 }}>http://localhost:7878/oauth/callback</code>.
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <input className="panel-input panel-input-full" placeholder="Client ID" value={gClientId} onChange={e => setGClientId(e.target.value)} />
              <input className="panel-input panel-input-full" placeholder="Client Secret" type="password" value={gClientSecret} onChange={e => setGClientSecret(e.target.value)} />
              <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => setGoogleConfiguring(false)}>Cancel</button>
                <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={saveGoogleConfig} disabled={googleLoading || !gClientId.trim()}>
                  {googleLoading ? "Saving..." : "Save & Connect"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Authorization code entry */}
        {gAwaitingCode && (
          <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>
              A browser window has opened. After authorizing with Google, paste the authorization code below:
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <input className="panel-input" style={{ flex: 1 }} placeholder="Paste authorization code here"
                value={gAuthCode} onChange={e => setGAuthCode(e.target.value)}
                onKeyDown={e => e.key === "Enter" && completeGoogleOAuth()} />
              <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { setGAwaitingCode(false); setGAuthCode(""); }}>Cancel</button>
              <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={completeGoogleOAuth} disabled={googleLoading || !gAuthCode.trim()}>
                {googleLoading ? "Connecting..." : "Complete"}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Avatar */}
      <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 20 }}>
        <div style={{
          width: 64, height: 64, borderRadius: "50%", background: "var(--accent-color)",
          display: "flex", alignItems: "center", justifyContent: "center", fontSize: 22, fontWeight: 700, color: "var(--btn-primary-fg)",
          overflow: "hidden", flexShrink: 0,
        }}>
          {profile.avatarUrl ? <img src={profile.avatarUrl} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} /> : initials}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", color: "var(--text-primary)" }}>{profile.displayName || "Set your name"}</div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{profile.email || "No email set"}</div>
        </div>
      </div>

      <div style={{ marginBottom: 12 }}>
        <label className="panel-label">Display Name</label>
        <input className="panel-input panel-input-full" value={profile.displayName} onChange={e => setProfile({ ...profile, displayName: e.target.value })} placeholder="Your name" />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label className="panel-label">Email</label>
        <input className="panel-input panel-input-full" type="email" value={profile.email} onChange={e => setProfile({ ...profile, email: e.target.value })} placeholder="you@example.com" />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label className="panel-label">Bio</label>
        <textarea className="panel-input panel-input-full" style={{ minHeight: 60, resize: "vertical" }} value={profile.bio} onChange={e => setProfile({ ...profile, bio: e.target.value })} placeholder="A short bio..." />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label className="panel-label">Avatar URL</label>
        <input className="panel-input panel-input-full" value={profile.avatarUrl} onChange={e => setProfile({ ...profile, avatarUrl: e.target.value })} placeholder="https://..." />
      </div>

      <button className="panel-btn panel-btn-primary" onClick={save}>
        {saved ? <><Check size={14} /> Saved</> : <><Save size={14} /> Save Profile</>}
      </button>
    </div>
  );
}

function AppearanceSection() {
  const [activeThemeId, setActiveThemeId] = useState("dark-sherwood");
  const [fontSize, setFontSize] = useState(13);
  const [density, setDensity] = useState<"compact" | "normal" | "spacious">("normal");
  const [filterCategory, setFilterCategory] = useState<"all" | "standard" | "high-contrast" | "color-blind" | "supercar">("all");

  useEffect(() => {
    const storedTheme = localStorage.getItem(STORAGE_KEYS.theme) || "dark-sherwood";
    const storedSize = localStorage.getItem(STORAGE_KEYS.fontSize);
    const storedDensity = localStorage.getItem("vibeui-density");
    setActiveThemeId(storedTheme);
    if (storedSize) setFontSize(parseInt(storedSize, 10));
    if (storedDensity) setDensity(storedDensity as typeof density);
  }, []);

  const applyTheme = useCallback((theme: ThemeDef) => {
    setActiveThemeId(theme.id);
    applyThemeById(theme.id);
  }, []);

  const applyFontSize = (size: number) => {
    setFontSize(size);
    localStorage.setItem(STORAGE_KEYS.fontSize, String(size));
    document.documentElement.style.setProperty("--editor-font-size", `${size}px`);
  };

  const applyDensity = (d: typeof density) => {
    setDensity(d);
    localStorage.setItem("vibeui-density", d);
    const spacing = d === "compact" ? "2px" : d === "spacious" ? "6px" : "4px";
    document.documentElement.style.setProperty("--density-spacing", spacing);
  };

  const filtered = filterCategory === "all" ? THEMES : THEMES.filter(t => t.category === filterCategory);

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Appearance</h3>

      {/* Category filter */}
      <div style={{ display: "flex", gap: 4, marginBottom: 14, flexWrap: "wrap" }}>
        {(["all", "standard", "supercar", "high-contrast", "color-blind"] as const).map(cat => (
          <button key={cat} onClick={() => setFilterCategory(cat)} className={`panel-tab ${filterCategory === cat ? "active" : ""}`} style={{ textTransform: "capitalize" }}>
            {cat === "all" ? "All Themes" : cat.replace("-", " ")}
          </button>
        ))}
      </div>

      {/* Theme pairs grid — grouped by pairId */}
      {(() => {
        const pairIds = [...new Set(filtered.map(t => t.pairId))];
        return pairIds.map(pid => {
          const pair = filtered.filter(t => t.pairId === pid);
          const dark = pair.find(t => t.mode === "dark");
          const light = pair.find(t => t.mode === "light");
          const pairName = dark?.name || light?.name || pid;
          const isActivePair = pair.some(t => t.id === activeThemeId);
          const activeInPair = pair.find(t => t.id === activeThemeId);
          const signature = (activeInPair ?? dark ?? light)?.preview.accent ?? "var(--accent-blue)";
          return (
            <div key={pid} style={{
              marginBottom: 14, borderRadius: "var(--radius-md)", overflow: "hidden",
              border: isActivePair ? `2px solid ${signature}` : "1px solid var(--border-color)",
              outline: isActivePair ? `2px solid color-mix(in srgb, ${signature} 25%, transparent)` : "none", outlineOffset: 2,
            }}>
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", padding: "8px 12px", background: "var(--bg-tertiary)", textTransform: "uppercase", letterSpacing: 0.5 }}>
                {pairName}
              </div>
              <div style={{ display: "grid", gridTemplateColumns: pair.length > 1 ? "1fr 1fr" : "1fr" }}>
                {[dark, light].filter(Boolean).map(theme => {
                  if (!theme) return null;
                  const isActive = activeThemeId === theme.id;
                  return (
                    <button key={theme.id} onClick={() => applyTheme(theme)} style={{
                      padding: 0, border: "none", borderRight: theme.mode === "dark" && light ? "1px solid var(--border-color)" : "none",
                      cursor: "pointer", background: "none", transition: "var(--transition-fast)",
                      outline: isActive ? `2px solid ${theme.preview.accent} inset` : "none",
                    }}>
                      <div style={{ display: "flex", height: 36 }}>
                        <div style={{ flex: 3, background: theme.preview.bg }} />
                        <div style={{ flex: 2, background: theme.preview.secondary }} />
                        <div style={{ flex: 1, background: theme.preview.accent }} />
                      </div>
                      <div style={{ padding: "4px 8px", background: theme.preview.bg, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 500, color: theme.preview.fg }}>{theme.name}</span>
                        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                          {theme.mode === "dark" ? <Moon size={10} color={theme.preview.fg} /> : <Sun size={10} color={theme.preview.fg} />}
                          {isActive && <Check size={12} color={theme.preview.accent} />}
                        </div>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          );
        });
      })()}

      <div className="panel-divider" />

      {/* Font size */}
      <div style={{ marginBottom: 16 }}>
        <label className="panel-label">Editor Font Size: {fontSize}px</label>
        <input type="range" min={10} max={22} value={fontSize} onChange={e => applyFontSize(+e.target.value)} style={{ width: "100%", accentColor: "var(--accent-blue)" }} />
      </div>

      {/* UI Density */}
      <div style={{ marginBottom: 16 }}>
        <label className="panel-label">UI Density</label>
        <div style={{ display: "flex", gap: 8 }}>
          {(["compact", "normal", "spacious"] as const).map(d => (
            <button key={d} onClick={() => applyDensity(d)} className={`panel-tab ${density === d ? "active" : ""}`} style={{ flex: 1, textTransform: "capitalize" }}>{d}</button>
          ))}
        </div>
      </div>

    </div>
  );
}

function OAuthSection() {
  const [providers, setProviders] = useState<OAuthProvider[]>(OAUTH_PROVIDERS);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [configuring, setConfiguring] = useState<string | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [authCode, setAuthCode] = useState("");
  const [awaitingCode, setAwaitingCode] = useState<string | null>(null);

  // Load connection status on mount
  useEffect(() => {
    loadStatuses();
  }, []);

  const loadStatuses = async () => {
    try {
      const connected = await invoke<Array<{ provider: string; email: string; display_name: string; expired: boolean }>>("cloud_oauth_list_connected");
      setProviders(prev => prev.map(p => {
        const match = connected.find((c: { provider: string }) => c.provider === p.id);
        if (match) {
          return { ...p, connected: true, email: match.email || match.display_name, displayName: match.display_name, expired: match.expired };
        }
        return { ...p, connected: false, email: undefined, displayName: undefined, expired: undefined };
      }));
    } catch { /* ignore — first run */ }
  };

  const handleConnect = async (id: string) => {
    setError(null);
    // Check if client config exists
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      if (!config.client_id) {
        setConfiguring(id);
        setClientId("");
        setClientSecret("");
        return;
      }
      // Start OAuth flow
      await startOAuthFlow(id, config.client_id);
    } catch (_e) {
      setConfiguring(id);
      setClientId("");
      setClientSecret("");
    }
  };

  const startOAuthFlow = async (id: string, cId: string) => {
    setLoading(id);
    setError(null);
    try {
      const redirectUri = "http://localhost:7878/oauth/callback";
      await invoke<string>("cloud_oauth_initiate", {
        provider: id,
        clientId: cId,
        redirectUri,
      });
      // Browser opened — now wait for auth code
      setAwaitingCode(id);
      setAuthCode("");
    } catch (_e) {
      setError(String(_e));
    } finally {
      setLoading(null);
    }
  };

  const completeOAuth = async (id: string) => {
    if (!authCode.trim()) { setError("Authorization code is required"); return; }
    setLoading(id);
    setError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      const result = await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
        provider: id,
        code: authCode.trim(),
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
        redirectUri: "http://localhost:7878/oauth/callback",
      });
      setProviders(prev => prev.map(p =>
        p.id === id ? { ...p, connected: true, email: result.email || result.display_name, displayName: result.display_name, expired: false } : p
      ));
      setAwaitingCode(null);
      setAuthCode("");
    } catch (_e) {
      setError(String(_e));
    } finally {
      setLoading(null);
    }
  };

  const saveClientConfig = async (id: string) => {
    if (!clientId.trim()) { setError("Client ID is required"); return; }
    setLoading(id);
    try {
      await invoke("cloud_oauth_save_client_config", {
        provider: id,
        clientId: clientId.trim(),
        clientSecret: clientSecret.trim(),
      });
      setConfiguring(null);
      // Now start the OAuth flow
      await startOAuthFlow(id, clientId.trim());
    } catch (_e) {
      setError(String(_e));
    } finally {
      setLoading(null);
    }
  };

  const handleDisconnect = async (id: string) => {
    setLoading(id);
    try {
      await invoke("cloud_oauth_disconnect", { provider: id });
      setProviders(prev => prev.map(p => p.id === id ? { ...p, connected: false, email: undefined, displayName: undefined, expired: undefined } : p));
    } catch (_e) {
      setError(String(_e));
    } finally {
      setLoading(null);
    }
  };

  const handleRefresh = async (id: string) => {
    setLoading(id);
    setError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      await invoke("cloud_oauth_refresh", {
        provider: id,
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
      });
      await loadStatuses();
    } catch (_e) {
      setError(String(_e));
    } finally {
      setLoading(null);
    }
  };

  const providerColors: Record<string, string> = {
    google: "#4285f4", github: "var(--text-secondary)", gitlab: "#fc6d26",
    bitbucket: "#0052cc", microsoft: "#00a4ef", apple: "var(--text-primary)",
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 8px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Cloud OAuth</h3>
      <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
        Connect your cloud accounts via OAuth 2.0 for scanning, IAM, IaC generation, and cost analysis.
        You'll need to register an OAuth app with each provider and enter your client credentials.
      </p>

      {error && (
        <div className="panel-error" style={{ marginBottom: 12 }}>
          <span>{error}</span>
          <button className="panel-btn" aria-label="Dismiss error" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit", display: "flex", alignItems: "center" }} onClick={() => setError(null)}><X size={14} /></button>
        </div>
      )}

      {providers.map(p => (
        <div key={p.id} className="panel-card" style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <div style={{
              width: 36, height: 36, borderRadius: "var(--radius-sm)", background: providerColors[p.id] || "var(--bg-tertiary)",
              display: "flex", alignItems: "center", justifyContent: "center", color: "var(--btn-primary-fg)", fontSize: "var(--font-size-base)", fontWeight: 700, flexShrink: 0,
            }}>
              {p.icon}
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{p.name}</div>
              {p.connected ? (
                <div style={{ fontSize: "var(--font-size-sm)" }}>
                  <span style={{ color: p.expired ? "var(--warning-color)" : "var(--success-color)" }}>
                    {p.expired ? "Token expired" : "Connected"}
                  </span>
                  {p.email && <span style={{ color: "var(--text-secondary)", marginLeft: 6 }}>{p.email}</span>}
                </div>
              ) : (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Not connected</div>
              )}
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              {p.connected && p.expired && (
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}
                  onClick={() => handleRefresh(p.id)} disabled={loading === p.id}>
                  Refresh
                </button>
              )}
              {p.connected ? (
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }}
                  onClick={() => handleDisconnect(p.id)} disabled={loading === p.id}>
                  Disconnect
                </button>
              ) : (
                <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }}
                  onClick={() => handleConnect(p.id)} disabled={loading === p.id}>
                  {loading === p.id ? "..." : "Connect"}
                </button>
              )}
            </div>
          </div>

          {/* Client credential configuration form */}
          {configuring === p.id && (
            <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>
                Enter your OAuth app credentials for {p.name}. Register an app at the provider's developer console.
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <input className="panel-input panel-input-full" placeholder="Client ID" value={clientId} onChange={e => setClientId(e.target.value)} />
                <input className="panel-input panel-input-full" placeholder="Client Secret (optional for some providers)" type="password"
                  value={clientSecret} onChange={e => setClientSecret(e.target.value)} />
                <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => setConfiguring(null)}>Cancel</button>
                  <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }}
                    onClick={() => saveClientConfig(p.id)} disabled={loading === p.id || !clientId.trim()}>
                    {loading === p.id ? "Saving..." : "Save & Connect"}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Authorization code entry (after browser redirect) */}
          {awaitingCode === p.id && (
            <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>
                A browser window has opened. After authorizing, paste the authorization code below:
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <input className="panel-input" style={{ flex: 1 }} placeholder="Paste authorization code here"
                  value={authCode} onChange={e => setAuthCode(e.target.value)} />
                <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => { setAwaitingCode(null); setAuthCode(""); }}>Cancel</button>
                <button className="panel-btn panel-btn-primary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }}
                  onClick={() => completeOAuth(p.id)} disabled={loading === p.id || !authCode.trim()}>
                  {loading === p.id ? "Connecting..." : "Complete"}
                </button>
              </div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function CustomizationsSection() {
  const [customs, setCustoms] = useState<SavedCustomization[]>(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.customizations);
    return stored ? JSON.parse(stored) : [];
  });
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");

  const saveList = (list: SavedCustomization[]) => {
    setCustoms(list);
    localStorage.setItem(STORAGE_KEYS.customizations, JSON.stringify(list));
  };

  const saveCurrentPrefs = () => {
    if (!name.trim()) return;
    const custom: SavedCustomization = {
      id: Date.now().toString(36),
      name: name.trim(),
      createdAt: new Date().toISOString(),
      theme: localStorage.getItem(STORAGE_KEYS.theme) || "dark-sherwood",
      fontSize: parseInt(localStorage.getItem(STORAGE_KEYS.fontSize) || "13", 10),
      density: localStorage.getItem("vibeui-density") || "normal",
    };
    saveList([...customs, custom]);
    setName("");
    setMessage("Customization saved!");
    setTimeout(() => setMessage(""), 2000);
  };

  const loadCustom = (c: SavedCustomization) => {
    const theme = THEMES.find(t => t.id === c.theme);
    if (theme) {
      applyThemeById(theme.id);
    }
    localStorage.setItem(STORAGE_KEYS.fontSize, String(c.fontSize));
    document.documentElement.style.setProperty("--editor-font-size", `${c.fontSize}px`);
    localStorage.setItem("vibeui-density", c.density);
    setMessage(`Loaded "${c.name}"`);
    setTimeout(() => setMessage(""), 2000);
  };

  const deleteCustom = (id: string) => saveList(customs.filter(c => c.id !== id));

  const exportAll = () => {
    const blob = new Blob([JSON.stringify({ profile: localStorage.getItem(STORAGE_KEYS.profile), customs, oauth: localStorage.getItem(STORAGE_KEYS.oauth) }, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = "vibeui-settings.json"; a.click();
    URL.revokeObjectURL(url);
  };

  const importSettings = () => {
    const input = document.createElement("input");
    input.type = "file"; input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const data = JSON.parse(text);
        if (data.profile) localStorage.setItem(STORAGE_KEYS.profile, data.profile);
        if (data.oauth) localStorage.setItem(STORAGE_KEYS.oauth, data.oauth);
        if (data.customs) saveList(data.customs);
        setMessage("Settings imported!");
        setTimeout(() => setMessage(""), 2000);
      } catch {
        setMessage("Invalid file format");
        setTimeout(() => setMessage(""), 2000);
      }
    };
    input.click();
  };

  const resetAll = () => {
    if (!confirm("Reset all customizations and preferences to defaults?")) return;
    Object.values(STORAGE_KEYS).forEach(k => localStorage.removeItem(k));
    localStorage.removeItem("vibeui-density");
    document.documentElement.setAttribute("data-theme", "dark");
    // Clear inline styles
    const root = document.documentElement;
    if (THEMES[0].vars) Object.keys(THEMES[0].vars).forEach(k => root.style.removeProperty(k));
    root.style.removeProperty("--editor-font-size");
    root.style.removeProperty("--density-spacing");
    setCustoms([]);
    setMessage("All settings reset to defaults");
    setTimeout(() => setMessage(""), 2000);
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Saved Customizations</h3>

      {/* Save current */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input className="panel-input" style={{ flex: 1 }} placeholder="Customization name..." value={name} onChange={e => setName(e.target.value)} onKeyDown={e => e.key === "Enter" && saveCurrentPrefs()} />
        <button className="panel-btn panel-btn-primary" onClick={saveCurrentPrefs}><Save size={14} /> Save Current</button>
      </div>

      {message && <div style={{ padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--success-bg)", color: "var(--success-color)", fontSize: "var(--font-size-base)", marginBottom: 12 }}>{message}</div>}

      {/* Saved list */}
      {customs.length === 0 ? (
        <div style={{ padding: 20, textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>No saved customizations yet. Save your current setup above.</div>
      ) : (
        customs.map(c => (
          <div key={c.id} className="panel-card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{c.name}</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                {THEMES.find(t => t.id === c.theme)?.name || c.theme} · {c.fontSize}px · {c.density} · {new Date(c.createdAt).toLocaleDateString()}
              </div>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)" }} onClick={() => loadCustom(c)}>Load</button>
              <button className="panel-btn panel-btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }} onClick={() => deleteCustom(c.id)}>Delete</button>
            </div>
          </div>
        ))
      )}

      <div className="panel-divider" />

      {/* Import / Export / Reset */}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <button className="panel-btn panel-btn-secondary" onClick={exportAll}><Download size={13} /> Export All</button>
        <button className="panel-btn panel-btn-secondary" onClick={importSettings}><Upload size={13} /> Import</button>
        <button className="panel-btn panel-btn-secondary" style={{ color: "var(--error-color)" }} onClick={resetAll}><RotateCcw size={13} /> Reset All</button>
      </div>
    </div>
  );
}

interface ApiKeyValidation {
  provider: string;
  valid: boolean;
  error: string | null;
  latency_ms: number;
}

function ApiKeysSection() {
  const [settings, setSettings] = useState<ApiKeySettings>({
    anthropic_api_key: "", openai_api_key: "", gemini_api_key: "", grok_api_key: "", groq_api_key: "",
    openrouter_api_key: "", azure_openai_api_key: "", azure_openai_api_url: "",
    mistral_api_key: "", cerebras_api_key: "", deepseek_api_key: "", zhipu_api_key: "",
    vercel_ai_api_key: "", vercel_ai_api_url: "", minimax_api_key: "", perplexity_api_key: "",
    together_api_key: "", fireworks_api_key: "", sambanova_api_key: "",
    ollama_api_key: "", ollama_api_url: "",
    claude_model: "claude-3-5-sonnet-latest", openai_model: "gpt-4o", openrouter_model: "",
  });
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [validations, setValidations] = useState<Record<string, ApiKeyValidation>>({});
  const [validating, setValidating] = useState<Record<string, boolean>>({});
  const loadedRef = useRef(false);
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load keys on mount
  useEffect(() => {
    let cancelled = false;
    invoke<ApiKeySettings>("get_provider_api_keys")
      .then(s => {
        if (!cancelled) {
          setSettings(s);
          // Mark as loaded after a tick so the auto-save effect skips the initial set
          setTimeout(() => { loadedRef.current = true; }, 0);
        }
      })
      .catch(() => { loadedRef.current = true; });
    return () => { cancelled = true; };
  }, []);

  // Auto-save: debounce 1s after any settings change (skip the initial load)
  useEffect(() => {
    if (!loadedRef.current) return;
    if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current);
    autoSaveTimerRef.current = setTimeout(async () => {
      try {
        const providers = await invoke<string[]>("save_provider_api_keys", { settings });
        window.dispatchEvent(new CustomEvent("vibeui:providers-updated", { detail: providers }));
        setMessage({ type: "success", text: `Auto-saved. ${providers.length} model(s) available.` });
      } catch (_e) {
        setMessage({ type: "error", text: String(_e) });
      }
    }, 1000);
    return () => { if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current); };
  }, [settings]);

  // Listen for app-level validation events from useApiKeyMonitor (runs in App.tsx)
  useEffect(() => {
    const onValidations = (e: Event) => {
      const map = (e as CustomEvent<Record<string, ApiKeyValidation>>).detail;
      setValidations(map);
    };
    window.addEventListener("vibeui:api-key-validations", onValidations);

    // Also trigger an initial validation via Tauri for immediate feedback when panel opens
    invoke<ApiKeyValidation[]>("validate_all_api_keys")
      .then(results => {
        const map: Record<string, ApiKeyValidation> = {};
        results.forEach(r => { map[r.provider] = r; });
        setValidations(map);
      })
      .catch(() => {});

    return () => window.removeEventListener("vibeui:api-key-validations", onValidations);
  }, []);

  const validateSingle = async (provider: string, key: string, url?: string) => {
    setValidating(prev => ({ ...prev, [provider]: true }));
    try {
      const result = await invoke<ApiKeyValidation>("validate_api_key", {
        provider, apiKey: key, apiUrl: url ?? null,
      });
      setValidations(prev => ({ ...prev, [provider]: result }));
    } catch {
      setValidations(prev => ({
        ...prev,
        [provider]: { provider, valid: false, error: "Validation failed", latency_ms: 0 },
      }));
    } finally {
      setValidating(prev => ({ ...prev, [provider]: false }));
    }
  };

  const handleSave = async () => {
    setSaving(true); setMessage(null);
    try {
      const providers = await invoke<string[]>("save_provider_api_keys", { settings });
      // Emit a custom event so App.tsx can refresh its provider dropdown
      window.dispatchEvent(new CustomEvent("vibeui:providers-updated", { detail: providers }));
      setMessage({ type: "success", text: `Saved. ${providers.length} model(s) available.` });
    } catch (_e) {
      setMessage({ type: "error", text: String(_e) });
    } finally { setSaving(false); }
  };

  const renderSecretField = (label: string, fieldKey: keyof ApiKeySettings, placeholder: string, provider?: string) => {
    const v = provider ? validations[provider] : undefined;
    const isValidating = provider ? validating[provider] : false;
    return (
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label className="panel-label">{label}</label>
          {v && (
            <span style={{
              fontSize: "var(--font-size-xs)", fontWeight: 600, display: "inline-flex", alignItems: "center", gap: 4,
              color: v.valid ? "var(--accent-green)" : (v.error === "No key configured" ? "var(--text-secondary)" : "var(--accent-rose)"),
            }}>
              {v.valid
                ? <><CheckCircle size={10} strokeWidth={2} /> OK ({v.latency_ms}ms)</>
                : v.error === "No key configured" && provider !== "ollama"
                  ? <><MinusCircle size={10} strokeWidth={2} /> Not set</>
                  : v.error === "No key configured" && provider === "ollama"
                    ? <><MinusCircle size={10} strokeWidth={2} /> Using device key</>
                    : <><AlertCircle size={10} strokeWidth={2} /> {v.error}</>
              }
            </span>
          )}
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <input
            type={showKey[fieldKey] ? "text" : "password"}
            value={settings[fieldKey]}
            onChange={e => setSettings({ ...settings, [fieldKey]: e.target.value })}
            placeholder={placeholder}
            className="panel-input"
            style={{ flex: 1, fontFamily: "var(--font-mono)" }}
          />
          <button onClick={() => setShowKey({ ...showKey, [fieldKey]: !showKey[fieldKey] })} className="panel-btn panel-btn-secondary" style={{ padding: "4px 8px", display: "flex", alignItems: "center" }}>
            {showKey[fieldKey] ? <EyeOff size={14} /> : <Eye size={14} />}
          </button>
          {provider && (settings[fieldKey] || provider === "ollama") && (
            <button
              onClick={() => validateSingle(provider, settings[fieldKey] || "", provider === "ollama" ? settings.ollama_api_url : provider === "azure_openai" ? settings.azure_openai_api_url : provider === "vercel_ai" ? settings.vercel_ai_api_url : undefined)}
              disabled={isValidating}
              className="panel-btn panel-btn-secondary"
              style={{ padding: "4px 8px", display: "flex", alignItems: "center", gap: 4, fontSize: "var(--font-size-sm)", opacity: isValidating ? 0.5 : 1 }}
            >
              {isValidating ? <Loader2 size={12} strokeWidth={2} className="spin" /> : <Zap size={12} strokeWidth={2} />}
              Test
            </button>
          )}
        </div>
      </div>
    );
  };

  const renderSectionHeader = (title: string) => (
    <div style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 10, borderBottom: "1px solid var(--border-color)", paddingBottom: 4 }}>
      {title}
    </div>
  );

  return (
    <div>
      <h3 style={{ margin: "0 0 8px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>API Keys (BYOK)</h3>
      <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 18, lineHeight: 1.5 }}>
        Keys stored securely in <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: 3 }}>~/.vibecli/profile_settings.db</code>. Leave empty to disable.
      </p>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Anthropic (Claude)")}
        {renderSecretField("API Key", "anthropic_api_key", "sk-ant-api03-...", "anthropic")}
        <p style={modelsHintStyle}>
          Models: Opus 4.6, Sonnet 4.6, Haiku 4.5, 3.5 Sonnet, 3.5 Haiku, 3 Opus
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("OpenAI")}
        {renderSecretField("API Key", "openai_api_key", "sk-proj-...", "openai")}
        <p style={modelsHintStyle}>
          Models: GPT-4o, GPT-4o mini, GPT-4 Turbo, GPT-4, GPT-3.5 Turbo, o1, o1-mini, o1-preview, o3-mini
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Google (Gemini)")}
        {renderSecretField("API Key", "gemini_api_key", "AIzaSy...", "gemini")}
        <p style={modelsHintStyle}>
          Models: 2.5 Pro, 2.5 Flash, 2.0 Flash, 2.0 Flash Lite, 1.5 Pro, 1.5 Flash
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("xAI (Grok)")}
        {renderSecretField("API Key", "grok_api_key", "xai-...", "grok")}
        <p style={modelsHintStyle}>
          Models: Grok-3, Grok-3 mini, Grok-2, Grok-2 mini
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Groq")}
        {renderSecretField("API Key", "groq_api_key", "gsk_...", "groq")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 8B, Mixtral 8x7B, Gemma 2 9B
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("OpenRouter")}
        {renderSecretField("API Key", "openrouter_api_key", "sk-or-v1-...", "openrouter")}
        <div style={{ marginBottom: 12 }}>
          <label className="panel-label">Model</label>
          <input className="panel-input panel-input-full" value={settings.openrouter_model} onChange={e => setSettings({ ...settings, openrouter_model: e.target.value })} placeholder="anthropic/claude-3.5-sonnet" />
        </div>
        <p style={modelsHintStyle}>
          Routes to 200+ models. Enter a model ID or browse at openrouter.ai/models
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Azure OpenAI")}
        {renderSecretField("API Key", "azure_openai_api_key", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", "azure_openai")}
        <div style={{ marginBottom: 12 }}>
          <label className="panel-label">Endpoint URL</label>
          <input className="panel-input panel-input-full" value={settings.azure_openai_api_url} onChange={e => setSettings({ ...settings, azure_openai_api_url: e.target.value })} placeholder="https://your-resource.openai.azure.com" />
        </div>
        <p style={modelsHintStyle}>
          Models: GPT-4o, GPT-4 Turbo, GPT-3.5 Turbo (via your Azure deployment)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Mistral AI")}
        {renderSecretField("API Key", "mistral_api_key", "...", "mistral")}
        <p style={modelsHintStyle}>
          Models: Mistral Large, Mistral Medium, Mistral Small, Codestral, Mistral Nemo
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Cerebras")}
        {renderSecretField("API Key", "cerebras_api_key", "csk-...", "cerebras")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 8B (ultra-fast inference)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("DeepSeek")}
        {renderSecretField("API Key", "deepseek_api_key", "sk-...", "deepseek")}
        <p style={modelsHintStyle}>
          Models: DeepSeek-V3, DeepSeek-Coder, DeepSeek-R1
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Zhipu (GLM)")}
        {renderSecretField("API Key", "zhipu_api_key", "id.secret", "zhipu")}
        <p style={modelsHintStyle}>
          Models: GLM-4, GLM-4V, GLM-3 Turbo (format: &quot;id.secret&quot;)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Vercel AI Gateway")}
        {renderSecretField("API Key", "vercel_ai_api_key", "...", "vercel_ai")}
        <div style={{ marginBottom: 12 }}>
          <label className="panel-label">Gateway URL</label>
          <input className="panel-input panel-input-full" value={settings.vercel_ai_api_url} onChange={e => setSettings({ ...settings, vercel_ai_api_url: e.target.value })} placeholder="https://gateway.vercel.ai/v1" />
        </div>
        <p style={modelsHintStyle}>
          Unified proxy to multiple providers. Requires API key and gateway URL.
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("MiniMax")}
        {renderSecretField("API Key", "minimax_api_key", "...", "minimax")}
        <p style={modelsHintStyle}>
          Models: abab6.5, abab6, abab5.5
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Perplexity")}
        {renderSecretField("API Key", "perplexity_api_key", "pplx-...", "perplexity")}
        <p style={modelsHintStyle}>
          Models: Sonar Pro, Sonar, Sonar Deep Research (search-augmented AI)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Together AI")}
        {renderSecretField("API Key", "together_api_key", "...", "together")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Mixtral 8x22B, Qwen 2.5, CodeLlama (open model hosting)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Fireworks AI")}
        {renderSecretField("API Key", "fireworks_api_key", "fw_...", "fireworks")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Mixtral MoE, FireFunction v2 (fast inference)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("SambaNova")}
        {renderSecretField("API Key", "sambanova_api_key", "...", "sambanova")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 405B (fast inference on custom silicon)
        </p>
      </div>

      <button className="panel-btn panel-btn-primary" style={{ width: "100%" }} onClick={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save & Apply"}
      </button>

      {message && (
        message.type === "error"
          ? <div className="panel-error" style={{ marginTop: 12 }}><span>{message.text}</span></div>
          : <div style={{ marginTop: 12, padding: "8px 12px", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", background: "var(--success-bg)", color: "var(--success-color)", border: "1px solid var(--success-color)" }}>
              OK {message.text}
            </div>
      )}

      <div style={{ marginTop: 24, borderTop: "1px solid var(--border-color)", paddingTop: 16 }}>
        {renderSectionHeader("Local Models (Ollama)")}
        {renderSecretField("API Key", "ollama_api_key", "Optional — leave empty to use device key", "ollama")}
        <div style={{ marginBottom: 12 }}>
          <label className="panel-label">API URL</label>
          <input className="panel-input panel-input-full" value={settings.ollama_api_url} onChange={e => setSettings({ ...settings, ollama_api_url: e.target.value })} placeholder="http://localhost:11434" />
        </div>
        <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
          If no API key is set, a device key derived from your hostname and username is used automatically. Set a key when connecting to a remote or secured Ollama instance.
        </p>
      </div>
    </div>
  );
}

/* ── Integrations Section ──────────────────────────────────────────── */

type IntegrationCategory = "email" | "calendar" | "projecttools" | "messaging" | "search" | "voice" | "smarthome" | "infra";

interface IntegrationField { key: string; label: string; placeholder: string; url?: boolean }

const INTEGRATION_CATEGORIES: {
  id: IntegrationCategory; label: string; icon: React.ReactNode;
  description: string; fields: IntegrationField[];
}[] = [
  {
    id: "email", label: "Email", icon: <Mail size={14} strokeWidth={1.5} />,
    description: "Gmail or Outlook access tokens for inbox, compose, and triage commands. Add the refresh token + OAuth client ID/secret to enable automatic token refresh — without these, the access token expires after ~60 minutes.",
    fields: [
      { key: "gmail_access_token", label: "Gmail Access Token", placeholder: "ya29.a0A..." },
      { key: "gmail_refresh_token", label: "Gmail Refresh Token", placeholder: "1//0g..." },
      { key: "gmail_oauth_client_id", label: "Gmail OAuth Client ID", placeholder: "xxxxx.apps.googleusercontent.com" },
      { key: "gmail_oauth_client_secret", label: "Gmail OAuth Client Secret", placeholder: "GOCSPX-..." },
      { key: "outlook_access_token", label: "Outlook Access Token", placeholder: "eyJ0eXAi..." },
      { key: "outlook_refresh_token", label: "Outlook Refresh Token", placeholder: "M.R3_BAY..." },
      { key: "outlook_oauth_client_id", label: "Outlook OAuth Client ID", placeholder: "00000000-0000-0000-0000-000000000000" },
      { key: "outlook_oauth_client_secret", label: "Outlook OAuth Client Secret", placeholder: "client secret value" },
    ],
  },
  {
    id: "calendar", label: "Calendar", icon: <CalendarDays size={14} strokeWidth={1.5} />,
    description: "Google Calendar or Outlook Calendar tokens for event management.",
    fields: [
      { key: "google_access_token", label: "Google Calendar Token", placeholder: "ya29.a0A..." },
      { key: "outlook_access_token", label: "Outlook Calendar Token", placeholder: "eyJ0eXAi..." },
    ],
  },
  {
    id: "projecttools", label: "Project Tools", icon: <ClipboardList size={14} strokeWidth={1.5} />,
    description: "Linear, Jira, Notion, Todoist API keys for issue tracking and task management.",
    fields: [
      { key: "linear_api_key", label: "Linear API Key", placeholder: "lin_api_..." },
      { key: "notion_api_key", label: "Notion API Key", placeholder: "secret_..." },
      { key: "todoist_api_key", label: "Todoist API Token", placeholder: "..." },
      { key: "jira_url", label: "Jira Instance URL", placeholder: "https://yourorg.atlassian.net", url: true },
      { key: "jira_email", label: "Jira Email", placeholder: "you@example.com" },
      { key: "jira_api_token", label: "Jira API Token", placeholder: "ATATT3x..." },
    ],
  },
  {
    id: "messaging", label: "Messaging", icon: <MessageSquare size={14} strokeWidth={1.5} />,
    description: "Tokens for Telegram, Slack, Discord, WhatsApp, Teams, and 20+ more messaging platforms.",
    fields: [
      { key: "telegram_token", label: "Telegram Bot Token", placeholder: "123456:ABC-DEF..." },
      { key: "slack_bot_token", label: "Slack Bot Token", placeholder: "xoxb-..." },
      { key: "slack_app_token", label: "Slack App Token (Socket Mode)", placeholder: "xapp-..." },
      { key: "discord_token", label: "Discord Bot Token", placeholder: "MTI..." },
      { key: "whatsapp_access_token", label: "WhatsApp Access Token", placeholder: "EAAGm..." },
      { key: "whatsapp_phone_number_id", label: "WhatsApp Phone Number ID", placeholder: "12345..." },
      { key: "teams_tenant_id", label: "MS Teams Tenant ID", placeholder: "xxxxxxxx-xxxx-..." },
      { key: "teams_client_id", label: "MS Teams Client ID", placeholder: "xxxxxxxx-xxxx-..." },
      { key: "teams_client_secret", label: "MS Teams Client Secret", placeholder: "..." },
      { key: "matrix_homeserver_url", label: "Matrix Homeserver URL", placeholder: "https://matrix.org", url: true },
      { key: "matrix_access_token", label: "Matrix Access Token", placeholder: "syt_..." },
      { key: "matrix_room_id", label: "Matrix Room ID", placeholder: "!roomid:server" },
      { key: "twilio_account_sid", label: "Twilio Account SID", placeholder: "ACxxxxxxxx..." },
      { key: "twilio_auth_token", label: "Twilio Auth Token", placeholder: "..." },
      { key: "twilio_from_number", label: "Twilio From Number", placeholder: "+1234567890" },
      { key: "signal_api_url", label: "Signal API URL", placeholder: "http://localhost:8080", url: true },
      { key: "signal_phone_number", label: "Signal Phone Number", placeholder: "+1234567890" },
    ],
  },
  {
    id: "search", label: "Search & Web", icon: <Search size={14} strokeWidth={1.5} />,
    description: "Tavily and Brave Search API keys for web-grounded AI responses.",
    fields: [
      { key: "tavily_api_key", label: "Tavily API Key", placeholder: "tvly-..." },
      { key: "brave_api_key", label: "Brave Search API Key", placeholder: "BSA..." },
    ],
  },
  {
    id: "voice", label: "Voice & Audio", icon: <Mic size={14} strokeWidth={1.5} />,
    description: "ElevenLabs API key for text-to-speech voice synthesis.",
    fields: [
      { key: "elevenlabs_api_key", label: "ElevenLabs API Key", placeholder: "sk_..." },
      { key: "elevenlabs_voice_id", label: "ElevenLabs Voice ID", placeholder: "21m00Tcm4TlvDq8i..." },
    ],
  },
  {
    id: "smarthome", label: "Smart Home", icon: <Home size={14} strokeWidth={1.5} />,
    description: "Home Assistant long-lived access token and instance URL.",
    fields: [
      { key: "home_assistant_url", label: "Home Assistant URL", placeholder: "http://homeassistant.local:8123", url: true },
      { key: "home_assistant_token", label: "Long-Lived Access Token", placeholder: "eyJhbGci..." },
    ],
  },
  {
    id: "infra", label: "Infrastructure", icon: <Server size={14} strokeWidth={1.5} />,
    description: "Container registry and OpenSandbox credentials.",
    fields: [
      { key: "github_token", label: "GitHub Token", placeholder: "ghp_..." },
      { key: "open_sandbox_api_key", label: "OpenSandbox API Key", placeholder: "..." },
      { key: "open_sandbox_api_url", label: "OpenSandbox API URL", placeholder: "https://api.opensandbox.dev", url: true },
      { key: "registry_url", label: "Container Registry URL", placeholder: "registry.example.com", url: true },
      { key: "registry_username", label: "Registry Username", placeholder: "..." },
      { key: "registry_password", label: "Registry Password", placeholder: "..." },
    ],
  },
];

function IntegrationsSection() {
  const [activeCat, setActiveCat] = useState<IntegrationCategory>("email");
  const [values, setValues] = useState<Record<string, string>>({});
  const [showField, setShowField] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState<Record<string, boolean>>({});
  const [saved, setSaved] = useState<Record<string, boolean>>({});
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const cat = INTEGRATION_CATEGORIES.find(c => c.id === activeCat)!;

  // Load tokens for the active category
  useEffect(() => {
    const fields = cat.fields.map(f => f.key);
    invoke<Record<string, string>>("integration_tokens_get", { category: activeCat, fields })
      .then(data => setValues(prev => ({ ...prev, ...data })))
      .catch(() => {});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeCat]);

  const saveField = async (field: string) => {
    setSaving(prev => ({ ...prev, [field]: true }));
    setMessage(null);
    try {
      await invoke("integration_token_set", { category: activeCat, field, value: values[field] ?? "" });
      setSaved(prev => ({ ...prev, [field]: true }));
      setMessage({ type: "success", text: `${field} saved encrypted.` });
      setTimeout(() => setSaved(prev => ({ ...prev, [field]: false })), 2000);
    } catch (_e) {
      setMessage({ type: "error", text: String(_e) });
    } finally {
      setSaving(prev => ({ ...prev, [field]: false })); }
  };

  const clearField = async (field: string) => {
    try {
      await invoke("integration_token_delete", { category: activeCat, field });
      setValues(prev => ({ ...prev, [field]: "" }));
      setMessage({ type: "success", text: `${field} cleared.` });
    } catch (_e) {
      setMessage({ type: "error", text: String(_e) });
    }
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 4px", fontSize: "var(--font-size-xl)", fontWeight: 700 }}>Integrations</h3>
      <p style={{ margin: "0 0 16px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
        Tokens saved here are encrypted in the local SQLite profile database — never stored in plaintext.
      </p>

      {/* Category tabs */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 20 }}>
        {INTEGRATION_CATEGORIES.map(c => (
          <button
            key={c.id}
            className={`panel-btn ${activeCat === c.id ? "panel-btn-primary" : "panel-btn-secondary"}`}
            style={{ fontSize: "var(--font-size-sm)", gap: 6 }}
            onClick={() => { setActiveCat(c.id); setMessage(null); }}
          >
            <span>{c.icon}</span> {c.label}
          </button>
        ))}
      </div>

      {/* Active category */}
      <div className="panel-card" style={{ marginBottom: 12 }}>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 16 }}>
          {cat.description}
        </div>
        {cat.fields.map(field => {
          const val = values[field.key] ?? "";
          const isVisible = showField[field.key];
          const isSaving = saving[field.key];
          const isSaved = saved[field.key];
          const hasValue = val.length > 0;
          return (
            <div key={field.key} style={{ marginBottom: 14 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
                <label className="panel-label">{field.label}</label>
                <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                  {hasValue && (
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--success-color)", display: "flex", alignItems: "center", gap: 2 }}>
                      <CheckCircle size={10} /> set
                    </span>
                  )}
                  {!hasValue && (
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-tertiary, var(--text-secondary))" }}>
                      <MinusCircle size={10} style={{ verticalAlign: "middle" }} /> not configured
                    </span>
                  )}
                </div>
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <input
                  type={isVisible || field.url ? "text" : "password"}
                  value={val}
                  placeholder={field.placeholder}
                  className="panel-input panel-input-full"
                  style={{ flex: 1, fontFamily: field.url ? "inherit" : "monospace", fontSize: "var(--font-size-sm)" }}
                  onChange={e => setValues(prev => ({ ...prev, [field.key]: e.target.value }))}
                  onKeyDown={e => { if (e.key === "Enter") saveField(field.key); }}
                />
                {!field.url && (
                  <button
                    className="panel-btn panel-btn-secondary panel-btn-sm"
                    onClick={() => setShowField(prev => ({ ...prev, [field.key]: !isVisible }))}
                    title={isVisible ? "Hide" : "Show"}
                  >
                    {isVisible ? <EyeOff size={13} /> : <Eye size={13} />}
                  </button>
                )}
                <button
                  className="panel-btn panel-btn-secondary panel-btn-sm"
                  onClick={() => saveField(field.key)}
                  disabled={isSaving}
                  title="Save encrypted"
                >
                  {isSaving ? <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} /> : isSaved ? <Check size={13} /> : <Save size={13} />}
                </button>
                {hasValue && (
                  <button
                    className="panel-btn panel-btn-secondary panel-btn-sm"
                    onClick={() => clearField(field.key)}
                    title="Clear token"
                    style={{ color: "var(--error-color)" }}
                  >
                    <X size={13} />
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {message && (
        <div className={`panel-card ${message.type === "error" ? "panel-error" : ""}`}
          style={{ fontSize: "var(--font-size-sm)", color: message.type === "error" ? "var(--error-color)" : "var(--success-color)", display: "flex", alignItems: "center", gap: 6 }}>
          {message.type === "success" ? <CheckCircle size={14} /> : <AlertCircle size={14} />}
          {message.text}
        </div>
      )}

      <div className="panel-card" style={{ marginTop: 8, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
        <strong>Storage:</strong> All tokens are encrypted with ChaCha20-Poly1305 in{" "}
        <code style={{ background: "var(--bg-tertiary, var(--bg-secondary))", padding: "1px 4px", borderRadius: "var(--radius-xs-plus)" }}>
          ~/.vibecli/profile_settings.db
        </code>
        {" "}— never in plaintext files or environment variables.
        The CLI resolution order is: <strong>Settings → config.toml → env var</strong>.
      </div>
    </div>
  );
}

/* ── Sessions Section ──────────────────────────────────────────────── */
//
// F2.1 — recap & resume Settings surface. Toggles for when a session recap
// is generated (tab close / idle), which generator is used, and whether
// the daemon auto-resumes the last session on startup. All four values
// persist to a single `vibeui-sessions` JSON blob in localStorage so a
// future migration only has to read one key. Spec:
// docs/design/recap-resume/01-session.md.

function SessionsSection() {
  const [settings, setSettings] = useState<SessionsSettings>(SESSIONS_DEFAULTS);

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.sessions);
    if (!stored) return;
    try {
      const parsed = JSON.parse(stored) as Partial<SessionsSettings>;
      setSettings({
        recapOnTabClose: typeof parsed.recapOnTabClose === "boolean" ? parsed.recapOnTabClose : SESSIONS_DEFAULTS.recapOnTabClose,
        recapOnIdle: typeof parsed.recapOnIdle === "boolean" ? parsed.recapOnIdle : SESSIONS_DEFAULTS.recapOnIdle,
        idleMinutes: typeof parsed.idleMinutes === "number" && parsed.idleMinutes > 0 ? parsed.idleMinutes : SESSIONS_DEFAULTS.idleMinutes,
        generator: parsed.generator === "llm" ? "llm" : "heuristic",
        autoResumeLast: typeof parsed.autoResumeLast === "boolean" ? parsed.autoResumeLast : SESSIONS_DEFAULTS.autoResumeLast,
      });
    } catch {
      // corrupt blob → keep defaults; do not throw at the user
    }
  }, []);

  const update = useCallback(<K extends keyof SessionsSettings>(key: K, value: SessionsSettings[K]) => {
    setSettings(prev => {
      const next = { ...prev, [key]: value };
      localStorage.setItem(STORAGE_KEYS.sessions, JSON.stringify(next));
      return next;
    });
  }, []);

  const rowStyle: React.CSSProperties = {
    display: "flex", justifyContent: "space-between", alignItems: "center",
    padding: "12px 0", borderBottom: "1px solid var(--border-color)",
  };
  const labelStyle: React.CSSProperties = { fontSize: "var(--font-size-md)", color: "var(--text-primary)" };
  const hintStyle: React.CSSProperties = { fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 };

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Sessions</h3>
      <p style={{ ...hintStyle, marginBottom: 16 }}>
        Recap captures what each session accomplished so you can resume — or hand off — without re-reading the transcript.
      </p>

      {/* Recap on tab close */}
      <div style={rowStyle}>
        <div>
          <div style={labelStyle}>Recap on tab close</div>
          <div style={hintStyle}>Save a recap whenever you close a chat tab.</div>
        </div>
        <input
          type="checkbox"
          aria-label="Recap on tab close"
          checked={settings.recapOnTabClose}
          onChange={e => update("recapOnTabClose", e.target.checked)}
          style={{ width: 18, height: 18, accentColor: "var(--accent-color)" }}
        />
      </div>

      {/* Recap on idle */}
      <div style={rowStyle}>
        <div>
          <div style={labelStyle}>Recap on idle</div>
          <div style={hintStyle}>
            After {settings.idleMinutes} min of inactivity, generate a recap in the background.
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <input
            type="number"
            min={1}
            max={1440}
            aria-label="Idle minutes"
            value={settings.idleMinutes}
            disabled={!settings.recapOnIdle}
            onChange={e => {
              const n = parseInt(e.target.value, 10);
              if (Number.isFinite(n) && n > 0) update("idleMinutes", n);
            }}
            style={{
              width: 64, padding: "4px 6px",
              background: "var(--bg-secondary)", color: "var(--text-primary)",
              border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)",
              opacity: settings.recapOnIdle ? 1 : 0.5,
            }}
          />
          <input
            type="checkbox"
            aria-label="Recap on idle"
            checked={settings.recapOnIdle}
            onChange={e => update("recapOnIdle", e.target.checked)}
            style={{ width: 18, height: 18, accentColor: "var(--accent-color)" }}
          />
        </div>
      </div>

      {/* Recap generator */}
      <div style={rowStyle}>
        <div>
          <div style={labelStyle}>Recap generator</div>
          <div style={hintStyle}>
            Heuristic is instant and offline. LLM uses your currently selected provider.
          </div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          {(["heuristic", "llm"] as const).map(g => (
            <button
              key={g}
              type="button"
              aria-label={`Generator: ${g}`}
              aria-pressed={settings.generator === g}
              onClick={() => update("generator", g)}
              className={`panel-tab ${settings.generator === g ? "active" : ""}`}
              style={{ textTransform: "capitalize", minWidth: 96 }}
            >
              {g === "llm" ? "LLM" : "Heuristic"}
            </button>
          ))}
        </div>
      </div>

      {/* Auto-resume last session */}
      <div style={rowStyle}>
        <div>
          <div style={labelStyle}>Auto-resume last session on startup</div>
          <div style={hintStyle}>Open the most recent session automatically when VibeUI launches.</div>
        </div>
        <input
          type="checkbox"
          aria-label="Auto-resume last session on startup"
          checked={settings.autoResumeLast}
          onChange={e => update("autoResumeLast", e.target.checked)}
          style={{ width: 18, height: 18, accentColor: "var(--accent-color)" }}
        />
      </div>
    </div>
  );
}

/* ── Main Settings Panel ───────────────────────────────────────────── */

const SECTIONS: { key: SettingsSection; label: string; icon: React.ReactNode }[] = [
  { key: "profile", label: "Profile", icon: <User size={16} /> },
  { key: "appearance", label: "Appearance", icon: <Palette size={16} /> },
  { key: "oauth", label: "OAuth Login", icon: <LogIn size={16} /> },
  { key: "customizations", label: "Customizations", icon: <Save size={16} /> },
  { key: "apikeys", label: "API Keys", icon: <Key size={16} /> },
  { key: "integrations", label: "Integrations", icon: <Plug size={16} /> },
  { key: "sessions", label: "Sessions", icon: <MessageSquare size={16} /> },
];

export function SettingsPanel({ onClose }: { onClose?: () => void }) {
  const [section, setSection] = useState<SettingsSection>("profile");

  return (
    <div className="panel-container" style={{
      flexDirection: "row",
      borderRadius: "var(--radius-lg)", border: "1px solid var(--border-color)",
      boxShadow: "var(--elevation-3)",
    }}>
      {/* Sidebar nav */}
      <div style={{
        width: 200, background: "var(--bg-secondary)", borderRight: "1px solid var(--border-color)",
        display: "flex", flexDirection: "column", padding: "12px 8px",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 8px", marginBottom: 12 }}>
          <span style={{ fontWeight: 700, fontSize: "var(--font-size-lg)", color: "var(--accent-color)" }}>Settings</span>
          {onClose && <button className="panel-btn" onClick={onClose} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }} aria-label="Close settings"><X size={16} /></button>}
        </div>
        {SECTIONS.map(s => (
          <button className="panel-btn" key={s.key} style={sectionBtnStyle(section === s.key)} onClick={() => setSection(s.key)}>
            {s.icon}
            <span>{s.label}</span>
            {section === s.key && <ChevronRight size={14} style={{ marginLeft: "auto", opacity: 0.5 }} />}
          </button>
        ))}
      </div>

      {/* Content */}
      <div style={{ flex: 1, padding: 24, overflowY: "auto" }}>
        {section === "profile" && <ProfileSection />}
        {section === "appearance" && <AppearanceSection />}
        {section === "oauth" && <OAuthSection />}
        {section === "customizations" && <CustomizationsSection />}
        {section === "apikeys" && <ApiKeysSection />}
        {section === "integrations" && <IntegrationsSection />}
        {section === "sessions" && <SessionsSection />}
      </div>
    </div>
  );
}

export default SettingsPanel;
