import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Lightbulb, ChevronDown } from 'lucide-react';

// ── Types (mirrors review.rs) ──────────────────────────────────────────────

type Severity = 'info' | 'warning' | 'critical';
type ReviewFocus = 'security' | 'performance' | 'correctness' | 'style' | 'testing';

interface ReviewIssue {
 file: string;
 line: number;
 severity: Severity;
 category: ReviewFocus;
 description: string;
 suggested_fix?: string;
}

interface ReviewSuggestion {
 description: string;
 file?: string;
}

interface ReviewScore {
 overall: number;
 correctness: number;
 security: number;
 performance: number;
 style: number;
}

interface ReviewReport {
 base_ref: string;
 target_ref: string;
 summary: string;
 issues: ReviewIssue[];
 suggestions: ReviewSuggestion[];
 score: ReviewScore;
 files_reviewed: string[];
}

// ── Props ──────────────────────────────────────────────────────────────────

interface ReviewPanelProps {
 workspacePath: string | null;
 onOpenFile?: (path: string, line?: number) => void;
}

// ── Helpers ────────────────────────────────────────────────────────────────

const SEVERITY_STYLES: Record<Severity, { badge: string; border: string }> = {
 critical: { badge: 'bg-red-600 text-white', border: 'border-l-red-500' },
 warning: { badge: 'bg-yellow-500 text-black', border: 'border-l-yellow-400' },
 info: { badge: 'bg-blue-500 text-white', border: 'border-l-blue-400' },
};

const FOCUS_EMOJI: Record<ReviewFocus, string> = {
 security: '',
 performance: '',
 correctness: '',
 style: '',
 testing: '',
};

function ScoreBar({ label, value }: { label: string; value: number }) {
 const pct = Math.round((value / 10) * 100);
 const color = value >= 8 ? 'var(--text-success)' : value >= 5 ? 'var(--text-warning)' : 'var(--text-danger)';
 return (
 <div style={{ marginBottom: 6 }}>
 <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: "var(--font-size-base)", marginBottom: 2 }}>
 <span>{label}</span>
 <span style={{ color }}>{value.toFixed(1)}</span>
 </div>
 <div style={{ background: 'var(--border-color)', borderRadius: "var(--radius-xs-plus)", height: 6, overflow: 'hidden' }}>
 <div style={{ width: `${pct}%`, height: '100%', background: color, transition: 'width 0.3s' }} />
 </div>
 </div>
 );
}

// ── Component ──────────────────────────────────────────────────────────────

