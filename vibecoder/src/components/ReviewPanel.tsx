import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Lightbulb, ChevronDown } from 'lucide-react';
import { FixWithAIButton } from './FixWithAIButton';
import type { FixItem } from '../lib/fixWithAI';

// ── Types (mirrors review.rs) ──────────────────────────────────────────────

type Severity = 'info' | 'warning' | 'critical';
type ReviewFocus = 'security' | 'performance' | 'correctness' | 'style' | 'testing';

interface ReviewIssue {
 file: string;
 line: number;
 /** Which bucket the issue is filtered and coloured by. */
 severity: Severity;
 /** What the model actually called it. Shown on the badge, so a review that
  *  said "blocker" is not silently relabelled "critical" on screen. */
 severityLabel: string;
 category: string;
 description: string;
 suggested_fix?: string;
}

interface ReviewSuggestion {
 description: string;
 file?: string;
}

/** A score the model did not give is `null`, never 0 — see `parseScore`. */
interface ReviewScore {
 overall: number | null;
 correctness: number | null;
 security: number | null;
 performance: number | null;
 style: number | null;
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
 /** Provider name from the toolbar dropdown — forwarded so the AI reviewer
  *  uses the user's selected model instead of the chat engine's default. */
 selectedProvider?: string;
}

// ── Helpers ────────────────────────────────────────────────────────────────

const SEVERITY_COLORS: Record<Severity, { badge: string; text: string; border: string }> = {
 critical: { badge: 'var(--text-danger)', text: 'var(--btn-primary-fg)', border: 'var(--text-danger)' },
 warning: { badge: 'var(--text-warning)', text: 'var(--text-primary)', border: 'var(--text-warning)' },
 info: { badge: 'var(--text-info)', text: 'var(--btn-primary-fg)', border: 'var(--text-info)' },
};

/**
 * Words a model uses for each bucket, beyond the three the prompt asks for.
 *
 * The prompt says `critical|warning|info`; models answer "high", "major",
 * "Error", "nit". Indexing a style map with whatever came back is what crashed
 * the app — `SEVERITY_COLORS[severity].border` on an unlisted word threw out of
 * a render, and with the Source Control sidebar outside any error boundary that
 * took the whole window with it.
 */
const SEVERITY_SYNONYMS: Record<string, Severity> = {
 critical: 'critical', blocker: 'critical', high: 'critical', severe: 'critical',
 major: 'critical', error: 'critical', bug: 'critical',
 warning: 'warning', warn: 'warning', medium: 'warning', moderate: 'warning',
 info: 'info', low: 'info', minor: 'info', nit: 'info', note: 'info',
 suggestion: 'info', trivial: 'info', style: 'info',
};

/**
 * Which bucket to colour and filter an issue by.
 *
 * An unrecognised word reads as `info` — the quietest bucket, so a mystery
 * label cannot dress a nit up as a blocker. The word itself is kept and shown;
 * only the colour is a guess, and a guess about a colour is not a claim about
 * the code.
 */
function severityBucket(raw: unknown): Severity {
 const word = typeof raw === 'string' ? raw.trim().toLowerCase() : '';
 return SEVERITY_SYNONYMS[word] ?? 'info';
}

/**
 * One issue as the shared chat hand-off carries it.
 *
 * The severity that travels is the word the model used, not the bucket it was
 * coloured by: a review that said "blocker" must not reach chat as "info"
 * because nothing here recognised the word.
 */
function toFixItem(issue: ReviewIssue): FixItem {
 return {
  file: issue.file || null,
  line: issue.line || null,
  severity: issue.severityLabel,
  title: issue.category || null,
  message: issue.description,
  suggestion: issue.suggested_fix ?? null,
 };
}

const str = (v: unknown): string => (typeof v === 'string' ? v : '');
const strList = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);

/**
 * A score, or `null` when the model did not give one.
 *
 * Not `0`: a missing security score rendered as 0.0/10 is an accusation nobody
 * made, and rendered as 10 it is a clean bill of health nobody gave. Absent
 * stays absent, and the bar says so.
 */
