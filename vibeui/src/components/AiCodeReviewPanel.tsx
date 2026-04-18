/**
 * AiCodeReviewPanel — SonarQube-style AI code review.
 *
 * Scans files line-by-line against embedded SonarQube-compatible rules,
 * showing exact line numbers, code snippets, rule explanations, and fix guidance.
 */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types (mirror sonar_rules.rs) ────────────────────────────────────────────

interface SonarIssue {
  rule_key: string;
  rule_name: string;
  file: string;
  line: number;
  end_line: number;
  col_start: number;
  message: string;
  severity: string;
  issue_type: string;
  code_snippet: string;
  context_before: string;
  context_after: string;
  why: string;
  how_to_fix: string;
  effort: string;
}

interface SonarScanResult {
  file: string;
  issues: SonarIssue[];
  bugs: number;
  vulnerabilities: number;
  code_smells: number;
  security_hotspots: number;
  debt_minutes: number;
}

interface SonarRule {
  key: string;
  name: string;
  description: string;
  why: string;
  how_to_fix: string;
  severity: string;
  issue_type: string;
  language: string;
  tags: string[];
  effort_minutes: number;
}

// ── Severity helpers ──────────────────────────────────────────────────────────

const SEVERITY_CONFIG: Record<string, { color: string; bg: string; label: string }> = {
  BLOCKER:  { color: "var(--error-color)",   bg: "color-mix(in srgb, var(--error-color) 12%, transparent)",   label: "Blocker"  },
  CRITICAL: { color: "#ff5722",              bg: "color-mix(in srgb, #ff5722 12%, transparent)",              label: "Critical" },
  MAJOR:    { color: "var(--warning-color)", bg: "color-mix(in srgb, var(--warning-color) 12%, transparent)", label: "Major"    },
  MINOR:    { color: "var(--info-color)",    bg: "color-mix(in srgb, var(--info-color) 12%, transparent)",    label: "Minor"    },
  INFO:     { color: "var(--text-secondary)", bg: "transparent",                                               label: "Info"     },
};

const TYPE_ICON: Record<string, string> = {
  BUG:              "B",
  VULNERABILITY:    "V",
  CODE_SMELL:       "S",
  SECURITY_HOTSPOT: "H",
};

const TYPE_COLOR: Record<string, string> = {
  BUG:              "var(--error-color)",
  VULNERABILITY:    "#ff5722",
  CODE_SMELL:       "var(--warning-color)",
  SECURITY_HOTSPOT: "#e91e63",
};

function SeverityBadge({ severity }: { severity: string }) {
  const cfg = SEVERITY_CONFIG[severity] ?? SEVERITY_CONFIG.INFO;
  return (
    <span style={{
      display: "inline-block",
      padding: "1px 8px",
      borderRadius: "var(--radius-xs-plus)",
      fontSize: "var(--font-size-xs)",
      fontWeight: 600,
      background: cfg.bg,
      color: cfg.color,
      border: `1px solid ${cfg.color}`,
      letterSpacing: "0.03em",
    }}>
      {cfg.label}
    </span>
  );
}

function TypeBadge({ type_ }: { type_: string }) {
  return (
    <span style={{
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      width: 18,
      height: 18,
      borderRadius: 3,
      fontSize: 10,
      fontWeight: 700,
      background: TYPE_COLOR[type_] ?? "var(--text-secondary)",
      color: "var(--btn-primary-fg)",
      flexShrink: 0,
    }}>
      {TYPE_ICON[type_] ?? "?"}
    </span>
  );
}

// ── Code Snippet with line highlight ─────────────────────────────────────────

function CodeBlock({
  before, snippet, after, line, col,
}: {
  before: string; snippet: string; after: string; line: number; col: number;
}) {
  const lines = [
    before && { n: line - 1, code: before, highlight: false },
    { n: line, code: snippet, highlight: true },
    after && { n: line + 1, code: after, highlight: false },
  ].filter(Boolean) as { n: number; code: string; highlight: boolean }[];

  return (
    <div style={{
      borderRadius: "var(--radius-xs-plus)",
      overflow: "hidden",
      border: "1px solid var(--border-color)",
      fontFamily: "monospace",
      fontSize: "var(--font-size-sm)",
      marginTop: 8,
    }}>
      {lines.map(({ n, code, highlight }) => (
        <div key={n} style={{
          display: "flex",
          background: highlight
            ? "color-mix(in srgb, var(--warning-color) 10%, var(--bg-secondary))"
            : "var(--bg-secondary)",
          borderLeft: highlight ? "3px solid var(--warning-color)" : "3px solid transparent",
        }}>
          <span style={{
            minWidth: 36,
            textAlign: "right",
            padding: "3px 8px 3px 0",
            color: "var(--text-muted, var(--text-secondary))",
            userSelect: "none",
            fontSize: "var(--font-size-xs)",
          }}>
            {n}
          </span>
          <span style={{ padding: "3px 8px", whiteSpace: "pre", overflowX: "auto", flexGrow: 1 }}>
            {highlight && col > 0 ? (
              <>
                {code.slice(0, col)}
                <span style={{ background: "color-mix(in srgb, var(--warning-color) 35%, transparent)", borderRadius: 2 }}>
                  {code.slice(col)}
                </span>
              </>
            ) : code}
          </span>
        </div>
      ))}
    </div>
  );
}

