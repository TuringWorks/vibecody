/**
 * CompanyApprovalsPanel — Approval workflows dashboard.
 *
 * Shows pending and historical approval requests (hire, strategy,
 * budget, task, deploy). Approve/Reject via buttons with ID input.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, X } from "lucide-react";

interface CompanyApprovalsPanelProps {
  workspacePath?: string | null;
}


export function CompanyApprovalsPanel({ workspacePath: _wp }: CompanyApprovalsPanelProps) {
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [approvalId, setApprovalId] = useState("");
  const [decider, setDecider] = useState("admin");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  // Request form
  const [showRequest, setShowRequest] = useState(false);
  const [reqType, setReqType] = useState("hire");
  const [reqSubject, setReqSubject] = useState("");
  const [reqRequester, setReqRequester] = useState("system");
  const [reqReason, setReqReason] = useState("");

  const load = async () => {
    setLoading(true);
    try {
      const status = showAll ? undefined : "pending";
      const out = await invoke<string>("company_approval_list", { status });
      setOutput(out);
    } catch (e) {
      setOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { load(); }, [showAll]);

  const decide = async (decision: "approve" | "reject") => {
    if (!approvalId.trim()) return;
    try {
      const cmd = decision === "approve" ? "company_approval_approve" : "company_approval_reject";
      const out = await invoke<string>(cmd, { id: approvalId.trim(), decidedBy: decider || "admin" });
      setCmdResult(out);
      setApprovalId("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const requestApproval = async () => {
    if (!reqSubject.trim()) return;
    try {
      const out = await invoke<string>("company_approval_request", {
        requestType: reqType,
        subjectId: reqSubject.trim(),
        requesterId: reqRequester || "system",
        reason: reqReason,
      });
      setCmdResult(out);
      setReqSubject("");
      setReqReason("");
      setShowRequest(false);
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ justifyContent: "space-between" }}>
        <h3>Approvals</h3>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <label style={{ fontSize: "var(--font-size-sm)", cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}>
            <input type="checkbox" checked={showAll} onChange={(e) => setShowAll(e.target.checked)} />
            Show all
          </label>
          <button onClick={() => setShowRequest(!showRequest)} className="panel-btn panel-btn-secondary">
            {showRequest ? "Cancel" : "+ Request"}
          </button>
          <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
      </div>
      <div className="panel-body">

      {/* Request form */}
      {showRequest && (
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>New Approval Request</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
            <select value={reqType} onChange={(e) => setReqType(e.target.value)} className="panel-select">
              {["hire", "strategy", "budget", "task", "deploy"].map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
            <input value={reqSubject} onChange={(e) => setReqSubject(e.target.value)} placeholder="Subject ID *" className="panel-input" />
            <input value={reqRequester} onChange={(e) => setReqRequester(e.target.value)} placeholder="Requester" className="panel-input" />
            <input value={reqReason} onChange={(e) => setReqReason(e.target.value)} placeholder="Reason" className="panel-input" />
          </div>
          <button onClick={requestApproval} disabled={!reqSubject.trim()} className="panel-btn panel-btn-primary" style={{ opacity: reqSubject.trim() ? 1 : 0.5 }}>
            Submit Request
          </button>
        </div>
      )}

      {/* Approvals list */}
      <div className="panel-card" style={{ marginBottom: 14, minHeight: 120 }}>
        {loading ? (
          <span className="panel-loading">Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>
            {output || "No pending approvals."}
          </pre>
        )}
      </div>

      {/* Decision form */}
      <div className="panel-label" style={{ marginBottom: 6 }}>DECIDE</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <input
          value={approvalId}
          onChange={(e) => setApprovalId(e.target.value)}
          placeholder="Approval ID"
          className="panel-input"
          style={{ flex: 2 }}
        />
        <input
          value={decider}
          onChange={(e) => setDecider(e.target.value)}
          placeholder="Your name"
          className="panel-input"
          style={{ flex: 1 }}
        />
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
        <button
          onClick={() => decide("approve")}
          disabled={!approvalId.trim()}
          className="panel-btn panel-btn-primary"
          style={{ opacity: approvalId.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
        >
          <Check size={13} strokeWidth={2} style={{ marginRight: 4 }} /> Approve
        </button>
        <button
          onClick={() => decide("reject")}
          disabled={!approvalId.trim()}
          className="panel-btn panel-btn-danger"
          style={{ opacity: approvalId.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
        >
          <X size={13} strokeWidth={2} style={{ marginRight: 4 }} /> Reject
        </button>
      </div>

      {cmdResult && (
        <div className="panel-card" style={{ fontSize: "var(--font-size-base)" }}>
          {cmdResult}
          <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }} aria-label="Dismiss message"><X size={12} /></button>
        </div>
      )}
      </div>
    </div>
  );
}