function parseScore(v: unknown): number | null {
 return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

function parseIssue(raw: unknown): ReviewIssue {
 const o = (raw ?? {}) as Record<string, unknown>;
 const label = str(o.severity);
 return {
  file: str(o.file),
  line: typeof o.line === 'number' ? o.line : 0,
  severity: severityBucket(o.severity),
  severityLabel: label || 'unlabelled',
  category: str(o.category),
  description: str(o.description),
  suggested_fix: typeof o.suggested_fix === 'string' ? o.suggested_fix : undefined,
 };
}

/**
 * Turn the reviewer's reply into something this panel can render.
 *
 * `invoke<ReviewReport>` is a cast, not a check: the JSON on the other side is
 * whatever the model wrote, and the interface above is a compile-time story
 * about it. Every field is read through this once, at the edge, so nothing
 * inward has to wonder whether `issues` is an array or `score` exists.
 */
function parseReviewReport(raw: unknown): ReviewReport {
 const o = (raw ?? {}) as Record<string, unknown>;
 const score = (o.score ?? {}) as Record<string, unknown>;
 return {
  base_ref: str(o.base_ref),
  target_ref: str(o.target_ref),
  summary: str(o.summary),
  issues: Array.isArray(o.issues) ? o.issues.map(parseIssue) : [],
  suggestions: Array.isArray(o.suggestions)
   ? o.suggestions.map((sug) => {
     const su = (sug ?? {}) as Record<string, unknown>;
     return { description: str(su.description), file: typeof su.file === 'string' ? su.file : undefined };
    })
   : [],
  score: {
   overall: parseScore(score.overall),
   correctness: parseScore(score.correctness),
   security: parseScore(score.security),
   performance: parseScore(score.performance),
   style: parseScore(score.style),
  },
  files_reviewed: strList(o.files_reviewed),
 };
}

const FOCUS_EMOJI: Record<ReviewFocus, string> = {
 security: '',
 performance: '',
 correctness: '',
 style: '',
 testing: '',
};

/** One score. `null` means the reviewer did not give this one — the bar stays
 *  empty and reads "not scored" rather than showing a number nobody said. */
function ScoreBar({ label, value }: { label: string; value: number | null }) {
 const pct = value === null ? 0 : Math.round((value / 10) * 100);
 const color = value === null
  ? 'var(--text-secondary)'
  : value >= 8 ? 'var(--text-success)' : value >= 5 ? 'var(--text-warning)' : 'var(--text-danger)';
 return (
 <div style={{ marginBottom: 6 }}>
 <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: "var(--font-size-base)", marginBottom: 2 }}>
 <span>{label}</span>
 <span style={{ color }}>{value === null ? 'not scored' : value.toFixed(1)}</span>
 </div>
 <div style={{ background: 'var(--border-color)', borderRadius: "var(--radius-xs-plus)", height: 6, overflow: 'hidden' }}>
 <div style={{ width: `${pct}%`, height: '100%', background: color, transition: 'width 0.3s' }} />
 </div>
 </div>
 );
}

// ── Component ──────────────────────────────────────────────────────────────