// ── Issue Card ────────────────────────────────────────────────────────────────

function IssueCard({ issue, expanded, onToggle }: {
  issue: SonarIssue;
  expanded: boolean;
  onToggle: () => void;
}) {
  const sev = SEVERITY_CONFIG[issue.severity] ?? SEVERITY_CONFIG.INFO;
  const shortFile = issue.file.split(/[/\\]/).slice(-2).join("/");

  return (
    <div role="button" tabIndex={0}
      className="panel-card"
      style={{
        borderLeft: `3px solid ${sev.color}`,
        cursor: "pointer",
        marginBottom: 6,
        padding: 0,
      }}
      onClick={onToggle}
    >
      {/* Header row */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 12px" }}>
        <TypeBadge type_={issue.issue_type} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
            <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
              {issue.message}
            </span>
          </div>
          <div style={{ display: "flex", gap: 8, marginTop: 3, alignItems: "center", flexWrap: "wrap" }}>
            <span style={{ fontSize: "var(--font-size-xs)", color: sev.color, fontFamily: "monospace", fontWeight: 600 }}>
              {shortFile}:{issue.line}
            </span>
            <SeverityBadge severity={issue.severity} />
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "monospace" }}>
              {issue.rule_key}
            </span>
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
              {issue.effort}
            </span>
          </div>
        </div>
        <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", flexShrink: 0 }}>
          {expanded ? "▲" : "▼"}
        </span>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div role="button" tabIndex={0}
          style={{ borderTop: "1px solid var(--border-color)", padding: "12px 12px" }}
          onClick={e => e.stopPropagation()}
        >
          {/* Code block */}
          <CodeBlock
            before={issue.context_before}
            snippet={issue.code_snippet}
            after={issue.context_after}
            line={issue.line}
            col={issue.col_start}
          />

          {/* Rule name */}
          <div style={{ marginTop: 10, fontWeight: 600, fontSize: "var(--font-size-sm)", color: "var(--text-primary)" }}>
            {issue.rule_name}
          </div>

          {/* Why section */}
          <div style={{ marginTop: 8 }}>
            <div style={{
              fontSize: "var(--font-size-xs)", fontWeight: 700, letterSpacing: "0.06em",
              color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 3,
            }}>
              Why this is a problem
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-primary)", lineHeight: 1.5 }}>
              {issue.why}
            </div>
          </div>

          {/* How to fix section */}
          <div style={{ marginTop: 8 }}>
            <div style={{
              fontSize: "var(--font-size-xs)", fontWeight: 700, letterSpacing: "0.06em",
              color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 3,
            }}>
              How to fix
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-primary)", lineHeight: 1.5, fontFamily: "monospace" }}>
              {issue.how_to_fix}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Summary bar ───────────────────────────────────────────────────────────────

function SummaryBar({ result }: { result: SonarScanResult }) {
  const total = result.bugs + result.vulnerabilities + result.code_smells + result.security_hotspots;
  const debt = result.debt_minutes >= 60
    ? `${Math.floor(result.debt_minutes / 60)}h ${result.debt_minutes % 60}min`
    : `${result.debt_minutes}min`;

  const items = [
    { label: "Bugs",          value: result.bugs,              color: TYPE_COLOR.BUG,              icon: "B" },
    { label: "Vulnerabilities", value: result.vulnerabilities, color: TYPE_COLOR.VULNERABILITY,    icon: "V" },
    { label: "Code Smells",   value: result.code_smells,       color: TYPE_COLOR.CODE_SMELL,       icon: "S" },
    { label: "Hotspots",      value: result.security_hotspots, color: TYPE_COLOR.SECURITY_HOTSPOT, icon: "H" },
  ];

  return (
    <div style={{
      display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12,
      padding: "12px 12px",
      background: "var(--bg-secondary)",
      borderRadius: "var(--radius-xs-plus)",
      border: "1px solid var(--border-color)",
    }}>
      {items.map(({ label, value, color, icon }) => (
        <div key={label} style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 110 }}>
          <span style={{
            display: "inline-flex", alignItems: "center", justifyContent: "center",
            width: 20, height: 20, borderRadius: 3,
            background: color, color: "var(--btn-primary-fg)", fontSize: 10, fontWeight: 700,
          }}>{icon}</span>
          <span style={{ fontWeight: 700, fontSize: "var(--font-size-xl)", color: value > 0 ? color : "var(--text-secondary)" }}>
            {value}
          </span>
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{label}</span>
        </div>
      ))}
      <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4 }}>
        <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>Total: </span>
        <span style={{ fontWeight: 700 }}>{total}</span>
        <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginLeft: 8 }}>Debt: </span>
        <span style={{ fontWeight: 600, color: "var(--warning-color)" }}>{debt}</span>
      </div>
    </div>
  );
}

