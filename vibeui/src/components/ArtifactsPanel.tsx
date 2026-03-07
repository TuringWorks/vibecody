import { useState } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface TaskItem {
 id: number;
 description: string;
 done: boolean;
 file?: string;
}

interface PlanStep {
 id: number;
 description: string;
 tool: string;
 estimated_path?: string;
 status: string;
}

interface ReviewIssueRef {
 file: string;
 line: number;
 severity: string;
 description: string;
}

type ArtifactPayload =
 | { type: "task_list"; items: TaskItem[] }
 | { type: "implementation_plan"; steps: PlanStep[]; files: string[] }
 | { type: "file_change"; path: string; diff: string; content?: string }
 | { type: "command_output"; command: string; stdout: string; stderr: string; exit_code: number }
 | { type: "test_results"; passed: number; failed: number; skipped: number; output: string }
 | { type: "review_report"; issues: ReviewIssueRef[]; summary: string; score: number }
 | { type: "text"; title: string; content: string };

interface AgentArtifact {
 id: string;
 step: number;
 artifact: ArtifactPayload;
 timestamp: number;
 annotations: Array<{ text: string; timestamp: number; applied: boolean }>;
}

// ── Props ─────────────────────────────────────────────────────────────────────

interface ArtifactsPanelProps {
 artifacts: AgentArtifact[];
 onAnnotate?: (artifactId: string, text: string) => void;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function artifactIcon(a: ArtifactPayload): string {
 switch (a.type) {
 case "task_list": return "";
 case "implementation_plan": return "";
 case "file_change": return "";
 case "command_output": return "";
 case "test_results": return "";
 case "review_report": return "";
 case "text": return "";
 }
}

function artifactLabel(a: ArtifactPayload): string {
 switch (a.type) {
 case "task_list": return "Task List";
 case "implementation_plan": return "Implementation Plan";
 case "file_change": return `File Change: ${a.path}`;
 case "command_output": return `Command: ${a.command}`;
 case "test_results": return `Tests: ${a.passed} passed, ${a.failed} failed`;
 case "review_report": return `Code Review (score: ${a.score.toFixed(1)})`;
 case "text": return a.title;
 }
}

function severityColor(severity: string): string {
 switch (severity) {
 case "critical": return "var(--error-color, #f44)";
 case "warning": return "var(--warning-color, #fa0)";
 default: return "var(--info-color, #5af)";
 }
}

// ── Artifact body renderers ───────────────────────────────────────────────────

function TaskListBody({ items }: { items: TaskItem[] }) {
 return (
 <div>
 {items.map((item) => (
 <div key={item.id} style={{ display: "flex", alignItems: "flex-start", gap: "8px", marginBottom: "4px" }}>
 <span style={{ color: item.done ? "var(--success-color, #4c4)" : "var(--text-secondary)", marginTop: "2px" }}>
 {item.done ? "☑" : "☐"}
 </span>
 <span style={{ fontSize: "12px", color: item.done ? "var(--text-secondary)" : "var(--text-primary)", textDecoration: item.done ? "line-through" : "none" }}>
 {item.description}
 {item.file && <span style={{ marginLeft: "6px", opacity: 0.6, fontStyle: "italic" }}>{item.file}</span>}
 </span>
 </div>
 ))}
 </div>
 );
}

function ImplementationPlanBody({ steps, files }: { steps: PlanStep[]; files: string[] }) {
 return (
 <div>
 {files.length > 0 && (
 <div style={{ marginBottom: "8px", fontSize: "11px", color: "var(--text-secondary)" }}>
 Files: {files.join(", ")}
 </div>
 )}
 {steps.map((step, i) => (
 <div key={step.id} style={{ display: "flex", gap: "8px", marginBottom: "6px", alignItems: "flex-start" }}>
 <span style={{ fontSize: "11px", color: "var(--text-secondary)", minWidth: "20px", textAlign: "right" }}>
 {i + 1}.
 </span>
 <div>
 <div style={{ fontSize: "12px", color: "var(--text-primary)" }}>{step.description}</div>
 {step.estimated_path && (
 <div style={{ fontSize: "10px", color: "var(--accent-blue, #007acc)", marginTop: "1px" }}>
 {step.estimated_path}
 </div>
 )}
 </div>
 </div>
 ))}
 </div>
 );
}

function FileChangeBody({ path, diff }: { path: string; diff: string }) {
 const lines = diff.split("\n").slice(0, 40);
 return (
 <div>
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginBottom: "6px" }}>{path}</div>
 {diff ? (
 <pre style={{ fontSize: "11px", overflowX: "auto", maxHeight: "200px", overflowY: "auto", margin: 0, background: "var(--bg-primary)", padding: "8px", borderRadius: "4px", lineHeight: 1.4 }}>
 {lines.map((line, i) => (
 <div key={i} style={{ color: line.startsWith("+") ? "#4c4" : line.startsWith("-") ? "#f44" : "var(--text-secondary)" }}>
 {line}
 </div>
 ))}
 {diff.split("\n").length > 40 && <div style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>… {diff.split("\n").length - 40} more lines</div>}
 </pre>
 ) : (
 <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>New file</div>
 )}
 </div>
 );
}