export function ReviewPanel({ workspacePath, onOpenFile }: ReviewPanelProps) {
 const [isLoading, setIsLoading] = useState(false);
 const [report, setReport] = useState<ReviewReport | null>(null);
 const [error, setError] = useState<string | null>(null);
 const [filterSeverity, setFilterSev] = useState<Severity | 'all'>('all');
 const [baseRef, setBaseRef] = useState('');
 const [expandedIssue, setExpanded] = useState<number | null>(null);

 const runReview = async () => {
 if (!workspacePath) return;
 setIsLoading(true);
 setError(null);
 setReport(null);
 try {
 const result = await invoke<ReviewReport>('run_code_review', {
 workspacePath,
 baseRef: baseRef.trim() || null,
 targetRef: null,
 });
 setReport(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setIsLoading(false);
 }
 };

 const filteredIssues = report?.issues.filter(
 (i) => filterSeverity === 'all' || i.severity === filterSeverity,
 ) ?? [];

 const countBySev = (sev: Severity) =>
 report?.issues.filter((i) => i.severity === sev).length ?? 0;

 return (
 <div className="panel-container">
 {/* ── Toolbar ── */}
 <div className="panel-header">
 <input
 value={baseRef}
 onChange={(e) => setBaseRef(e.target.value)}
 placeholder="Base ref (e.g. main, HEAD~1) — leave blank for uncommitted"
 className="panel-input"
 style={{ flex: 1, minWidth: 180 }}
 />
 <button
 onClick={runReview}
 disabled={isLoading || !workspacePath}
 className="panel-btn panel-btn-primary"
 style={{ opacity: !workspacePath ? 0.5 : 1 }}
 >
 {isLoading ? ' Reviewing…' : ' Run Review'}
 </button>
 </div>

 <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
 {/* ── Error ── */}
 {error && (
 <div className="panel-error">{error}</div>
 )}

 {/* ── Loading placeholder ── */}
 {isLoading && (
 <div style={{ textAlign: 'center', color: 'var(--text-secondary)', paddingTop: 32, fontSize: "var(--font-size-md)" }}>
 Analyzing diff…<br />
 <span style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>This may take 15–30 seconds depending on diff size</span>
 </div>
 )}

 {/* ── Report ── */}
 {report && !isLoading && (
 <div style={{ flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 10 }}>

 {/* Summary card */}
 <div style={{ background: 'var(--bg-tertiary)', borderRadius: "var(--radius-sm)", padding: 12 }}>
 <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 6, color: 'var(--text-primary)' }}>
 Review Summary {report.base_ref && (
 <span style={{ fontWeight: 400, color: 'var(--text-secondary)', fontSize: "var(--font-size-sm)" }}>
 ({report.base_ref || 'working tree'} → {report.target_ref || 'HEAD'})
 </span>
 )}
 </div>
 <p style={{ fontSize: "var(--font-size-base)", color: 'var(--text-secondary)', margin: 0, lineHeight: 1.5 }}>{report.summary}</p>
 {report.files_reviewed.length > 0 && (
 <div style={{ marginTop: 8, fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>
 {report.files_reviewed.length} file{report.files_reviewed.length !== 1 ? 's' : ''} reviewed
 </div>
 )}
 </div>

 {/* Score bars */}
 <div style={{ background: 'var(--bg-tertiary)', borderRadius: "var(--radius-sm)", padding: 12 }}>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8, color: 'var(--text-primary)' }}>
 Quality Score — Overall: {report.score.overall.toFixed(1)} / 10
 </div>
 <ScoreBar label="Correctness" value={report.score.correctness} />
 <ScoreBar label="Security" value={report.score.security} />
 <ScoreBar label="Performance" value={report.score.performance} />
 <ScoreBar label="Style" value={report.score.style} />
 </div>

 {/* Severity filter tabs */}
 {report.issues.length > 0 && (
 <div style={{ display: 'flex', gap: 6 }}>
 {(['all', 'critical', 'warning', 'info'] as const).map((sev) => {
 const count = sev === 'all' ? report.issues.length : countBySev(sev);
 const active = filterSeverity === sev;
 return (
 <button
 key={sev}
 onClick={() => setFilterSev(sev)}
 style={{
 padding: '3px 10px', borderRadius: 12, border: '1px solid var(--border-color)',
 background: active ? 'var(--border-color)' : 'transparent', color: active ? 'var(--text-primary)' : 'var(--text-secondary)',
 fontSize: "var(--font-size-sm)", cursor: 'pointer',
 }}
 >
 {sev === 'all' ? 'All' : sev.charAt(0).toUpperCase() + sev.slice(1)} ({count})
 </button>
 );
 })}
 </div>
 )}

 {/* Issues list */}
 {filteredIssues.length === 0 && report.issues.length > 0 ? (
 <div style={{ color: 'var(--text-secondary)', fontSize: "var(--font-size-base)", textAlign: 'center', paddingTop: 12 }}>
 No {filterSeverity} issues found.
 </div>
 ) : filteredIssues.length === 0 ? (
 <div style={{ color: 'var(--text-success)', fontSize: "var(--font-size-md)", textAlign: 'center', paddingTop: 12 }}>
 No issues found — looks good!
 </div>
 ) : (
 <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
 {filteredIssues.map((issue, idx) => {
 const sty = SEVERITY_STYLES[issue.severity];
 const isOpen = expandedIssue === idx;
 return (
 <div
 key={idx}
 style={{
 background: 'var(--bg-tertiary)', borderRadius: "var(--radius-sm)", borderLeft: `3px solid`,
 borderLeftColor: sty.border.replace('border-l-', '').includes('red') ? 'var(--text-danger)'
 : sty.border.includes('yellow') ? 'var(--text-warning)' : 'var(--text-info)',
 overflow: 'hidden',
 }}
 >
 <div
 style={{ padding: '8px 10px', cursor: 'pointer', display: 'flex', gap: 8, alignItems: 'flex-start' }}
 onClick={() => setExpanded(isOpen ? null : idx)}
 >
 <span style={{ fontSize: "var(--font-size-sm)" }}>{FOCUS_EMOJI[issue.category]}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
 <span style={{
 fontSize: "var(--font-size-xs)", padding: '1px 6px', borderRadius: "var(--radius-md)",
 background: sty.badge.includes('red') ? 'var(--text-danger)'
 : sty.badge.includes('yellow') ? 'var(--text-warning)' : 'var(--text-info)',
 color: sty.badge.includes('yellow') ? '#000' : '#fff',
 }}>
 {issue.severity}
 </span>
 <span style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>{issue.category}</span>
 </div>
 <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)', marginTop: 3, lineHeight: 1.4 }}>
 {issue.description}
 </div>
 {issue.file && (
 <button
 onClick={(e) => {
 e.stopPropagation();
 onOpenFile?.(issue.file, issue.line);
 }}
 style={{
 marginTop: 4, fontSize: "var(--font-size-xs)", color: 'var(--accent-color)', background: 'none',
 border: 'none', cursor: 'pointer', padding: 0, textDecoration: 'underline',
 }}
 >
 {issue.file}{issue.line ? `:${issue.line}` : ''}
 </button>
 )}
 </div>
 {!isOpen && <ChevronDown size={12} style={{ color: 'var(--text-secondary)', flexShrink: 0 }} />}
 </div>

 {isOpen && issue.suggested_fix && (
 <div style={{ padding: '0 10px 10px', borderTop: '1px solid var(--border-color)', marginTop: 4 }}>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4, paddingTop: 8 }}>
 Suggested fix:
 </div>
 <pre style={{
 margin: 0, fontSize: "var(--font-size-sm)", color: 'var(--text-success)', background: 'var(--bg-primary)',
 borderRadius: "var(--radius-xs-plus)", padding: 8, overflowX: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
 }}>
 {issue.suggested_fix}
 </pre>
 </div>
 )}
 </div>
 );
 })}
 </div>
 )}

 {/* Suggestions */}
 {report.suggestions.length > 0 && (
 <div style={{ background: 'var(--bg-tertiary)', borderRadius: "var(--radius-sm)", padding: 12 }}>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8, color: 'var(--text-primary)' }}>
 <Lightbulb size={14} strokeWidth={1.5} /> Suggestions ({report.suggestions.length})
 </div>
 {report.suggestions.map((s, i) => (
 <div key={i} style={{ fontSize: "var(--font-size-base)", color: 'var(--text-secondary)', marginBottom: 4, paddingLeft: 8, borderLeft: '2px solid var(--border-color)' }}>
 {s.description}
 {s.file && (
 <span style={{ marginLeft: 6, color: 'var(--accent-color)', fontSize: "var(--font-size-sm)" }}>— {s.file}</span>
 )}
 </div>
 ))}
 </div>
 )}
 </div>
 )}

 {/* ── Empty state ── */}
 {!report && !isLoading && !error && (
 <div className="panel-empty">
 <div style={{ fontSize: "var(--font-size-md)" }}>Run a code review to see issues</div>
 <div style={{ fontSize: "var(--font-size-sm)" }}>Analyzes your uncommitted changes or compares branches</div>
 </div>
 )}
 </div>
 </div>
 );
}