// ── Main Panel ────────────────────────────────────────────────────────────────

type ActiveTab = "scan" | "rules";
type FilterType = "ALL" | "BUG" | "VULNERABILITY" | "CODE_SMELL" | "SECURITY_HOTSPOT";
type FilterSev = "ALL" | "BLOCKER" | "CRITICAL" | "MAJOR" | "MINOR" | "INFO";

export default function AiCodeReviewPanel() {
  const [tab, setTab] = useState<ActiveTab>("scan");
  const [filePath, setFilePath] = useState("");
  const [content, setContent] = useState("");
  const [result, setResult] = useState<SonarScanResult | null>(null);
  const [rules, setRules] = useState<SonarRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadMsg, setLoadMsg] = useState("");
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  const [filterType, setFilterType] = useState<FilterType>("ALL");
  const [filterSev, setFilterSev] = useState<FilterSev>("ALL");
  const [ruleSearch, setRuleSearch] = useState("");

  // Auto-load rules on mount
  useEffect(() => {
    invoke<SonarRule[]>("sonar_get_rules", {}).then(setRules).catch(() => {});
  }, []);

  const handleScan = useCallback(async () => {
    if (!content.trim()) return;
    setLoading(true);
    setResult(null);
    try {
      const res = await invoke<SonarScanResult>("sonar_scan_file", {
        filePath: filePath || "untitled",
        content,
      });
      setResult(res);
      setExpandedIds(new Set());
    } catch (e) {
      console.error(e);
    }
    setLoading(false);
  }, [filePath, content]);

  const handleLoadRules = useCallback(async () => {
    setLoadMsg("Loading…");
    try {
      const count = await invoke<number>("sonar_load_rules");
      const refreshed = await invoke<SonarRule[]>("sonar_get_rules", {});
      setRules(refreshed);
      setLoadMsg(`${count} rules loaded into local database`);
    } catch (e) {
      setLoadMsg(`Error: ${e}`);
    }
  }, []);

  const toggleExpanded = useCallback((key: string) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  }, []);

  const filteredIssues = result?.issues.filter(i =>
    (filterType === "ALL" || i.issue_type === filterType) &&
    (filterSev === "ALL" || i.severity === filterSev)
  ) ?? [];

  const filteredRules = rules.filter(r =>
    ruleSearch === "" ||
    r.key.toLowerCase().includes(ruleSearch.toLowerCase()) ||
    r.name.toLowerCase().includes(ruleSearch.toLowerCase()) ||
    r.language.toLowerCase().includes(ruleSearch.toLowerCase())
  );

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div style={{ display: "flex", gap: 4, marginBottom: 14 }}>
        {(["scan", "rules"] as ActiveTab[]).map(t => (
          <button key={t} className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => setTab(t)}>
            {t === "scan" ? "Code Scan" : "Rule Library"}
          </button>
        ))}
        {rules.length > 0 && (
          <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", alignSelf: "center" }}>
            {rules.length} rules loaded
          </span>
        )}
      </div>

      {/* ── SCAN TAB ────────────────────────────────────────────────── */}
      {tab === "scan" && (
        <>
          {/* Input */}
          <div className="panel-card" style={{ marginBottom: 10 }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <input
                className="panel-input"
                style={{ flex: 1 }}
                placeholder="File path (e.g. src/auth.ts)"
                value={filePath}
                onChange={e => setFilePath(e.target.value)}
              />
            </div>
            <textarea
              className="panel-input panel-input-full"
              rows={10}
              style={{ fontFamily: "monospace", resize: "vertical", marginBottom: 8 }}
              placeholder="Paste file content here…"
              value={content}
              onChange={e => setContent(e.target.value)}
            />
            <button
              className="panel-btn panel-btn-primary"
              onClick={handleScan}
              disabled={loading || !content.trim()}
            >
              {loading ? "Scanning…" : "Scan"}
            </button>
          </div>

          {/* Results */}
          {result && (
            <>
              {/* Fixed summary + filters */}
              <SummaryBar result={result} />

              {result.issues.length > 0 && (
                <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap", flexShrink: 0 }}>
                  {(["ALL", "BUG", "VULNERABILITY", "CODE_SMELL", "SECURITY_HOTSPOT"] as FilterType[]).map(t => (
                    <button
                      key={t}
                      className={`panel-btn panel-btn-sm ${filterType === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
                      onClick={() => setFilterType(t)}
                    >
                      {t === "ALL" ? "All Types" : t === "CODE_SMELL" ? "Code Smells" : t === "SECURITY_HOTSPOT" ? "Hotspots" : t.charAt(0) + t.slice(1).toLowerCase()}
                    </button>
                  ))}
                  <span style={{ width: 1, background: "var(--border-color)", margin: "0 4px" }} />
                  {(["ALL", "BLOCKER", "CRITICAL", "MAJOR", "MINOR", "INFO"] as FilterSev[]).map(s => (
                    <button
                      key={s}
                      className={`panel-btn panel-btn-sm ${filterSev === s ? "panel-btn-primary" : "panel-btn-secondary"}`}
                      onClick={() => setFilterSev(s)}
                    >
                      {s === "ALL" ? "All Sev" : s.charAt(0) + s.slice(1).toLowerCase()}
                    </button>
                  ))}
                </div>
              )}

              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6, flexShrink: 0 }}>
                {filteredIssues.length} issue{filteredIssues.length !== 1 ? "s" : ""}
                {(filterType !== "ALL" || filterSev !== "ALL") && ` (filtered from ${result.issues.length})`}
              </div>

              {/* Scrollable issue list */}
              <div style={{ flex: 1, overflowY: "auto", minHeight: 0 }}>
                {filteredIssues.length === 0 && (
                  <div className="panel-card" style={{ color: "var(--success-color)", textAlign: "center" }}>
                    No issues found with current filters.
                  </div>
                )}

                {filteredIssues.map((issue, idx) => {
                  const uid = `${issue.rule_key}-${issue.line}-${idx}`;
                  return (
                    <IssueCard
                      key={uid}
                      issue={issue}
                      expanded={expandedIds.has(uid)}
                      onToggle={() => toggleExpanded(uid)}
                    />
                  );
                })}
              </div>
            </>
          )}
        </>
      )}

      {/* ── RULES TAB ───────────────────────────────────────────────── */}
      {tab === "rules" && (
        <>
          {/* Sticky toolbar — stays visible while the list scrolls */}
          <div style={{ display: "flex", gap: 8, marginBottom: 8, flexShrink: 0 }}>
            <input
              className="panel-input"
              style={{ flex: 1 }}
              placeholder="Search rules by key, name, or language…"
              value={ruleSearch}
              onChange={e => setRuleSearch(e.target.value)}
            />
            <button className="panel-btn panel-btn-secondary" onClick={handleLoadRules}>
              Import to DB
            </button>
          </div>
          {loadMsg && (
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--success-color)", marginBottom: 6, flexShrink: 0 }}>
              {loadMsg}
            </div>
          )}

          {/* Scrollable rules list */}
          <div style={{ flex: 1, overflowY: "auto", minHeight: 0 }}>
            {filteredRules.length === 0 && (
              <div className="panel-card" style={{ color: "var(--text-secondary)" }}>No rules match.</div>
            )}

            {filteredRules.map(rule => (
              <div key={rule.key} className="panel-card" style={{ marginBottom: 6 }}>
                <div style={{ display: "flex", alignItems: "flex-start", gap: 8, marginBottom: 4 }}>
                  <TypeBadge type_={rule.issue_type} />
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
                      <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{rule.name}</span>
                      <SeverityBadge severity={rule.severity} />
                    </div>
                    <div style={{ display: "flex", gap: 8, marginTop: 2 }}>
                      <span style={{ fontSize: "var(--font-size-xs)", fontFamily: "monospace", color: "var(--text-secondary)" }}>
                        {rule.key}
                      </span>
                      <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                        {rule.language}
                      </span>
                      <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                        {rule.effort_minutes}min
                      </span>
                    </div>
                  </div>
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6 }}>
                  {rule.description}
                </div>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  {rule.tags.map(tag => (
                    <span key={tag} style={{
                      fontSize: "var(--font-size-xs)", padding: "1px 8px",
                      borderRadius: "var(--radius-xs-plus)",
                      background: "var(--bg-tertiary, var(--bg-secondary))",
                      color: "var(--text-secondary)",
                      border: "1px solid var(--border-color)",
                    }}>
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