function CommandOutputBody({ command, stdout, stderr, exit_code }: { command: string; stdout: string; stderr: string; exit_code: number }) {
 return (
 <div>
 <div style={{ fontSize: "11px", fontFamily: "monospace", color: "var(--accent-blue, #007acc)", marginBottom: "6px" }}>$ {command}</div>
 {stdout && (
 <pre style={{ fontSize: "11px", margin: "0 0 4px 0", maxHeight: "120px", overflowY: "auto", color: "var(--text-primary)" }}>
 {stdout.trim()}
 </pre>
 )}
 {stderr && (
 <pre style={{ fontSize: "11px", margin: "0 0 4px 0", color: "var(--error-color, #f44)", maxHeight: "80px", overflowY: "auto" }}>
 {stderr.trim()}
 </pre>
 )}
 <div style={{ fontSize: "10px", color: exit_code === 0 ? "var(--success-color, #4c4)" : "var(--error-color, #f44)" }}>
 Exit code: {exit_code}
 </div>
 </div>
 );
}

function TestResultsBody({ passed, failed, skipped, output }: { passed: number; failed: number; skipped: number; output: string }) {
 return (
 <div>
 <div style={{ display: "flex", gap: "12px", marginBottom: "8px" }}>
 <span style={{ fontSize: "12px", color: "var(--success-color, #4c4)" }}> {passed} passed</span>
 {failed > 0 && <span style={{ fontSize: "12px", color: "var(--error-color, #f44)" }}> {failed} failed</span>}
 {skipped > 0 && <span style={{ fontSize: "12px", color: "var(--text-secondary)" }}>{skipped} skipped</span>}
 </div>
 {output && (
 <pre style={{ fontSize: "10px", maxHeight: "100px", overflowY: "auto", margin: 0, color: "var(--text-secondary)" }}>
 {output.trim()}
 </pre>
 )}
 </div>
 );
}

function ReviewReportBody({ issues, summary, score }: { issues: ReviewIssueRef[]; summary: string; score: number }) {
 return (
 <div>
 <div style={{ fontSize: "12px", color: "var(--text-primary)", marginBottom: "8px" }}>{summary}</div>
 <div style={{ fontSize: "11px", color: score >= 8 ? "var(--success-color, #4c4)" : score >= 5 ? "var(--warning-color, #fa0)" : "var(--error-color, #f44)", marginBottom: "8px" }}>
 Score: {score.toFixed(1)}/10
 </div>
 {issues.map((issue, i) => (
 <div key={i} style={{ padding: "4px 0", borderTop: "1px solid var(--border-color)" }}>
 <span style={{ fontSize: "11px", color: severityColor(issue.severity), marginRight: "6px" }}>
 {issue.severity}
 </span>
 <span style={{ fontSize: "11px", color: "var(--text-secondary)", marginRight: "6px" }}>
 {issue.file}:{issue.line}
 </span>
 <span style={{ fontSize: "11px", color: "var(--text-primary)" }}>{issue.description}</span>
 </div>
 ))}
 </div>
 );
}

function TextBody({ content }: { content: string }) {
 return (
 <pre style={{ fontSize: "12px", whiteSpace: "pre-wrap", wordBreak: "break-word", margin: 0, color: "var(--text-primary)", maxHeight: "200px", overflowY: "auto" }}>
 {content}
 </pre>
 );
}

function ArtifactBody({ artifact }: { artifact: ArtifactPayload }) {
 switch (artifact.type) {
 case "task_list": return <TaskListBody items={artifact.items} />;
 case "implementation_plan": return <ImplementationPlanBody steps={artifact.steps} files={artifact.files} />;
 case "file_change": return <FileChangeBody path={artifact.path} diff={artifact.diff} />;
 case "command_output": return <CommandOutputBody command={artifact.command} stdout={artifact.stdout} stderr={artifact.stderr} exit_code={artifact.exit_code} />;
 case "test_results": return <TestResultsBody passed={artifact.passed} failed={artifact.failed} skipped={artifact.skipped} output={artifact.output} />;
 case "review_report": return <ReviewReportBody issues={artifact.issues} summary={artifact.summary} score={artifact.score} />;
 case "text": return <TextBody content={artifact.content} />;
 }
}

// ── Artifact Card ─────────────────────────────────────────────────────────────

