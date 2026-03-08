/**
 * CIReviewPanel — CI/CD AI Review Bot dashboard.
 *
 * Configure GitHub App webhook, view recent reviews, and monitor status.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GithubAppConfig {
  app_id: number;
  private_key_path: string | null;
  webhook_secret: string | null;
  auto_fix: boolean;
  severity_threshold: string;
}

interface CIReviewResult {
  pr_number: number;
  repo: string;
  commit_sha: string;
  findings_count: number;
  severity_counts: {
    critical: number;
    high: number;
    medium: number;
    low: number;
    info: number;
  };
  status: string;
  summary: string;
  timestamp: number;
}

export function CIReviewPanel() {
  const [config, setConfig] = useState<GithubAppConfig>({
    app_id: 0,
    private_key_path: null,
    webhook_secret: null,
    auto_fix: false,
    severity_threshold: "high",
  });
  const [reviews, setReviews] = useState<CIReviewResult[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    invoke<GithubAppConfig>("get_ci_review_config")
      .then(setConfig)
      .catch(() => {});
    invoke<CIReviewResult[]>("get_ci_review_history")
      .then(setReviews)
      .catch(() => {});
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      await invoke("save_ci_review_config", { config });
      setSuccess("Configuration saved");
      setTimeout(() => setSuccess(null), 3000);
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>CI/CD Review Bot</span>
        <div style={{ flex: 1 }} />
        <span style={{
          fontSize: 10, padding: "2px 8px", borderRadius: 10, fontWeight: 600,
          background: config.app_id > 0 ? "rgba(166,227,161,0.15)" : "rgba(108,112,134,0.15)",
          color: config.app_id > 0 ? "var(--success-color)" : "var(--text-muted)",
        }}>
          {config.app_id > 0 ? "Configured" : "Not configured"}
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {/* Configuration Section */}
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>GitHub App Configuration</div>
          <div style={{ fontSize: 10, color: "var(--text-secondary, #a6adc8)", marginBottom: 10 }}>
            Set up a GitHub App to auto-review PRs. The webhook endpoint will be at{" "}
            <code style={{ fontSize: 10 }}>/webhook/github</code> on your VibeCLI daemon.
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <div style={labelStyle}>App ID</div>
              <input
                type="number"
                value={config.app_id || ""}
                onChange={(e) => setConfig({ ...config, app_id: parseInt(e.target.value) || 0 })}
                placeholder="12345"
                style={inputStyle}
              />
            </div>

            <div>
              <div style={labelStyle}>Private Key Path</div>
              <input
                value={config.private_key_path || ""}
                onChange={(e) => setConfig({ ...config, private_key_path: e.target.value || null })}
                placeholder="/path/to/private-key.pem"
                style={inputStyle}
              />
            </div>

            <div>
              <div style={labelStyle}>Webhook Secret</div>
              <input
                type="password"
                value={config.webhook_secret || ""}
                onChange={(e) => setConfig({ ...config, webhook_secret: e.target.value || null })}
                placeholder="your-webhook-secret"
                style={inputStyle}
              />
            </div>

            <div>
              <div style={labelStyle}>Severity Threshold</div>
              <select
                value={config.severity_threshold}
                onChange={(e) => setConfig({ ...config, severity_threshold: e.target.value })}
                style={inputStyle}
              >
                <option value="critical">Critical only</option>
                <option value="high">High + Critical</option>
                <option value="medium">Medium and above</option>
                <option value="low">All findings</option>
              </select>
            </div>

            <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11 }}>
              <input
                type="checkbox"
                checked={config.auto_fix}
                onChange={(e) => setConfig({ ...config, auto_fix: e.target.checked })}
              />
              Auto-fix: push fixes to PR branch
            </label>

            <button onClick={handleSave} disabled={saving} style={{
              ...btnStyle, background: "var(--accent-primary, #6366f1)", color: "#fff", fontWeight: 700,
              opacity: saving ? 0.5 : 1,
            }}>
              {saving ? "Saving..." : "Save Configuration"}
            </button>

            {error && (
              <div style={{ fontSize: 11, color: "var(--text-danger, #f38ba8)", padding: "4px 8px", background: "rgba(243,139,168,0.05)", borderRadius: 4 }}>
                {error}
              </div>
            )}
            {success && (
              <div style={{ fontSize: 11, color: "var(--text-success, #a6e3a1)", padding: "4px 8px", background: "rgba(166,227,161,0.05)", borderRadius: 4 }}>
                {success}
              </div>
            )}
          </div>
        </div>

        {/* Setup Instructions */}
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>Setup Instructions</div>
          <div style={{
            fontSize: 10, padding: "8px 10px", borderRadius: 4,
            background: "var(--bg-primary)", lineHeight: 1.6,
          }}>
            1. Create a GitHub App at github.com/settings/apps/new<br />
            2. Set Webhook URL to: <code>https://your-server/webhook/github</code><br />
            3. Subscribe to <strong>Pull request</strong> events<br />
            4. Generate and download a private key (.pem)<br />
            5. Set permissions: Pull requests (Read & Write), Contents (Read)<br />
            6. Install the app on your repository<br />
            7. Run: <code>vibecli serve --port 7878</code>
          </div>
        </div>

        {/* CLI Usage */}
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>CLI CI Mode</div>
          <div style={{
            fontSize: 10, padding: "8px 10px", borderRadius: 4,
            background: "var(--bg-primary)", fontFamily: "monospace",
          }}>
            vibecli --review --ci-mode --base $BASE_SHA<br />
            vibecli --review --ci-mode --severity-threshold medium
          </div>
        </div>

        {/* Recent Reviews */}
        <div>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
            Recent Reviews ({reviews.length})
          </div>
          {reviews.length === 0 ? (
            <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
              No reviews yet. Reviews will appear here when the webhook receives PR events.
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {reviews.map((r, i) => (
                <div key={i} style={{
                  padding: "6px 8px", borderRadius: 4,
                  border: "1px solid var(--border-color)",
                  background: "var(--bg-primary)",
                }}>
                  <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 4 }}>
                    <span style={{
                      fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 700,
                      background: r.status === "success" ? "var(--success-color)" : "var(--error-color)",
                      color: "var(--bg-tertiary)",
                    }}>
                      {r.status}
                    </span>
                    <span style={{ fontSize: 11, fontWeight: 600 }}>
                      {r.repo} #{r.pr_number}
                    </span>
                    <div style={{ flex: 1 }} />
                    <span style={{ fontSize: 9, opacity: 0.5, fontFamily: "monospace" }}>
                      {new Date(r.timestamp * 1000).toLocaleString()}
                    </span>
                  </div>
                  <div style={{ fontSize: 10, opacity: 0.8 }}>{r.summary}</div>
                  <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
                    {r.severity_counts.critical > 0 && (
                      <span style={{ fontSize: 9, color: "var(--text-danger, #f38ba8)" }}>
                        {r.severity_counts.critical} critical
                      </span>
                    )}
                    {r.severity_counts.high > 0 && (
                      <span style={{ fontSize: 9, color: "var(--text-warning-alt, #fab387)" }}>
                        {r.severity_counts.high} high
                      </span>
                    )}
                    {r.severity_counts.medium > 0 && (
                      <span style={{ fontSize: 9, color: "var(--text-warning, #f9e2af)" }}>
                        {r.severity_counts.medium} medium
                      </span>
                    )}
                    {r.severity_counts.low > 0 && (
                      <span style={{ fontSize: 9, color: "var(--text-muted, #6c7086)" }}>
                        {r.severity_counts.low} low
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: 9, opacity: 0.4, marginTop: 2, fontFamily: "monospace" }}>
                    {r.commit_sha.slice(0, 8)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "6px 12px", fontSize: 12, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "5px 8px", fontSize: 11, borderRadius: 4, width: "100%", boxSizing: "border-box" as const,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};

const labelStyle: React.CSSProperties = {
  fontSize: 10, fontWeight: 600, marginBottom: 3,
  color: "var(--text-secondary, #a6adc8)",
  textTransform: "uppercase" as const, letterSpacing: "0.06em",
};
