import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Lightbulb } from 'lucide-react';

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
 const color = value >= 8 ? '#22c55e' : value >= 5 ? '#f59e0b' : '#ef4444';
 return (
 <div style={{ marginBottom: 6 }}>
 <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, marginBottom: 2 }}>
 <span>{label}</span>
 <span style={{ color }}>{value.toFixed(1)}</span>
 </div>
 <div style={{ background: '#374151', borderRadius: 4, height: 6, overflow: 'hidden' }}>
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
 <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', padding: 12, gap: 10 }}>
 {/* ── Toolbar ── */}
 <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
 <input
 value={baseRef}
 onChange={(e) => setBaseRef(e.target.value)}
 placeholder="Base ref (e.g. main, HEAD~1) — leave blank for uncommitted"
 style={{
 flex: 1, minWidth: 180, padding: '4px 8px', borderRadius: 4,
 border: '1px solid #374151', background: '#1f2937', color: '#f3f4f6', fontSize: 12,
 }}
 />
 <button
 onClick={runReview}
 disabled={isLoading || !workspacePath}
 style={{
 padding: '5px 14px', borderRadius: 4, border: 'none', cursor: 'pointer',
 background: isLoading ? '#374151' : '#6366f1', color: '#fff', fontSize: 13,
 opacity: !workspacePath ? 0.5 : 1,
 }}
 >
 {isLoading ? ' Reviewing…' : ' Run Review'}
 </button>
 </div>

 {/* ── Error ── */}
 {error && (
 <div style={{ padding: 8, borderRadius: 4, background: '#7f1d1d', color: '#fca5a5', fontSize: 12 }}>
 {error}
 </div>
 )}

 {/* ── Loading placeholder ── */}
 {isLoading && (
 <div style={{ textAlign: 'center', color: '#9ca3af', paddingTop: 32, fontSize: 13 }}>
 Analyzing diff…<br />
 <span style={{ fontSize: 11, color: '#6b7280' }}>This may take 15–30 seconds depending on diff size</span>
 </div>
 )}

 {/* ── Report ── */}
 {report && !isLoading && (
 <div style={{ flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 10 }}>

 {/* Summary card */}
 <div style={{ background: '#1f2937', borderRadius: 6, padding: 12 }}>
 <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 6, color: '#e5e7eb' }}>
 Review Summary {report.base_ref && (
 <span style={{ fontWeight: 400, color: '#9ca3af', fontSize: 11 }}>
 ({report.base_ref || 'working tree'} → {report.target_ref || 'HEAD'})
 </span>
 )}
 </div>
 <p style={{ fontSize: 12, color: '#d1d5db', margin: 0, lineHeight: 1.5 }}>{report.summary}</p>
 {report.files_reviewed.length > 0 && (
 <div style={{ marginTop: 8, fontSize: 11, color: '#6b7280' }}>
 {report.files_reviewed.length} file{report.files_reviewed.length !== 1 ? 's' : ''} reviewed
 </div>
 )}
 </div>

 {/* Score bars */}
 <div style={{ background: '#1f2937', borderRadius: 6, padding: 12 }}>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: '#e5e7eb' }}>
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
 padding: '3px 10px', borderRadius: 12, border: '1px solid #374151',
 background: active ? '#374151' : 'transparent', color: active ? '#f3f4f6' : '#9ca3af',
 fontSize: 11, cursor: 'pointer',
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
 <div style={{ color: '#9ca3af', fontSize: 12, textAlign: 'center', paddingTop: 12 }}>
 No {filterSeverity} issues found.
 </div>
 ) : filteredIssues.length === 0 ? (
 <div style={{ color: '#22c55e', fontSize: 13, textAlign: 'center', paddingTop: 12 }}>
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
 background: '#1f2937', borderRadius: 6, borderLeft: `3px solid`,
 borderLeftColor: sty.border.replace('border-l-', '').includes('red') ? '#ef4444'
 : sty.border.includes('yellow') ? '#f59e0b' : '#60a5fa',
 overflow: 'hidden',
 }}
 >
 <div
 style={{ padding: '8px 10px', cursor: 'pointer', display: 'flex', gap: 8, alignItems: 'flex-start' }}
 onClick={() => setExpanded(isOpen ? null : idx)}
 >
 <span style={{ fontSize: 11 }}>{FOCUS_EMOJI[issue.category]}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
 <span style={{
 fontSize: 10, padding: '1px 6px', borderRadius: 10,
 background: sty.badge.includes('red') ? '#ef4444'
 : sty.badge.includes('yellow') ? '#f59e0b' : '#60a5fa',
 color: sty.badge.includes('yellow') ? '#000' : '#fff',
 }}>
 {issue.severity}
 </span>
 <span style={{ fontSize: 11, color: '#9ca3af' }}>{issue.category}</span>
 </div>
 <div style={{ fontSize: 12, color: '#e5e7eb', marginTop: 3, lineHeight: 1.4 }}>
 {issue.description}
 </div>
 {issue.file && (
 <button
 onClick={(e) => {
 e.stopPropagation();
 onOpenFile?.(issue.file, issue.line);
 }}
 style={{
 marginTop: 4, fontSize: 10, color: '#818cf8', background: 'none',
 border: 'none', cursor: 'pointer', padding: 0, textDecoration: 'underline',
 }}
 >
 {issue.file}{issue.line ? `:${issue.line}` : ''}
 </button>
 )}
 </div>
 <span style={{ color: '#6b7280', fontSize: 11, flexShrink: 0 }}>{isOpen ? '' : '▼'}</span>
 </div>

 {isOpen && issue.suggested_fix && (
 <div style={{ padding: '0 10px 10px', borderTop: '1px solid #374151', marginTop: 4 }}>
 <div style={{ fontSize: 11, color: '#9ca3af', marginBottom: 4, paddingTop: 8 }}>
 Suggested fix:
 </div>
 <pre style={{
 margin: 0, fontSize: 11, color: '#86efac', background: '#111827',
 borderRadius: 4, padding: 8, overflowX: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
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
 <div style={{ background: '#1f2937', borderRadius: 6, padding: 12 }}>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: '#e5e7eb' }}>
 <Lightbulb size={14} strokeWidth={1.5} /> Suggestions ({report.suggestions.length})
 </div>
 {report.suggestions.map((s, i) => (
 <div key={i} style={{ fontSize: 12, color: '#d1d5db', marginBottom: 4, paddingLeft: 8, borderLeft: '2px solid #374151' }}>
 {s.description}
 {s.file && (
 <span style={{ marginLeft: 6, color: '#818cf8', fontSize: 11 }}>— {s.file}</span>
 )}
 </div>
 ))}
 </div>
 )}
 </div>
 )}

 {/* ── Empty state ── */}
 {!report && !isLoading && !error && (
 <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', gap: 8, color: '#6b7280' }}>
 <div style={{ fontSize: 32 }}></div>
 <div style={{ fontSize: 13 }}>Run a code review to see issues</div>
 <div style={{ fontSize: 11 }}>Analyzes your uncommitted changes or compares branches</div>
 </div>
 )}
 </div>
 );
}
