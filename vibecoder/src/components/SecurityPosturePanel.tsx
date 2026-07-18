import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Security Posture panel — unified scanner aggregator UI.
//
// Design: docs/design/security-posture/panel.md.
//
// Surfaces every finding from every registered scanner (today:
// vulnerability_db + health_score; secret-leak / license / taint
// land in subsequent slices) as a single severity-ranked feed.
// Lets the user one-click promote a finding into the Goals system
// (once the bridge ships), suppress with a required reason, or
// view the audit log of past decisions.

type Severity = "critical" | "high" | "medium" | "low" | "info";

interface CategoryEnum {
  kind:
    | "prompt_injection"
    | "path_traversal"
    | "secret_leak"
    | "dependency_cve"
    | "sast"
    | "license_risk"
    | "code_health"
    | "other";
  label?: string;
}

interface FindingStatus {
  kind: "open" | "suppressed" | "goal_linked" | "fixed";
  reason?: string;
  goal_id?: string;
  at_unix_ms?: number;
}

interface SecurityFinding {
  id: string;
  severity: Severity;
  category: CategoryEnum;
  scanner: string;
  file: string;
  line?: number | null;
  column?: number | null;
  snippet?: string | null;
  rule_id: string;
  title: string;
  remediation?: string | null;
  references: string[];
  status: FindingStatus;
  first_seen_unix_ms: number;
  last_seen_unix_ms: number;
}

interface ScannerError {
  scanner: string;
  message: string;
}

interface AggregatorResult {
  findings: SecurityFinding[];
  errors: ScannerError[];
}

// DecisionLogEntry is used by the audit-log drawer which lands in a
// later slice. Exported so the drawer component can import it once
// it ships.
export interface DecisionLogEntry {
  at_unix_ms: number;
  finding_id: string;
  operation: "suppress" | "unsuppress" | "link_goal" | "unlink_goal" | "auto_resolved";
  reason?: string | null;
}

interface Props {
  workspace: string;
}

const SEVERITY_ORDER: Severity[] = ["critical", "high", "medium", "low", "info"];

const SEVERITY_COLOR: Record<Severity, string> = {
  critical: "#dc2626", // red-600
  high: "#ea580c",     // orange-600
  medium: "#ca8a04",   // yellow-600
  low: "#2563eb",      // blue-600
  info: "#737373",     // neutral-500
};

// Shape icons paired with colour (panel.md accessibility note —
// colour-blind users get the same severity signal).
const SEVERITY_GLYPH: Record<Severity, string> = {
  critical: "✗",
  high: "▲",
  medium: "●",
  low: "◆",
  info: "·",
};

function categoryLabel(c: CategoryEnum): string {
  if (c.kind === "other" && c.label) return c.label;
  return c.kind.replace(/_/g, " ");
}

function statusBadge(s: FindingStatus): string {
  switch (s.kind) {
    case "open":         return "Open";
    case "suppressed":   return "Suppressed";
    case "goal_linked":  return "Linked to goal";
    case "fixed":        return "Fixed";
  }
}

