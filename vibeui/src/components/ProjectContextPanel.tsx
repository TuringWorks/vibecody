import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * ProjectContextPanel — Smart project understanding dashboard.
 *
 * Displays the auto-detected project profile including:
 * - Languages, frameworks, architecture type
 * - Build, test, and lint commands (one-click run)
 * - Key files with previews
 * - Entry points and environment variables
 * - Project summary injected into every AI conversation
 *
 * This is VibeCody's answer to Cursor's "always-on project understanding"
 * and Windsurf's "Fast Context" — but with full transparency and control.
 */

interface BuildCommand {
  label: string;
  command: string;
  working_dir: string | null;
}

interface TestCommand {
  label: string;
  command: string;
  framework: string;
}

interface LintCommand {
  label: string;
  command: string;
}

interface KeyFile {
  path: string;
  role: string;
  preview: string;
}

interface ProjectProfile {
  name: string;
  description: string;
  languages: string[];
  frameworks: string[];
  architecture: string;
  build_commands: BuildCommand[];
  test_commands: TestCommand[];
  lint_commands: LintCommand[];
  key_files: KeyFile[];
  entry_points: string[];
  package_managers: string[];
  env_vars: string[];
  summary: string;
  scanned_at: number;
}

type Tab = "overview" | "commands" | "files" | "context";

