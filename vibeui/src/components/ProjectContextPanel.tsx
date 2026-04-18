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
      } catch (_e) {
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
    } catch (_e) {
      setRunOutput(`Error: ${_e}`);
    }
  };

  const rescan = async () => {
    setLoading(true);
    setError("");
    try {
      const scanned = await invoke<ProjectProfile>("scan_project_profile", { workspacePath: wp });
      setProfile(scanned);
    } catch (_e) {
      setError(`Scan failed: ${_e}`);
    }
    setLoading(false);
  };

  const badgeStyle: React.CSSProperties = {
    display: "inline-block",
    padding: "2px 8px",
    borderRadius: "12px",
    background: "var(--badge-bg, #264f78)",
    color: "var(--badge-fg, #fff)",
    fontSize: "var(--font-size-base)",
    marginRight: "6px",
    marginBottom: "4px",
  };

  if (loading) {
    return <div className="panel-loading">Scanning project...</div>;
  }
  if (error) {
    return (
      <div className="panel-error" style={{ padding: "12px 16px" }}>
        <p style={{ color: "var(--error-color, #f44)", margin: "0 0 8px" }}>{error}</p>
        <button onClick={rescan} className="panel-btn panel-btn-primary">Scan Now</button>
      </div>
    );
  }
  if (!profile) {
    return (
      <div className="panel-empty">
        <p>No project profile loaded. Open a workspace folder first.</p>
        <button onClick={rescan} className="panel-btn panel-btn-primary">Scan Project</button>
      </div>
    );
  }

  const scannedDate = new Date(profile.scanned_at * 1000).toLocaleString();

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        <button className={`panel-tab${tab === "overview" ? " active" : ""}`} onClick={() => setTab("overview")}>Overview</button>
        <button className={`panel-tab${tab === "commands" ? " active" : ""}`} onClick={() => setTab("commands")}>Commands</button>
        <button className={`panel-tab${tab === "files" ? " active" : ""}`} onClick={() => setTab("files")}>Key Files</button>
        <button className={`panel-tab${tab === "context" ? " active" : ""}`} onClick={() => setTab("context")}>AI Context</button>
        <div style={{ marginLeft: "auto", padding: "8px 12px" }}>
          <button onClick={rescan} className="panel-btn panel-btn-secondary" style={{ background: "transparent", color: "var(--text-secondary)" }}>
            Rescan
          </button>
        </div>
      </div>

      {/* Tab content */}
      <div className="panel-body">
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
                  <code key={e} style={{ marginRight: "8px", fontSize: "var(--font-size-base)", color: "var(--link-color, #4ec9b0)" }}>{e}</code>
                ))}
              </div>
            )}

            {profile.env_vars.length > 0 && (
              <div style={{ marginBottom: "12px" }}>
                <strong>Environment variables:</strong>{" "}
                {profile.env_vars.map((v) => <code key={v} style={{ marginRight: "8px", fontSize: "var(--font-size-base)" }}>{v}</code>)}
              </div>
            )}

            <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", marginTop: "16px" }}>
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
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
                    <code style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{cmd.command}</code>
                    <button className="panel-btn panel-btn-primary" style={{ marginLeft: "8px" }} onClick={() => runCommand(cmd.command)}>Run</button>
                  </div>
                ))}
              </div>
            )}

            {profile.test_commands.length > 0 && (
              <div style={{ marginBottom: "16px" }}>
                <h4 style={{ margin: "0 0 8px 0" }}>Test</h4>
                {profile.test_commands.map((cmd, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
                    <code style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{cmd.command}</code>
                    <span style={{ ...badgeStyle, marginLeft: "8px" }}>{cmd.framework}</span>
                    <button className="panel-btn panel-btn-primary" style={{ marginLeft: "8px" }} onClick={() => runCommand(cmd.command)}>Run</button>
                  </div>
                ))}
              </div>
            )}

            {profile.lint_commands.length > 0 && (
              <div style={{ marginBottom: "16px" }}>
                <h4 style={{ margin: "0 0 8px 0" }}>Lint / Format</h4>
                {profile.lint_commands.map((cmd, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: "8px" }}>
                    <code style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{cmd.command}</code>
                    <button className="panel-btn panel-btn-primary" style={{ marginLeft: "8px" }} onClick={() => runCommand(cmd.command)}>Run</button>
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
                  borderRadius: "var(--radius-xs-plus)",
                  fontSize: "var(--font-size-base)",
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
                    <strong style={{ fontSize: "var(--font-size-md)" }}>{kf.path}</strong>
                    <span style={{ ...badgeStyle, marginLeft: "8px" }}>{kf.role}</span>
                  </div>
                  <pre style={{
                    background: "var(--bg-primary)",
                    padding: "8px",
                    borderRadius: "var(--radius-xs-plus)",
                    fontSize: "var(--font-size-sm)",
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
            <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", marginBottom: "12px" }}>
              This context is automatically injected into every AI agent conversation.
              It gives the AI deep understanding of your project without manual @-mentions.
            </p>
            <pre style={{
              background: "var(--bg-primary)",
              padding: "12px",
              borderRadius: "var(--radius-xs-plus)",
              fontSize: "var(--font-size-base)",
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