export function SecurityPosturePanel({ workspace }: Props) {
  const [findings, setFindings] = useState<SecurityFinding[]>([]);
  const [errors, setErrors] = useState<ScannerError[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Filter state. Defaults match panel.md: hide Low + Info, hide
  // suppressed, hide fixed.
  const [showSeverity, setShowSeverity] = useState<Set<Severity>>(
    new Set(["critical", "high", "medium"])
  );
  const [hideSuppressed, setHideSuppressed] = useState(true);
  const [hideFixed, setHideFixed] = useState(true);
  const [scannerFilter, setScannerFilter] = useState<string>("all");

  const loadCached = useCallback(async () => {
    if (!workspace) return;
    setLoadError(null);
    try {
      const cached = await invoke<SecurityFinding[]>("security_posture_findings", {
        workspacePath: workspace,
      });
      setFindings(cached);
    } catch (e) {
      setLoadError(String(e));
    }
  }, [workspace]);

  useEffect(() => {
    void loadCached();
  }, [loadCached]);

  const rescan = useCallback(async () => {
    if (!workspace) return;
    setScanning(true);
    setLoadError(null);
    try {
      const result = await invoke<AggregatorResult>("security_posture_scan", {
        workspacePath: workspace,
      });
      setFindings(result.findings);
      setErrors(result.errors);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setScanning(false);
    }
  }, [workspace]);

  const suppress = useCallback(
    async (id: string) => {
      const reason = window.prompt(
        "Suppression reason (recorded in audit log, required):"
      );
      if (!reason || !reason.trim()) return;
      try {
        await invoke("security_posture_suppress", {
          workspacePath: workspace,
          findingId: id,
          reason: reason.trim(),
        });
        await loadCached();
      } catch (e) {
        setLoadError(String(e));
      }
    },
    [workspace, loadCached]
  );

  const unsuppress = useCallback(
    async (id: string) => {
      try {
        await invoke("security_posture_unsuppress", {
          workspacePath: workspace,
          findingId: id,
        });
        await loadCached();
      } catch (e) {
        setLoadError(String(e));
      }
    },
    [workspace, loadCached]
  );

  const createGoal = useCallback(
    async (id: string) => {
      try {
        await invoke<string>("security_posture_create_goal", {
          workspacePath: workspace,
          findingId: id,
        });
        await loadCached();
      } catch (e) {
        // The bridge is a deliberate stub — surface the message
        // verbatim so the user sees the "coming in the next slice"
        // note. Don't treat it as a fatal panel error.
        setLoadError(String(e));
      }
    },
    [workspace, loadCached]
  );

  const allScanners = useMemo(() => {
    const set = new Set<string>();
    for (const f of findings) set.add(f.scanner);
    return ["all", ...Array.from(set).sort()];
  }, [findings]);

  const filtered = useMemo(() => {
    return findings.filter((f) => {
      if (!showSeverity.has(f.severity)) return false;
      if (scannerFilter !== "all" && f.scanner !== scannerFilter) return false;
      if (hideSuppressed && f.status.kind === "suppressed") return false;
      if (hideFixed && f.status.kind === "fixed") return false;
      return true;
    });
  }, [findings, showSeverity, scannerFilter, hideSuppressed, hideFixed]);

  // Group by severity for the section headers in the feed.
  const grouped = useMemo(() => {
    const buckets = new Map<Severity, SecurityFinding[]>();
    for (const sev of SEVERITY_ORDER) buckets.set(sev, []);
    for (const f of filtered) buckets.get(f.severity)?.push(f);
    return buckets;
  }, [filtered]);

  const selected = useMemo(
    () => findings.find((f) => f.id === selectedId) ?? null,
    [findings, selectedId]
  );

  const toggleSeverity = (sev: Severity) => {
    setShowSeverity((prev) => {
      const next = new Set(prev);
      if (next.has(sev)) {
        next.delete(sev);
      } else {
        next.add(sev);
      }
      return next;
    });
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        fontFamily: "var(--font-mono, monospace)",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "12px 16px",
          borderBottom: "1px solid var(--border-color, #2a2a2a)",
          display: "flex",
          alignItems: "center",
          gap: 12,
          flexWrap: "wrap",
        }}
      >
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Security Posture</h2>
        <button
          onClick={() => void rescan()}
          disabled={scanning || !workspace}
          style={{
            padding: "4px 10px",
            background: "var(--accent-color, #2563eb)",
            color: "white",
            border: "none",
            borderRadius: 4,
            cursor: scanning ? "wait" : "pointer",
          }}
        >
          {scanning ? "Scanning…" : "Rescan all"}
        </button>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
          {SEVERITY_ORDER.map((sev) => (
            <label key={sev} style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12 }}>
              <input
                type="checkbox"
                checked={showSeverity.has(sev)}
                onChange={() => toggleSeverity(sev)}
              />
              <span style={{ color: SEVERITY_COLOR[sev] }}>
                {SEVERITY_GLYPH[sev]} {sev}
              </span>
            </label>
          ))}
          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12 }}>
            <input
              type="checkbox"
              checked={hideSuppressed}
              onChange={(e) => setHideSuppressed(e.target.checked)}
            />
            hide suppressed
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12 }}>
            <input
              type="checkbox"
              checked={hideFixed}
              onChange={(e) => setHideFixed(e.target.checked)}
            />
            hide fixed
          </label>
          <select
            value={scannerFilter}
            onChange={(e) => setScannerFilter(e.target.value)}
            style={{ fontSize: 12, padding: "2px 4px" }}
          >
            {allScanners.map((s) => (
              <option key={s} value={s}>
                {s === "all" ? "all scanners" : s}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Per-scanner error banners */}
      {errors.length > 0 && (
        <div
          style={{
            padding: "8px 16px",
            background: "rgba(220,38,38,0.1)",
            borderBottom: "1px solid var(--border-color, #2a2a2a)",
            fontSize: 12,
          }}
        >
          {errors.map((e) => (
            <div key={e.scanner}>
              <strong>{e.scanner}:</strong> {e.message}
            </div>
          ))}
        </div>
      )}

      {/* Load error */}
      {loadError && (
        <div
          style={{
            padding: "8px 16px",
            background: "rgba(220,38,38,0.1)",
            borderBottom: "1px solid var(--border-color, #2a2a2a)",
            fontSize: 12,
          }}
        >
          {loadError}
          <button
            onClick={() => setLoadError(null)}
            style={{ marginLeft: 8, background: "none", border: "none", color: "inherit", cursor: "pointer" }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Two-pane body */}
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        {/* Feed */}
        <div
          style={{
            width: "38%",
            borderRight: "1px solid var(--border-color, #2a2a2a)",
            overflowY: "auto",
            fontSize: 13,
          }}
          role="listbox"
          aria-label="Security findings"
        >
          {SEVERITY_ORDER.map((sev) => {
            const rows = grouped.get(sev) ?? [];
            if (rows.length === 0) return null;
            return (
              <div key={sev}>
                <div
                  style={{
                    padding: "6px 12px",
                    background: "var(--bg-secondary, #1a1a1a)",
                    color: SEVERITY_COLOR[sev],
                    fontWeight: 600,
                    fontSize: 12,
                    position: "sticky",
                    top: 0,
                  }}
                >
                  {SEVERITY_GLYPH[sev]} {sev.toUpperCase()} ({rows.length})
                </div>
                {rows.map((f) => (
                  <div
                    key={f.id}
                    role="option"
                    aria-selected={selectedId === f.id}
                    onClick={() => setSelectedId(f.id)}
                    style={{
                      padding: "8px 12px",
                      cursor: "pointer",
                      borderBottom: "1px solid var(--border-color-dim, #1a1a1a)",
                      background:
                        selectedId === f.id
                          ? "var(--bg-selected, rgba(37,99,235,0.15))"
                          : "transparent",
                    }}
                  >
                    <div style={{ fontWeight: 500 }}>{f.title}</div>
                    <div style={{ fontSize: 11, color: "var(--text-dim, #888)", marginTop: 2 }}>
                      {f.file}
                      {f.line ? `:${f.line}` : ""} ·{" "}
                      <span style={{ color: "var(--text-muted, #aaa)" }}>{f.scanner}</span> ·{" "}
                      {categoryLabel(f.category)}
                      {f.status.kind !== "open" && (
                        <>
                          {" · "}
                          <span style={{ fontStyle: "italic" }}>{statusBadge(f.status)}</span>
                        </>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            );
          })}
          {filtered.length === 0 && (
            <div
              style={{
                padding: 24,
                textAlign: "center",
                color: "var(--text-dim, #888)",
              }}
            >
              {findings.length === 0
                ? "No findings yet. Click 'Rescan all' to run."
                : "No findings match current filters."}
            </div>
          )}
        </div>

        {/* Detail */}
        <div
          style={{
            flex: 1,
            padding: 20,
            overflowY: "auto",
            fontSize: 13,
          }}
        >
          {selected ? (
            <FindingDetail
              finding={selected}
              onSuppress={() => void suppress(selected.id)}
              onUnsuppress={() => void unsuppress(selected.id)}
              onCreateGoal={() => void createGoal(selected.id)}
            />
          ) : (
            <div style={{ color: "var(--text-dim, #888)", textAlign: "center", paddingTop: 40 }}>
              Select a finding from the feed to see details.
            </div>
          )}
        </div>
      </div>

      {/* Footer counts */}
      <div
        style={{
          padding: "6px 16px",
          borderTop: "1px solid var(--border-color, #2a2a2a)",
          fontSize: 11,
          color: "var(--text-dim, #888)",
          display: "flex",
          justifyContent: "space-between",
        }}
      >
        <span>
          {filtered.length} shown · {findings.length} total
        </span>
        <span>
          {SEVERITY_ORDER.map((sev) => {
            const n = findings.filter((f) => f.severity === sev).length;
            return (
              <span key={sev} style={{ marginLeft: 12, color: SEVERITY_COLOR[sev] }}>
                {SEVERITY_GLYPH[sev]} {n}
              </span>
            );
          })}
        </span>
      </div>
    </div>
  );
}

interface FindingDetailProps {
  finding: SecurityFinding;
  onSuppress: () => void;
  onUnsuppress: () => void;
  onCreateGoal: () => void;
}

function FindingDetail({ finding, onSuppress, onUnsuppress, onCreateGoal }: FindingDetailProps) {
  return (
    <div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 4,
        }}
      >
        <span
          style={{
            color: SEVERITY_COLOR[finding.severity],
            fontWeight: 700,
            fontSize: 14,
          }}
        >
          {SEVERITY_GLYPH[finding.severity]} {finding.severity.toUpperCase()}
        </span>
        <span
          style={{
            padding: "2px 6px",
            background: "var(--bg-tertiary, #222)",
            borderRadius: 3,
            fontSize: 11,
          }}
        >
          {finding.scanner}
        </span>
        <span
          style={{
            padding: "2px 6px",
            background: "var(--bg-tertiary, #222)",
            borderRadius: 3,
            fontSize: 11,
          }}
        >
          {categoryLabel(finding.category)}
        </span>
      </div>

      <h3 style={{ margin: "8px 0", fontSize: 15 }}>{finding.title}</h3>

      <div style={{ fontSize: 12, color: "var(--text-dim, #888)", marginBottom: 12 }}>
        <code>{finding.file}</code>
        {finding.line ? `:${finding.line}` : ""}
        {finding.column ? `:${finding.column}` : ""} · rule <code>{finding.rule_id}</code>
      </div>

      {finding.snippet && (
        <pre
          style={{
            background: "var(--bg-secondary, #1a1a1a)",
            padding: 10,
            borderRadius: 4,
            fontSize: 12,
            overflowX: "auto",
            margin: "8px 0 16px",
          }}
        >
          {finding.snippet}
        </pre>
      )}

      {finding.remediation && (
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>Remediation</div>
          <div style={{ whiteSpace: "pre-wrap" }}>{finding.remediation}</div>
        </div>
      )}

      {finding.references.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>References</div>
          {finding.references.map((url) => (
            <div key={url}>
              <a href={url} target="_blank" rel="noopener noreferrer" style={{ color: "#3b82f6" }}>
                {url}
              </a>
            </div>
          ))}
        </div>
      )}

      <div style={{ fontSize: 11, color: "var(--text-dim, #888)", marginBottom: 16 }}>
        Status: <strong>{statusBadge(finding.status)}</strong>
        {finding.status.kind === "suppressed" && finding.status.reason && (
          <> — reason: <em>{finding.status.reason}</em></>
        )}
        {finding.status.kind === "goal_linked" && finding.status.goal_id && (
          <> — goal id: <code>{finding.status.goal_id}</code></>
        )}
        <br />
        First seen: {new Date(finding.first_seen_unix_ms).toLocaleString()} · Last seen:{" "}
        {new Date(finding.last_seen_unix_ms).toLocaleString()}
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        {finding.status.kind !== "goal_linked" && (
          <button
            onClick={onCreateGoal}
            style={{
              padding: "6px 12px",
              background: "var(--accent-color, #2563eb)",
              color: "white",
              border: "none",
              borderRadius: 4,
              cursor: "pointer",
            }}
          >
            Create work item
          </button>
        )}
        {finding.status.kind === "suppressed" ? (
          <button
            onClick={onUnsuppress}
            style={{
              padding: "6px 12px",
              background: "transparent",
              color: "inherit",
              border: "1px solid var(--border-color, #2a2a2a)",
              borderRadius: 4,
              cursor: "pointer",
            }}
          >
            Lift suppression
          </button>
        ) : (
          <button
            onClick={onSuppress}
            style={{
              padding: "6px 12px",
              background: "transparent",
              color: "inherit",
              border: "1px solid var(--border-color, #2a2a2a)",
              borderRadius: 4,
              cursor: "pointer",
            }}
          >
            Suppress…
          </button>
        )}
      </div>
    </div>
  );
}

export default SecurityPosturePanel;
