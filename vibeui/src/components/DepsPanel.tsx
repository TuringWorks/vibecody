/**
 * DepsPanel — Dependency Manager.
 *
 * Auto-detects package manager (npm/yarn/pnpm/cargo/pip/go), scans for
 * outdated and vulnerable dependencies, displays structured table with
 * per-package upgrade actions.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EmptyState } from "./EmptyState";
import { StatusMessage } from "./StatusMessage";

interface DepInfo {
  name: string;
  current: string;
  latest: string;
  wanted: string;
  dep_type: string;
  is_outdated: boolean;
  is_vulnerable: boolean;
  vulnerability: string | null;
}

interface DepsResult {
  manager: string;
  deps: DepInfo[];
  total: number;
  outdated: number;
  vulnerable: number;
  raw_output: string;
}

interface DepsPanelProps {
  workspacePath: string | null;
}

type Filter = "all" | "outdated" | "vulnerable";

const managerLabel: Record<string, string> = {
  npm: "npm", yarn: "Yarn", pnpm: "pnpm", cargo: "Cargo", pip: "pip", go: "Go Modules",
};

export function DepsPanel({ workspacePath }: DepsPanelProps) {
  const [manager, setManager] = useState<string | null>(null);
  const [result, setResult] = useState<DepsResult | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [upgrading, setUpgrading] = useState<Set<string>>(new Set());
  const [showRaw, setShowRaw] = useState(false);

  useEffect(() => {
    if (!workspacePath) return;
    invoke<string>("detect_package_manager", { workspace: workspacePath })
      .then(setManager)
      .catch(() => setManager(null));
  }, [workspacePath]);

  if (!workspacePath) {
    return (
      <EmptyState
        icon="📦"
        title="No workspace open"
        description="Open a workspace folder to manage dependencies."
      />
    );
  }

  const handleScan = async () => {
    if (!manager) return;
    setScanning(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<DepsResult>("scan_dependencies", {
        workspace: workspacePath, manager,
      });
      setResult(r);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  };

  const handleUpgrade = async (pkg: string) => {
    if (!manager) return;
    setUpgrading((prev) => new Set(prev).add(pkg));
    try {
      await invoke<string>("upgrade_dependency", {
        workspace: workspacePath, manager, package: pkg, version: null,
      });
      // Re-scan after upgrade
      handleScan();
    } catch (e: unknown) {
      setError(`Failed to upgrade ${pkg}: ${e}`);
    } finally {
      setUpgrading((prev) => {
        const next = new Set(prev);
        next.delete(pkg);
        return next;
      });
    }
  };

  const filtered = result
    ? result.deps.filter((d) => {
        if (filter === "outdated") return d.is_outdated;
        if (filter === "vulnerable") return d.is_vulnerable;
        return true;
      })
    : [];

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12, height: "100%", overflowY: "auto" }}>
      {/* Manager badge + Scan button */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <div style={{
          background: "var(--bg-secondary)", borderRadius: 6, padding: "6px 12px",
          border: "1px solid var(--border-color)", fontSize: 12, fontWeight: 600,
        }}>
          {manager ? managerLabel[manager] || manager : "No package manager detected"}
        </div>
        <button
          onClick={handleScan}
          disabled={scanning || !manager}
          style={{
            padding: "6px 16px", fontSize: 12, fontWeight: 600,
            background: scanning ? "var(--bg-tertiary)" : "var(--accent-color, #007acc)",
            color: "var(--text-primary, #e0e0e0)", border: "none", borderRadius: 6,
            cursor: scanning || !manager ? "not-allowed" : "pointer",
          }}
        >
          {scanning ? "Scanning..." : "Scan Dependencies"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <StatusMessage variant="error" message={error} inline />
      )}

      {/* Results */}
      {result && (
        <>
          {/* Summary bar */}
          <div style={{
            display: "flex", gap: 12, fontSize: 12,
            background: "var(--bg-secondary)", borderRadius: 6, padding: 10,
            border: "1px solid var(--border-color)",
          }}>
            <span>Total: <strong>{result.total}</strong></span>
            <span style={{ color: result.outdated > 0 ? "var(--warning-color, #ff9800)" : "inherit" }}>
              Outdated: <strong>{result.outdated}</strong>
            </span>
            <span style={{ color: result.vulnerable > 0 ? "var(--error-color, #f44336)" : "inherit" }}>
              Vulnerable: <strong>{result.vulnerable}</strong>
            </span>
          </div>

          {/* Filter tabs */}
          <div style={{ display: "flex", gap: 6 }}>
            {(["all", "outdated", "vulnerable"] as Filter[]).map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                style={{
                  padding: "4px 12px", fontSize: 11, borderRadius: 12,
                  background: filter === f ? "var(--accent-blue, #6366f1)" : "var(--bg-secondary)",
                  border: `1px solid ${filter === f ? "var(--accent-blue, #6366f1)" : "var(--border-color)"}`,
                  color: "var(--text-primary)", cursor: "pointer",
                  fontWeight: filter === f ? 600 : 400,
                }}
              >
                {f.charAt(0).toUpperCase() + f.slice(1)}
                {f === "outdated" && result.outdated > 0 ? ` (${result.outdated})` : ""}
                {f === "vulnerable" && result.vulnerable > 0 ? ` (${result.vulnerable})` : ""}
              </button>
            ))}
          </div>

          {/* Dependency table */}
          {filtered.length > 0 ? (
            <div style={{ flex: 1, overflowY: "auto" }}>
              {/* Header */}
              <div style={{
                display: "grid", gridTemplateColumns: "1fr 80px 80px 80px 90px 60px",
                gap: 4, padding: "6px 8px", fontSize: 11, fontWeight: 600,
                borderBottom: "1px solid var(--border-color)", opacity: 0.7,
              }}>
                <span>Package</span>
                <span style={{ textAlign: "right" }}>Current</span>
                <span style={{ textAlign: "right" }}>Wanted</span>
                <span style={{ textAlign: "right" }}>Latest</span>
                <span style={{ textAlign: "center" }}>Status</span>
                <span></span>
              </div>
              {/* Rows */}
              {filtered.map((dep) => (
                <div
                  key={dep.name}
                  style={{
                    display: "grid", gridTemplateColumns: "1fr 80px 80px 80px 90px 60px",
                    gap: 4, padding: "5px 8px", fontSize: 11,
                    borderBottom: "1px solid var(--border-color)",
                    background: dep.is_vulnerable ? "rgba(243,139,168,0.05)" : "transparent",
                  }}
                >
                  <span style={{ fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={dep.name}>
                    {dep.name}
                  </span>
                  <span style={{ textAlign: "right", fontFamily: "monospace", opacity: 0.7 }}>{dep.current}</span>
                  <span style={{ textAlign: "right", fontFamily: "monospace", opacity: 0.7 }}>{dep.wanted}</span>
                  <span style={{ textAlign: "right", fontFamily: "monospace", color: dep.is_outdated ? "var(--warning-color, #ff9800)" : "inherit" }}>
                    {dep.latest}
                  </span>
                  <span style={{ textAlign: "center", fontSize: 10 }}>
                    {dep.is_vulnerable && (
                      <span title={dep.vulnerability || "Vulnerable"} style={{ color: "var(--text-danger, #f38ba8)", marginRight: 4 }}>
                        &#9888;
                      </span>
                    )}
                    {dep.is_outdated && <span style={{ color: "var(--text-warning-alt, #fab387)" }}>&#11014; outdated</span>}
                  </span>
                  <span style={{ textAlign: "center" }}>
                    {dep.is_outdated && (
                      <button
                        onClick={() => handleUpgrade(dep.name)}
                        disabled={upgrading.has(dep.name)}
                        title={`Upgrade to ${dep.latest}`}
                        style={{
                          background: "none", border: "1px solid var(--border-color)",
                          borderRadius: 4, padding: "1px 6px", fontSize: 10,
                          color: "var(--text-info, #89b4fa)", cursor: "pointer",
                        }}
                      >
                        {upgrading.has(dep.name) ? "..." : "Up"}
                      </button>
                    )}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <StatusMessage
              variant="empty"
              message={filter === "all" ? "No dependencies found." : `No ${filter} dependencies.`}
            />
          )}

          {/* Raw output toggle */}
          <div>
            <button
              onClick={() => setShowRaw(!showRaw)}
              style={{ background: "none", border: "none", cursor: "pointer", fontSize: 11, color: "var(--text-info, #89b4fa)", padding: 0, textDecoration: "underline" }}
            >
              {showRaw ? "Hide raw output" : "Show raw output"}
            </button>
            {showRaw && (
              <pre style={{
                marginTop: 8, background: "var(--bg-secondary)", borderRadius: 6,
                padding: 10, fontSize: 10, fontFamily: "monospace", maxHeight: 200,
                overflowY: "auto", whiteSpace: "pre-wrap", color: "var(--text-primary)",
                border: "1px solid var(--border-color)",
              }}>
                {result.raw_output || "(no output)"}
              </pre>
            )}
          </div>
        </>
      )}

      {/* Empty state */}
      {!result && !scanning && !error && (
        <EmptyState
          title={manager ? "Ready to scan" : "No package manager detected"}
          description={manager ? "Click Scan Dependencies to check for outdated and vulnerable packages." : "No package manager detected for this workspace."}
        />
      )}
    </div>
  );
}
