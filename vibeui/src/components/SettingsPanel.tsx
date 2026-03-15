import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ApiKeySettings {
    anthropic_api_key: string;
    openai_api_key: string;
    gemini_api_key: string;
    grok_api_key: string;
    claude_model: string;
    openai_model: string;
}

export function SettingsPanel() {
    const [settings, setSettings] = useState<ApiKeySettings>({
        anthropic_api_key: "",
        openai_api_key: "",
        gemini_api_key: "",
        grok_api_key: "",
        claude_model: "claude-3-5-sonnet-latest",
        openai_model: "gpt-4o",
    });
    const [saving, setSaving] = useState(false);
    const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
    const [showKey, setShowKey] = useState<Record<string, boolean>>({});

    useEffect(() => {
        let cancelled = false;
        invoke<ApiKeySettings>("get_provider_api_keys")
            .then((s) => { if (!cancelled) setSettings(s); })
            .catch((e: unknown) => { if (!cancelled) console.error("Failed to load API keys:", e); });
        return () => { cancelled = true; };
    }, []);

    const handleSave = async () => {
        setSaving(true);
        setMessage(null);
        try {
            await invoke("save_provider_api_keys", { settings });
            setMessage({ type: "success", text: "Settings saved. Providers re-registered." });
        } catch (e: unknown) {
            setMessage({ type: "error", text: String(e) });
        } finally {
            setSaving(false);
        }
    };

    const SecretField = ({
        label,
        fieldKey,
        placeholder,
    }: {
        label: string;
        fieldKey: keyof ApiKeySettings;
        placeholder: string;
    }) => (
        <div style={{ marginBottom: "12px" }}>
            <label style={{ display: "block", fontSize: "11px", color: "var(--text-secondary)", marginBottom: "4px" }}>
                {label}
            </label>
            <div style={{ display: "flex", gap: "6px" }}>
                <input
                    type={showKey[fieldKey] ? "text" : "password"}
                    value={settings[fieldKey]}
                    onChange={(e) => setSettings({ ...settings, [fieldKey]: e.target.value })}
                    placeholder={placeholder}
                    style={{
                        flex: 1, padding: "6px 8px", fontSize: "12px",
                        background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                        color: "var(--text-primary)", borderRadius: "4px", fontFamily: "monospace",
                    }}
                />
                <button
                    onClick={() => setShowKey({ ...showKey, [fieldKey]: !showKey[fieldKey] })}
                    style={{
                        padding: "4px 8px", background: "none",
                        border: "1px solid var(--border-color)",
                        color: "var(--text-secondary)", cursor: "pointer",
                        borderRadius: "4px", fontSize: "11px",
                    }}
                >
                    {showKey[fieldKey] ? "Hide" : "Show"}
                </button>
            </div>
        </div>
    );

    const TextField = ({
        label,
        fieldKey,
        placeholder,
    }: {
        label: string;
        fieldKey: keyof ApiKeySettings;
        placeholder: string;
    }) => (
        <div style={{ marginBottom: "12px" }}>
            <label style={{ display: "block", fontSize: "11px", color: "var(--text-secondary)", marginBottom: "4px" }}>
                {label}
            </label>
            <input
                type="text"
                value={settings[fieldKey]}
                onChange={(e) => setSettings({ ...settings, [fieldKey]: e.target.value })}
                placeholder={placeholder}
                style={{
                    width: "100%", boxSizing: "border-box",
                    padding: "6px 8px", fontSize: "12px",
                    background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                    color: "var(--text-primary)", borderRadius: "4px",
                }}
            />
        </div>
    );

    const SectionHeader = ({ title }: { title: string }) => (
        <div style={{
            fontSize: "10px", fontWeight: 600,
            color: "var(--text-secondary)", textTransform: "uppercase",
            letterSpacing: "0.07em", marginBottom: "10px",
            borderBottom: "1px solid var(--border-color)", paddingBottom: "4px",
        }}>
            {title}
        </div>
    );

    return (
        <div style={{ padding: "16px", overflowY: "auto", height: "100%", boxSizing: "border-box" }}>
            <h3 style={{ margin: "0 0 6px", fontSize: "14px", color: "var(--text-primary)", fontWeight: 600 }}>
                API Keys (BYOK)
            </h3>
            <p style={{ fontSize: "11px", color: "var(--text-secondary)", marginBottom: "18px", lineHeight: 1.5 }}>
                Keys are stored at <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: "3px" }}>~/.vibeui/api_keys.json</code>.<br />
                Leave a field empty to disable that provider.
            </p>

            {/* Anthropic */}
            <div style={{ marginBottom: "20px" }}>
                <SectionHeader title="Anthropic (Claude)" />
                <SecretField label="API Key" fieldKey="anthropic_api_key" placeholder="sk-ant-api03-..." />
                <TextField label="Model" fieldKey="claude_model" placeholder="claude-3-5-sonnet-latest" />
            </div>

            {/* OpenAI */}
            <div style={{ marginBottom: "20px" }}>
                <SectionHeader title="OpenAI" />
                <SecretField label="API Key" fieldKey="openai_api_key" placeholder="sk-proj-..." />
                <TextField label="Model" fieldKey="openai_model" placeholder="gpt-4o" />
            </div>

            {/* Gemini */}
            <div style={{ marginBottom: "20px" }}>
                <SectionHeader title="Google (Gemini)" />
                <SecretField label="API Key" fieldKey="gemini_api_key" placeholder="AIzaSy..." />
            </div>

            {/* Grok */}
            <div style={{ marginBottom: "20px" }}>
                <SectionHeader title="xAI (Grok)" />
                <SecretField label="API Key" fieldKey="grok_api_key" placeholder="xai-..." />
            </div>

            <button
                onClick={handleSave}
                disabled={saving}
                style={{
                    width: "100%", padding: "8px 12px",
                    background: "var(--accent-color)",
                    border: "none", color: "white", borderRadius: "4px",
                    fontSize: "13px", fontWeight: 500,
                    cursor: saving ? "not-allowed" : "pointer",
                    opacity: saving ? 0.7 : 1,
                }}
            >
                {saving ? "Saving…" : "Save & Apply"}
            </button>

            {message && (
                <div style={{
                    marginTop: "12px", padding: "8px 10px",
                    borderRadius: "4px", fontSize: "12px",
                    background: message.type === "success"
                        ? "rgba(56, 161, 105, 0.12)"
                        : "rgba(255,77,79,0.12)",
                    color: message.type === "success"
                        ? "var(--text-primary)"
                        : "var(--error-color)",
                    border: `1px solid ${message.type === "success"
                        ? "rgba(56,161,105,0.4)"
                        : "var(--error-color)"}`,
                }}>
                    {message.type === "success" ? "✓ " : "✗ "}{message.text}
                </div>
            )}

            <div style={{ marginTop: "24px", borderTop: "1px solid var(--border-color)", paddingTop: "16px" }}>
                <SectionHeader title="Local Models (Ollama)" />
                <p style={{ fontSize: "11px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                    Ollama models are auto-detected at <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: "3px" }}>http://localhost:11434</code>.<br />
                    Start Ollama locally and models will appear in the provider selector automatically.
                </p>
            </div>
        </div>
    );
}