export function ProjectContextPanel({ workspacePath }: { workspacePath?: string | null }) {
  const [tab, setTab] = useState<Tab>("overview");
  const [profile, setProfile] = useState<ProjectProfile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [runOutput, setRunOutput] = useState<string | null>(null);

  const wp = workspacePath || "";

  const loadProfile = useCallback(async () => {
    if (!wp) return;
    setLoading(true);
    setError("");
    try {
      const result = await invoke<string>("read_file_content", { path: `${wp}/.vibecli/project-profile.json` });
      setProfile(JSON.parse(result));
    } catch {
      // No cached profile — try to scan
      try {
        const scanned = await invoke<ProjectProfile>("scan_project_profile", { workspacePath: wp });
        setProfile(scanned);
      } catch (e) {
        setError("No project profile found. Open a project folder and run /init in the CLI.");
      }
    }
    setLoading(false);
  }, [wp]);

  useEffect(() => { loadProfile(); }, [loadProfile]);

  const runCommand = async (cmd: string) => {
    setRunOutput(`Running: ${cmd}...`);
    try {
      const result = await invoke<string>("run_terminal_command", { command: cmd, workspacePath: wp });
      setRunOutput(result);
    } catch (e) {
      setRunOutput(`Error: ${e}`);
    }
  };

  const rescan = async () => {
    setLoading(true);
    setError("");
    try {
      const scanned = await invoke<ProjectProfile>("scan_project_profile", { workspacePath: wp });
      setProfile(scanned);
    } catch (e) {
      setError(`Scan failed: ${e}`);
    }
    setLoading(false);
  };

  const tabStyle = (t: Tab): React.CSSProperties => ({
    padding: "6px 14px",
    cursor: "pointer",
    borderBottom: tab === t ? "2px solid var(--accent-color, #007acc)" : "2px solid transparent",
    background: "none",
    border: "none",
    color: tab === t ? "var(--text-primary)" : "var(--text-secondary)",
    fontWeight: tab === t ? 600 : 400,
    fontSize: "13px",
  });

  const sectionStyle: React.CSSProperties = {
    padding: "12px 16px",
    overflowY: "auto",
    flex: 1,
  };

  const badgeStyle: React.CSSProperties = {
    display: "inline-block",
    padding: "2px 8px",
    borderRadius: "12px",
    background: "var(--badge-bg, #264f78)",
    color: "var(--badge-fg, #fff)",
    fontSize: "12px",
    marginRight: "6px",
    marginBottom: "4px",
  };

  const cmdBtnStyle: React.CSSProperties = {
    padding: "4px 10px",
    cursor: "pointer",
    background: "var(--accent-blue)",
    color: "var(--button-fg, #fff)",
    border: "none",
    borderRadius: "4px",
    fontSize: "12px",
    marginLeft: "8px",
  };

  if (loading) {
    return <div style={sectionStyle}>Scanning project...</div>;
  }
  if (error) {
    return (
      <div style={sectionStyle}>
        <p style={{ color: "var(--error-color, #f44)" }}>{error}</p>
        <button onClick={rescan} style={cmdBtnStyle}>Scan Now</button>
      </div>
    );
  }
  if (!profile) {
    return (
      <div style={sectionStyle}>
        <p>No project profile loaded. Open a workspace folder first.</p>
        <button onClick={rescan} style={cmdBtnStyle}>Scan Project</button>
      </div>
    );
  }

  const scannedDate = new Date(profile.scanned_at * 1000).toLocaleString();

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
        <button style={tabStyle("overview")} onClick={() => setTab("overview")}>Overview</button>
        <button style={tabStyle("commands")} onClick={() => setTab("commands")}>Commands</button>
        <button style={tabStyle("files")} onClick={() => setTab("files")}>Key Files</button>
        <button style={tabStyle("context")} onClick={() => setTab("context")}>AI Context</button>
        <div style={{ marginLeft: "auto", padding: "6px 12px" }}>
          <button onClick={rescan} style={{ ...cmdBtnStyle, background: "transparent", color: "var(--text-secondary)" }}>
            Rescan
          </button>
        </div>
      </div>

      {/* Tab content */}
      <div style={sectionStyle}>
        {tab === "overview" && (
          <div>
            <h3 style={{ margin: "0 0 8px 0", fontSize: "16px" }}>{profile.name}</h3>
            {profile.description && <p style={{ color: "var(--text-secondary)", margin: "0 0 12px 0" }}>{profile.description}</p>}

            <div style={{ marginBottom: "12px" }}>
              <strong>Architecture:</strong> <span style={badgeStyle}>{profile.architecture}</span>
            </div>

            <div style={{ marginBottom: "12px" }}>
              <strong>Languages:</strong>{" "}
              {profile.languages.map((l) => <span key={l} style={badgeStyle}>{l}</span>)}
            </div>

            {profile.frameworks.length > 0 && (
              <div style={{ marginBottom: "12px" }}>
                <strong>Frameworks:</strong>{" "}
                {profile.frameworks.map((f) => <span key={f} style={badgeStyle}>{f}</span>)}
              </div>
            )}

            {profile.package_managers.length > 0 && (
              <div style={{ marginBottom: "12px" }}>
                <strong>Package managers:</strong>{" "}
                {profile.package_managers.map((p) => <span key={p} style={badgeStyle}>{p}</span>)}
              </div>
            )}

            {profile.entry_points.length > 0 && (
              <div style={{ marginBottom: "12px" }}>
                <strong>Entry points:</strong>{" "}
                {profile.entry_points.map((e) => (
                  <code key={e} style={{ marginRight: "8px", fontSize: "12px", color: "var(--link-color, #4ec9b0)" }}>{e}</code>
                ))}
              </div>
            )}

            {profile.env_vars.length > 0 && (
              <div style={{ marginBottom: "12px" }}>
                <strong>Environment variables:</strong>{" "}
                {profile.env_vars.map((v) => <code key={v} style={{ marginRight: "8px", fontSize: "12px" }}>{v}</code>)}
              </div>
            )}

            <div style={{ color: "var(--text-secondary)", fontSize: "11px", marginTop: "16px" }}>
              Last scanned: {scannedDate}
            </div>
          </div>
        )}

        {tab === "commands" && (
          <div>
            {profile.build_commands.length > 0 && (
              <div style={{ marginBottom: "16px" }}>
                <h4 style={{ margin: "0 0 8px 0" }}>Build</h4>
                {profile.build_commands.map((cmd, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "6px" }}>
                    <code style={{ flex: 1, fontSize: "12px" }}>{cmd.command}</code>
                    <button style={cmdBtnStyle} onClick={() => runCommand(cmd.command)}>Run</button>
                  </div>
                ))}
              </div>
            )}

            {profile.test_commands.length > 0 && (
              <div style={{ marginBottom: "16px" }}>
                <h4 style={{ margin: "0 0 8px 0" }}>Test</h4>
                {profile.test_commands.map((cmd, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "6px" }}>
                    <code style={{ flex: 1, fontSize: "12px" }}>{cmd.command}</code>
                    <span style={{ ...badgeStyle, marginLeft: "8px" }}>{cmd.framework}</span>
                    <button style={cmdBtnStyle} onClick={() => runCommand(cmd.command)}>Run</button>
                  </div>
                ))}
              </div>
            )}

            {profile.lint_commands.length > 0 && (
              <div style={{ marginBottom: "16px" }}>
                <h4 style={{ margin: "0 0 8px 0" }}>Lint / Format</h4>
                {profile.lint_commands.map((cmd, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "6px" }}>
                    <code style={{ flex: 1, fontSize: "12px" }}>{cmd.command}</code>
                    <button style={cmdBtnStyle} onClick={() => runCommand(cmd.command)}>Run</button>
                  </div>
                ))}
              </div>
            )}

            {runOutput && (
              <div style={{ marginTop: "12px" }}>
                <h4 style={{ margin: "0 0 4px 0" }}>Output</h4>
                <pre style={{
                  background: "var(--bg-primary)",
                  padding: "8px",
                  borderRadius: "4px",
                  fontSize: "12px",
                  maxHeight: "200px",
                  overflow: "auto",
                  whiteSpace: "pre-wrap",
                }}>{runOutput}</pre>
              </div>
            )}
          </div>
        )}

        {tab === "files" && (
          <div>
            {profile.key_files.length === 0 ? (
              <p>No key files detected.</p>
            ) : (
              profile.key_files.map((kf, i) => (
                <div key={i} style={{ marginBottom: "16px" }}>
                  <div style={{ display: "flex", alignItems: "center", marginBottom: "4px" }}>
                    <strong style={{ fontSize: "13px" }}>{kf.path}</strong>
                    <span style={{ ...badgeStyle, marginLeft: "8px" }}>{kf.role}</span>
                  </div>
                  <pre style={{
                    background: "var(--bg-primary)",
                    padding: "8px",
                    borderRadius: "4px",
                    fontSize: "11px",
                    maxHeight: "150px",
                    overflow: "auto",
                    whiteSpace: "pre-wrap",
                    margin: 0,
                  }}>{kf.preview || "(empty)"}</pre>
                </div>
              ))
            )}
          </div>
        )}

        {tab === "context" && (
          <div>
            <h4 style={{ margin: "0 0 8px 0" }}>AI System Prompt Context</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", marginBottom: "12px" }}>
              This context is automatically injected into every AI agent conversation.
              It gives the AI deep understanding of your project without manual @-mentions.
            </p>
            <pre style={{
              background: "var(--bg-primary)",
              padding: "12px",
              borderRadius: "4px",
              fontSize: "12px",
              maxHeight: "400px",
              overflow: "auto",
              whiteSpace: "pre-wrap",
            }}>{profile.summary}</pre>
          </div>
        )}
      </div>
    </div>
  );
}
