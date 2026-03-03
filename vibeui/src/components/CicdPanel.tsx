import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Platform = "github" | "gitlab" | "circleci" | "jenkins" | "bitbucket";
type BuildType = "rust" | "node" | "go" | "python" | "java" | "dotnet" | "unknown";
type SubTab = "config" | "release" | "secrets";

const PLATFORMS: { id: Platform; label: string; icon: string }[] = [
  { id: "github",    label: "GitHub Actions",       icon: "🐙" },
  { id: "gitlab",    label: "GitLab CI",             icon: "🦊" },
  { id: "circleci",  label: "CircleCI",              icon: "⭕" },
  { id: "jenkins",   label: "Jenkins",               icon: "🤖" },
  { id: "bitbucket", label: "Bitbucket Pipelines",   icon: "🪣" },
];

const BUILD_TYPES: { id: BuildType; label: string }[] = [
  { id: "rust",    label: "Rust" },
  { id: "node",    label: "Node.js" },
  { id: "go",      label: "Go" },
  { id: "python",  label: "Python" },
  { id: "java",    label: "Java" },
  { id: "dotnet",  label: ".NET" },
  { id: "unknown", label: "Generic" },
];

const RELEASE_TARGETS = [
  { id: "linux-x64",   label: "Linux x64" },
  { id: "linux-arm64", label: "Linux ARM64" },
  { id: "macos-arm64", label: "macOS ARM64" },
  { id: "macos-x64",   label: "macOS x64" },
  { id: "windows-x64", label: "Windows x64" },
];

const SECRETS_REFERENCE: { platform: Platform; secrets: string[] }[] = [
  { platform: "github",    secrets: ["CARGO_REGISTRY_TOKEN", "NPM_TOKEN", "DOCKER_USERNAME", "DOCKER_PASSWORD", "KUBE_CONFIG", "GH_TOKEN"] },
  { platform: "gitlab",    secrets: ["CI_REGISTRY_USER", "CI_REGISTRY_PASSWORD", "KUBE_CONFIG", "NPM_TOKEN"] },
  { platform: "circleci",  secrets: ["DOCKERHUB_USERNAME", "DOCKERHUB_PASSWORD", "KUBE_CONFIG", "NPM_TOKEN"] },
  { platform: "jenkins",   secrets: ["DOCKER_CREDENTIALS", "KUBE_CONFIG", "NPM_TOKEN", "SONAR_TOKEN"] },
  { platform: "bitbucket", secrets: ["DOCKER_HUB_USERNAME", "DOCKER_HUB_PASSWORD", "KUBE_CONFIG", "NPM_TOKEN"] },
];

const CICD_OUTPUT_PATHS: Record<Platform, string> = {
  github:    ".github/workflows/ci.yml",
  gitlab:    ".gitlab-ci.yml",
  circleci:  ".circleci/config.yml",
  jenkins:   "Jenkinsfile",
  bitbucket: "bitbucket-pipelines.yml",
};

interface CicdPanelProps {
  workspacePath: string | null;
}