function ArtifactCard({
 artifact,
 onAnnotate,
}: {
 artifact: AgentArtifact;
 onAnnotate?: (id: string, text: string) => void;
}) {
 const [expanded, setExpanded] = useState(true);
 const [annotationInput, setAnnotationInput] = useState("");
 const [showAnnotationForm, setShowAnnotationForm] = useState(false);

 const pendingAnnotations = artifact.annotations.filter((a) => !a.applied);

 function submitAnnotation() {
 if (!annotationInput.trim() || !onAnnotate) return;
 onAnnotate(artifact.id, annotationInput.trim());
 setAnnotationInput("");
 setShowAnnotationForm(false);
 }

 return (
 <div style={{
 border: "1px solid var(--border-color)",
 borderRadius: "6px",
 marginBottom: "8px",
 background: "var(--bg-secondary)",
 overflow: "hidden",
 }}>
 {/* Header */}
 <div
 onClick={() => setExpanded(!expanded)}
 style={{
 display: "flex",
 alignItems: "center",
 padding: "8px 10px",
 cursor: "pointer",
 gap: "8px",
 background: "var(--bg-secondary)",
 userSelect: "none",
 }}
 >
 <span style={{ fontSize: "14px" }}>{artifactIcon(artifact.artifact)}</span>
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
 {artifactLabel(artifact.artifact)}
 </div>
 <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
 Step {artifact.step}
 {pendingAnnotations.length > 0 && (
 <span style={{ marginLeft: "8px", color: "var(--warning-color, #fa0)" }}>
 {pendingAnnotations.length} pending note{pendingAnnotations.length > 1 ? "s" : ""}
 </span>
 )}
 </div>
 </div>
 <span style={{ fontSize: "10px", color: "var(--text-secondary)" }}>{expanded ? "" : "▼"}</span>
 </div>

 {/* Body */}
 {expanded && (
 <div style={{ padding: "8px 10px", borderTop: "1px solid var(--border-color)" }}>
 <ArtifactBody artifact={artifact.artifact} />

 {/* Existing annotations */}
 {artifact.annotations.length > 0 && (
 <div style={{ marginTop: "10px", paddingTop: "8px", borderTop: "1px solid var(--border-color)" }}>
 {artifact.annotations.map((ann, i) => (
 <div key={i} style={{ marginBottom: "4px", display: "flex", gap: "6px", alignItems: "flex-start" }}>
 <span style={{ fontSize: "10px", color: ann.applied ? "var(--success-color, #4c4)" : "var(--warning-color, #fa0)" }}>
 {ann.applied ? "✔" : ""}
 </span>
 <span style={{ fontSize: "11px", color: ann.applied ? "var(--text-secondary)" : "var(--text-primary)", fontStyle: "italic" }}>
 {ann.text}
 </span>
 </div>
 ))}
 </div>
 )}

 {/* Annotation form */}
 {onAnnotate && (
 <div style={{ marginTop: "8px" }}>
 {showAnnotationForm ? (
 <div style={{ display: "flex", gap: "6px" }}>
 <input
 type="text"
 value={annotationInput}
 onChange={(e) => setAnnotationInput(e.target.value)}
 onKeyDown={(e) => { if (e.key === "Enter") submitAnnotation(); if (e.key === "Escape") setShowAnnotationForm(false); }}
 placeholder="Add feedback…"
 autoFocus
 style={{
 flex: 1,
 padding: "4px 7px",
 fontSize: "11px",
 background: "var(--bg-input, var(--bg-primary))",
 border: "1px solid var(--border-color)",
 borderRadius: "3px",
 color: "var(--text-primary)",
 outline: "none",
 }}
 />
 <button onClick={submitAnnotation} style={{ padding: "4px 8px", fontSize: "11px", background: "var(--accent-color, #007acc)", color: "var(--text-primary, #fff)", border: "none", borderRadius: "3px", cursor: "pointer" }}>
 Add
 </button>
 <button onClick={() => setShowAnnotationForm(false)} style={{ padding: "4px 6px", fontSize: "11px", background: "none", border: "1px solid var(--border-color)", borderRadius: "3px", color: "var(--text-secondary)", cursor: "pointer" }}>
 ✕
 </button>
 </div>
 ) : (
 <button
 onClick={() => setShowAnnotationForm(true)}
 style={{ fontSize: "11px", padding: "3px 8px", background: "none", border: "1px dashed var(--border-color)", borderRadius: "3px", color: "var(--text-secondary)", cursor: "pointer" }}
 >
 + Add note
 </button>
 )}
 </div>
 )}
 </div>
 )}
 </div>
 );
}

// ── ArtifactsPanel ────────────────────────────────────────────────────────────

export function ArtifactsPanel({ artifacts, onAnnotate }: ArtifactsPanelProps) {
 if (artifacts.length === 0) {
 return (
 <div style={{ padding: "24px 16px", color: "var(--text-secondary)", fontSize: "13px", textAlign: "center" }}>
 <div style={{ fontSize: "24px", marginBottom: "8px" }}></div>
 <div>No artifacts yet.</div>
 <div style={{ fontSize: "11px", marginTop: "4px", opacity: 0.7 }}>
 Artifacts appear when the agent writes files, runs commands, or produces structured output.
 </div>
 </div>
 );
 }

 return (
 <div style={{ padding: "8px", overflowY: "auto", height: "100%" }}>
 <div style={{ fontSize: "11px", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-secondary)", marginBottom: "8px", padding: "4px 2px" }}>
 {artifacts.length} Artifact{artifacts.length !== 1 ? "s" : ""}
 </div>
 {artifacts.map((artifact) => (
 <ArtifactCard key={artifact.id} artifact={artifact} onAnnotate={onAnnotate} />
 ))}
 </div>
 );
}