export function ReviewPanel({ workspacePath, onOpenFile, selectedProvider }: ReviewPanelProps) {
 const [isLoading, setIsLoading] = useState(false);
 const [report, setReport] = useState<ReviewReport | null>(null);
 const [error, setError] = useState<string | null>(null);
 const [filterSeverity, setFilterSev] = useState<Severity | 'all'>('all');
 // Bumped by each review so a hand-off button stops claiming the previous
 // report's issues were sent.
 const [runId, setRunId] = useState(0);
 const [baseRef, setBaseRef] = useState('');
 const [expandedIssue, setExpanded] = useState<number | null>(null);

 const runReview = async () => {
 if (!workspacePath) return;
 setIsLoading(true);
 setError(null);
 setReport(null);
 setRunId((n) => n + 1);
 try {
 const result = await invoke<unknown>('run_code_review', {
 workspacePath,
 baseRef: baseRef.trim() || null,
 targetRef: null,
 provider: selectedProvider || null,
 });
 // The command hands back the model's JSON verbatim. Everything below this
 // line reads a checked shape; nothing reads the reply.
 setReport(parseReviewReport(result));
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
 Quality Score{report.score.overall !== null ? ` — Overall: ${report.score.overall.toFixed(1)} / 10` : ''}
 </div>
 <ScoreBar label="Correctness" value={report.score.correctness} />
 <ScoreBar label="Security" value={report.score.security} />
 <ScoreBar label="Performance" value={report.score.performance} />
 <ScoreBar label="Style" value={report.score.style} />
 </div>

 {/* Severity filter tabs */}
 {report.issues.length > 0 && (
 <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
 {(['all', 'critical', 'warning', 'info'] as const).map((sev) => {
 const count = sev === 'all' ? report.issues.length : countBySev(sev);
 const active = filterSeverity === sev;
 return (
 <button
 key={sev}
 onClick={() => setFilterSev(sev)}
 style={{
 padding: '3px 12px', borderRadius: 12, border: '1px solid var(--border-color)',
 background: active ? 'var(--border-color)' : 'transparent', color: active ? 'var(--text-primary)' : 'var(--text-secondary)',
 fontSize: "var(--font-size-sm)", cursor: 'pointer',
 }}
 >
 {sev === 'all' ? 'All' : sev.charAt(0).toUpperCase() + sev.slice(1)} ({count})
 </button>
 );
 })}
 <span style={{ flex: 1 }} />
 <FixWithAIButton
 items={filteredIssues.map(toFixItem)}
 source="code review"
 resetKey={runId}
 label={`Fix all ${filteredIssues.length} with AI`}
 title={filterSeverity === 'all'
 ? 'Write a fix request for the issues shown into the chat composer'
 : `Write a fix request for the ${filterSeverity} issues shown into the chat composer`}
 />
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
 const sty = SEVERITY_COLORS[issue.severity];
 const isOpen = expandedIssue === idx;
 return (
 <div
 key={idx}
 style={{
 background: 'var(--bg-tertiary)', borderRadius: "var(--radius-sm)", borderLeft: `3px solid`,
 borderLeftColor: sty.border,
 overflow: 'hidden',
 }}
 >
 <div role="button" tabIndex={0}
 style={{ padding: '8px 12px', cursor: 'pointer', display: 'flex', gap: 8, alignItems: 'flex-start' }}
 onClick={() => setExpanded(isOpen ? null : idx)}
 >
 <span style={{ fontSize: "var(--font-size-sm)" }}>{FOCUS_EMOJI[issue.category as ReviewFocus] ?? ''}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
 <span style={{
 fontSize: "var(--font-size-xs)", padding: '1px 8px', borderRadius: "var(--radius-md)",
 background: sty.badge,
 color: sty.text,
 }}>
 {issue.severityLabel}
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
 <FixWithAIButton
 items={[toFixItem(issue)]}
 source="code review"
 resetKey={runId}
 title="Write a fix request for this issue into the chat composer"
 />
 {!isOpen && <ChevronDown size={12} style={{ color: 'var(--text-secondary)', flexShrink: 0 }} />}
 </div>

 {isOpen && issue.suggested_fix && (
 <div style={{ padding: '0 12px 12px', borderTop: '1px solid var(--border-color)', marginTop: 4 }}>
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
 <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: 'var(--text-primary)', flex: 1 }}>
 <Lightbulb size={14} strokeWidth={1.5} /> Suggestions ({report.suggestions.length})
 </div>
 <FixWithAIButton
 items={report.suggestions.map((sg) => ({ file: sg.file ?? null, message: sg.description }))}
 source="code review suggestion"
 resetKey={runId}
 label={`Fix all ${report.suggestions.length} with AI`}
 title="Write a change request for these suggestions into the chat composer"
 />
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