export default function CicdPanel({ workspacePath }: CicdPanelProps) {
  const [subTab, setSubTab] = useState<SubTab>("config");
  const [platform, setPlatform] = useState<Platform>("github");
  const [buildType, setBuildType] = useState<BuildType>("unknown");
  const [preview, setPreview] = useState<string>("");
  const [writtenPath, setWrittenPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [releaseTargets, setReleaseTargets] = useState<Set<string>>(
    new Set(["linux-x64", "macos-arm64", "windows-x64"])
  );
  const [releasePreview, setReleasePreview] = useState<string>("");
  const [releaseWrittenPath, setReleaseWrittenPath] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [secretsCopied, setSecretsCopied] = useState<string | null>(null);

  // Auto-detect build type on mount
  useEffect(() => {
    if (!workspacePath) return;
    invoke<string>("detect_build_type", { workspace: workspacePath })
      .then((bt) => setBuildType(bt as BuildType))
      .catch(() => {});
  }, [workspacePath]);

  const handleGenerate = async () => {
    if (!workspacePath) { setError("No workspace open."); return; }
    setLoading(true);
    setError(null);
    setWrittenPath(null);
    try {
      const result = await invoke<string>("generate_cicd_config", {
        workspace: workspacePath,
        platform,
        buildType,
      });
      setPreview(result);
      setWrittenPath(CICD_OUTPUT_PATHS[platform]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleGenerateRelease = async () => {
    if (!workspacePath) { setError("No workspace open."); return; }
    setLoading(true);
    setError(null);
    setReleaseWrittenPath(null);
    try {
      const result = await invoke<string>("generate_release_workflow", {
        workspace: workspacePath,
        buildType,
        targets: Array.from(releaseTargets),
      });
      setReleasePreview(result);
      setReleaseWrittenPath(".github/workflows/release.yml");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  };

  const handleCopySecret = (secret: string) => {
    navigator.clipboard.writeText(secret).then(() => {
      setSecretsCopied(secret);
      setTimeout(() => setSecretsCopied(null), 1500);
    }).catch(() => {});
  };

  const toggleTarget = (id: string) => {
    setReleaseTargets((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const platformSecrets = SECRETS_REFERENCE.find((s) => s.platform === platform)?.secrets ?? [];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Sub-tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        {(["config", "release", "secrets"] as SubTab[]).map((t) => (
          <button
            key={t}
            onClick={() => setSubTab(t)}
            style={{
              padding: "6px 14px",
              fontSize: "12px",
              background: subTab === t ? "var(--bg-primary)" : "transparent",
              color: subTab === t ? "var(--text-primary)" : "var(--text-muted)",
              border: "none",
              borderBottom: subTab === t ? "2px solid var(--accent-color)" : "2px solid transparent",
              cursor: "pointer",
              fontWeight: subTab === t ? 600 : 400,
            }}
          >
            {t === "config" ? "⚙️ Config Generator" : t === "release" ? "📦 Binary Builds" : "🔑 Secrets"}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: "12px" }}>
        {error && (
          <div style={{ padding: "8px 12px", background: "var(--error-bg, #2a1a1a)", color: "var(--text-danger, #ff6b6b)", borderRadius: 4, marginBottom: 10, fontSize: 12 }}>
            {error}
          </div>
        )}

        {/* ── Config Generator ── */}
        {subTab === "config" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {/* Build type */}
            <div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>BUILD TYPE</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {BUILD_TYPES.map((bt) => (
                  <button
                    key={bt.id}
                    onClick={() => setBuildType(bt.id)}
                    style={{
                      padding: "4px 10px",
                      fontSize: 12,
                      borderRadius: 12,
                      border: "1px solid",
                      borderColor: buildType === bt.id ? "var(--accent-color)" : "var(--border-color)",
                      background: buildType === bt.id ? "var(--accent-color)" : "transparent",
                      color: buildType === bt.id ? "#fff" : "var(--text-secondary)",
                      cursor: "pointer",
                    }}
                  >
                    {bt.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Platform selector */}
            <div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>CI/CD PLATFORM</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {PLATFORMS.map((p) => (
                  <button
                    key={p.id}
                    onClick={() => setPlatform(p.id)}
                    title={p.label}
                    style={{
                      padding: "6px 12px",
                      fontSize: 12,
                      borderRadius: 6,
                      border: "1px solid",
                      borderColor: platform === p.id ? "var(--accent-color)" : "var(--border-color)",
                      background: platform === p.id ? "var(--accent-color)" : "var(--bg-secondary)",
                      color: platform === p.id ? "#fff" : "var(--text-secondary)",
                      cursor: "pointer",
                    }}
                  >
                    {p.icon} {p.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Generate button */}
            <button
              onClick={handleGenerate}
              disabled={loading || !workspacePath}
              style={{
                alignSelf: "flex-start",
                padding: "7px 16px",
                fontSize: 13,
                background: "var(--accent-color)",
                color: "#fff",
                border: "none",
                borderRadius: 6,
                cursor: loading ? "wait" : "pointer",
                opacity: loading ? 0.7 : 1,
              }}
            >
              {loading ? "⏳ Generating..." : "✨ Generate & Write"}
            </button>

            {/* Written notice */}
            {writtenPath && (
              <div style={{ fontSize: 12, color: "var(--text-success, #52c41a)" }}>
                ✅ Written to <code style={{ fontSize: 11 }}>{writtenPath}</code>
              </div>
            )}

            {/* Preview */}
            {preview && (
              <div style={{ position: "relative" }}>
                <button
                  onClick={() => handleCopy(preview)}
                  style={{
                    position: "absolute", top: 6, right: 6,
                    padding: "2px 8px", fontSize: 11, background: "var(--bg-tertiary, #333)",
                    color: "var(--text-secondary)", border: "1px solid var(--border-color)",
                    borderRadius: 4, cursor: "pointer", zIndex: 1,
                  }}
                >
                  {copied ? "✓ Copied" : "Copy"}
                </button>
                <pre style={{
                  margin: 0, padding: "12px 40px 12px 12px",
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-color)",
                  borderRadius: 6,
                  fontSize: 11,
                  lineHeight: 1.5,
                  overflowX: "auto",
                  maxHeight: 400,
                  overflowY: "auto",
                  whiteSpace: "pre",
                  color: "var(--text-primary)",
                }}>
                  {preview}
                </pre>
              </div>
            )}
          </div>
        )}

        {/* ── Binary Builds ── */}
        {subTab === "release" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              Generates a <strong>GitHub Actions release workflow</strong> triggered on version tags (<code>v*</code>).
              Builds binaries for each selected platform using cross-compilation.
            </div>

            {/* Build type (re-use from state) */}
            <div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>BUILD TYPE</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {BUILD_TYPES.filter(bt => bt.id !== "unknown").map((bt) => (
                  <button
                    key={bt.id}
                    onClick={() => setBuildType(bt.id)}
                    style={{
                      padding: "4px 10px", fontSize: 12, borderRadius: 12, border: "1px solid",
                      borderColor: buildType === bt.id ? "var(--accent-color)" : "var(--border-color)",
                      background: buildType === bt.id ? "var(--accent-color)" : "transparent",
                      color: buildType === bt.id ? "#fff" : "var(--text-secondary)",
                      cursor: "pointer",
                    }}
                  >{bt.label}</button>
                ))}
              </div>
            </div>

            {/* Target checkboxes */}
            <div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>TARGET PLATFORMS</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {RELEASE_TARGETS.map((t) => (
                  <label key={t.id} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, cursor: "pointer" }}>
                    <input
                      type="checkbox"
                      checked={releaseTargets.has(t.id)}
                      onChange={() => toggleTarget(t.id)}
                    />
                    {t.label}
                  </label>
                ))}
              </div>
            </div>

            <button
              onClick={handleGenerateRelease}
              disabled={loading || !workspacePath || releaseTargets.size === 0}
              style={{
                alignSelf: "flex-start", padding: "7px 16px", fontSize: 13,
                background: "var(--accent-color)", color: "#fff", border: "none",
                borderRadius: 6, cursor: loading ? "wait" : "pointer", opacity: loading ? 0.7 : 1,
              }}
            >
              {loading ? "⏳ Generating..." : "📦 Generate Release Workflow"}
            </button>

            {releaseWrittenPath && (
              <div style={{ fontSize: 12, color: "var(--text-success, #52c41a)" }}>
                ✅ Written to <code style={{ fontSize: 11 }}>{releaseWrittenPath}</code>
              </div>
            )}

            {releasePreview && (
              <pre style={{
                margin: 0, padding: 12,
                background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
                borderRadius: 6, fontSize: 11, lineHeight: 1.5,
                overflowX: "auto", maxHeight: 400, overflowY: "auto",
                whiteSpace: "pre", color: "var(--text-primary)",
              }}>
                {releasePreview}
              </pre>
            )}
          </div>
        )}

        {/* ── Secrets Reference ── */}
        {subTab === "secrets" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              Required secrets to configure in your CI/CD platform settings for the selected platform.
            </div>

            <div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>PLATFORM</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 12 }}>
                {PLATFORMS.map((p) => (
                  <button key={p.id} onClick={() => setPlatform(p.id)} style={{
                    padding: "4px 10px", fontSize: 12, borderRadius: 12, border: "1px solid",
                    borderColor: platform === p.id ? "var(--accent-color)" : "var(--border-color)",
                    background: platform === p.id ? "var(--accent-color)" : "transparent",
                    color: platform === p.id ? "#fff" : "var(--text-secondary)",
                    cursor: "pointer",
                  }}>{p.icon} {p.label}</button>
                ))}
              </div>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {platformSecrets.map((secret) => (
                <div key={secret} style={{
                  display: "flex", alignItems: "center", justifyContent: "space-between",
                  padding: "8px 12px",
                  background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
                  borderRadius: 6,
                }}>
                  <code style={{ fontSize: 12 }}>{secret}</code>
                  <button
                    onClick={() => handleCopySecret(secret)}
                    style={{
                      padding: "2px 8px", fontSize: 11,
                      background: secretsCopied === secret ? "var(--text-success, #52c41a)" : "var(--bg-tertiary, #333)",
                      color: secretsCopied === secret ? "#fff" : "var(--text-secondary)",
                      border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer",
                    }}
                  >
                    {secretsCopied === secret ? "✓" : "Copy"}
                  </button>
                </div>
              ))}
            </div>

            <div style={{ marginTop: 8, padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, fontSize: 12, color: "var(--text-muted)", lineHeight: 1.6 }}>
              <strong>How to add secrets:</strong><br />
              • <strong>GitHub</strong>: Settings → Secrets and variables → Actions → New repository secret<br />
              • <strong>GitLab</strong>: Settings → CI/CD → Variables → Add variable<br />
              • <strong>CircleCI</strong>: Project Settings → Environment Variables<br />
              • <strong>Jenkins</strong>: Manage Jenkins → Credentials → System → Global credentials<br />
              • <strong>Bitbucket</strong>: Repository Settings → Pipelines → Repository variables
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
