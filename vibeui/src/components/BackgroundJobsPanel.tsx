import React, { useState, useEffect, useRef } from 'react';
import { Clock, CircleCheck, CircleX, Square } from 'lucide-react';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

interface JobRecord {
 session_id: string;
 task: string;
 status: 'running' | 'complete' | 'failed' | 'cancelled';
 provider: string;
 started_at: number;
 finished_at?: number;
 summary?: string;
}

interface BackgroundJobsPanelProps {
 /** URL of the vibecli daemon (default: http://localhost:7878) */
 daemonUrl?: string;
}

const STATUS_ICONS: Record<string, React.ReactNode> = {
 running: <Clock size={14} strokeWidth={1.5} style={{ color: "#facc15" }} />,
 complete: <CircleCheck size={14} strokeWidth={1.5} style={{ color: "var(--text-success, #a6e3a1)" }} />,
 failed: <CircleX size={14} strokeWidth={1.5} style={{ color: "var(--text-danger, #f38ba8)" }} />,
 cancelled: <Square size={14} strokeWidth={1.5} />,
};

const PROVIDERS = ['ollama', 'claude', 'openai', 'gemini', 'grok'];
const APPROVALS = ['suggest', 'auto-edit', 'full-auto'];

export function BackgroundJobsPanel({ daemonUrl = 'http://localhost:7878' }: BackgroundJobsPanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [jobs, setJobs] = useState<JobRecord[]>([]);
 const [daemonOnline, setDaemonOnline] = useState(false);
 const [expandedId, setExpandedId] = useState<string | null>(null);
 const [liveEvents, setLiveEvents] = useState<Record<string, string[]>>({});
 const [task, setTask] = useState('');
 const [provider, setProvider] = useState('ollama');
 const [approval, setApproval] = useState('suggest');
 const [submitting, setSubmitting] = useState(false);
 const esRefs = useRef<Record<string, EventSource>>({});

 const fetchJobs = async () => {
 try {
 const res = await fetch(`${daemonUrl}/jobs`);
 if (!res.ok) throw new Error(await res.text());
 const data: JobRecord[] = await res.json();
 setJobs(data);
 setDaemonOnline(true);
 } catch {
 setDaemonOnline(false);
 }
 };

 // Poll every 5 seconds
 useEffect(() => {
 fetchJobs();
 const id = setInterval(fetchJobs, 5000);
 return () => clearInterval(id);
 }, [daemonUrl]);

 // Close all open EventSources on unmount to prevent resource leaks
 useEffect(() => {
 return () => {
 Object.values(esRefs.current).forEach(es => es.close());
 esRefs.current = {};
 };
 }, []);

 const submitJob = async () => {
 if (!task.trim()) return;
 setSubmitting(true);
 try {
 const res = await fetch(`${daemonUrl}/agent`, {
 method: 'POST',
 headers: { 'Content-Type': 'application/json' },
 body: JSON.stringify({ task, approval }),
 });
 if (!res.ok) throw new Error(await res.text());
 setTask('');
 await fetchJobs();
 } catch (e) {
 toast.error(`Failed to submit job: ${e}`);
 } finally {
 setSubmitting(false);
 }
 };

 const cancelJob = async (id: string) => {
 try {
 await fetch(`${daemonUrl}/jobs/${id}/cancel`, { method: 'POST' });
 await fetchJobs();
 } catch (e) {
 toast.error(`Failed to cancel: ${e}`);
 }
 };

 const streamLive = (id: string) => {
 if (esRefs.current[id]) {
 esRefs.current[id].close();
 delete esRefs.current[id];
 setLiveEvents((prev) => { const n = { ...prev }; delete n[id]; return n; });
 return;
 }
 const es = new EventSource(`${daemonUrl}/stream/${id}`);
 es.onmessage = (e) => {
 try {
 const payload = JSON.parse(e.data);
 const line = `[${payload.type}] ${payload.content ?? payload.tool_name ?? ''}`;
 setLiveEvents((prev) => ({
 ...prev,
 [id]: [...(prev[id] ?? []).slice(-49), line],
 }));
 if (payload.type === 'complete' || payload.type === 'error') {
 es.close();
 delete esRefs.current[id];
 fetchJobs();
 }
 } catch { /* ignore */ }
 };
 es.onerror = () => { es.close(); delete esRefs.current[id]; };
 esRefs.current[id] = es;
 };

 const fmtTime = (ms: number) =>
 new Date(ms).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' });

 return (
 <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', padding: '8px' }}>
 {/* Header */}
 <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
 <strong style={{ fontSize: '13px' }}>Background Jobs</strong>
 <span style={{
 fontSize: '10px', padding: '2px 8px', borderRadius: '10px',
 background: daemonOnline ? 'var(--accent-green, #52c41a)' : 'var(--text-secondary)',
 color: '#fff',
 }}>
 {daemonOnline ? '● online' : '○ offline'}
 </span>
 </div>

 {!daemonOnline && (
 <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '8px', padding: '6px', background: 'var(--bg-tertiary)', borderRadius: '4px' }}>
 Daemon not running. Start it with: <code>vibecli serve --port 7878</code>
 </div>
 )}

 {/* Submit form */}
 <div style={{ marginBottom: '10px', padding: '8px', background: 'var(--bg-secondary)', borderRadius: '6px' }}>
 <textarea
 value={task}
 onChange={(e) => setTask(e.target.value)}
 placeholder="Describe a background agent task…"
 rows={2}
 style={{
 width: '100%', boxSizing: 'border-box', resize: 'vertical',
 padding: '6px', fontSize: '12px', fontFamily: 'inherit',
 background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)',
 color: 'var(--text-primary)', borderRadius: '4px', marginBottom: '6px',
 }}
 />
 <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
 <select
 value={provider}
 onChange={(e) => setProvider(e.target.value)}
 style={selectStyle}
 >
 {PROVIDERS.map((p) => <option key={p} value={p}>{p}</option>)}
 </select>
 <select
 value={approval}
 onChange={(e) => setApproval(e.target.value)}
 style={selectStyle}
 >
 {APPROVALS.map((a) => <option key={a} value={a}>{a}</option>)}
 </select>
 <button
 onClick={submitJob}
 disabled={submitting || !task.trim() || !daemonOnline}
 style={{
 padding: '4px 12px', fontSize: '12px', borderRadius: '4px',
 background: 'var(--accent-blue)', color: '#fff', border: 'none',
 cursor: submitting ? 'wait' : 'pointer', marginLeft: 'auto',
 }}
 >
 {submitting ? 'Submitting…' : ' Submit'}
 </button>
 </div>
 </div>

 {/* Job list */}
 <div style={{ flex: 1, overflowY: 'auto' }}>
 {jobs.length === 0 && daemonOnline && (
 <p style={{ fontSize: '12px', color: 'var(--text-secondary)', textAlign: 'center', marginTop: '20px' }}>
 No jobs yet. Submit one above.
 </p>
 )}
 {jobs.map((job) => (
 <div key={job.session_id} style={{
 marginBottom: '6px', borderRadius: '6px',
 background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
 }}>
 {/* Job row */}
 <div
 onClick={() => setExpandedId(expandedId === job.session_id ? null : job.session_id)}
 style={{ padding: '8px 10px', cursor: 'pointer', display: 'flex', alignItems: 'flex-start', gap: '8px' }}
 >
 <span style={{ fontSize: '14px', flexShrink: 0 }}>{STATUS_ICONS[job.status] ?? '?'}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: '12px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
 {job.task}
 </div>
 <div style={{ fontSize: '10px', color: 'var(--text-secondary)', marginTop: '2px' }}>
 {job.provider} · {job.status} · {fmtTime(job.started_at)}
 </div>
 </div>
 {job.status === 'running' && (
 <button
 onClick={(e) => { e.stopPropagation(); cancelJob(job.session_id); }}
 style={{ fontSize: '10px', padding: '2px 6px', border: 'none', borderRadius: '3px', background: 'var(--text-danger, #ff4d4f)', color: '#fff', cursor: 'pointer', flexShrink: 0 }}
 >
 Cancel
 </button>
 )}
 </div>

 {/* Expanded detail */}
 {expandedId === job.session_id && (
 <div style={{ borderTop: '1px solid var(--border-color)', padding: '8px 10px' }}>
 {job.summary && (
 <div style={{ fontSize: '11px', marginBottom: '6px', whiteSpace: 'pre-wrap' }}>
 <strong>Summary:</strong> {job.summary}
 </div>
 )}
 {job.status === 'running' && (
 <button
 onClick={() => streamLive(job.session_id)}
 style={{ fontSize: '11px', padding: '2px 8px', border: '1px solid var(--border-color)', borderRadius: '3px', background: 'none', color: 'var(--accent-blue)', cursor: 'pointer', marginBottom: '6px' }}
 >
 {esRefs.current[job.session_id] ? ' Stop stream' : ' Stream live'}
 </button>
 )}
 {liveEvents[job.session_id] && liveEvents[job.session_id].length > 0 && (
 <div style={{
 fontSize: '10px', fontFamily: 'monospace', maxHeight: '120px',
 overflowY: 'auto', background: 'var(--bg-tertiary)', padding: '4px 6px',
 borderRadius: '4px', color: 'var(--text-secondary)',
 }}>
 {liveEvents[job.session_id].map((line, i) => (
 <div key={i}>{line}</div>
 ))}
 </div>
 )}
 </div>
 )}
 </div>
 ))}
 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}

const selectStyle: React.CSSProperties = {
 padding: '3px 6px', fontSize: '11px', borderRadius: '4px',
 background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)',
 color: 'var(--text-primary)',
};
