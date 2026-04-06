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

const btnStyle: React.CSSProperties = {
  fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)",
};

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
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Approvals</span>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <label style={{ fontSize: 11, cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}>
            <input type="checkbox" checked={showAll} onChange={(e) => setShowAll(e.target.checked)} />
            Show all
          </label>
          <button onClick={() => setShowRequest(!showRequest)} style={btnStyle}>
            {showRequest ? "Cancel" : "+ Request"}
          </button>
          <button onClick={load} style={btnStyle}>Refresh</button>
        </div>
      </div>

      {/* Request form */}
      {showRequest && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>New Approval Request</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
            <select value={reqType} onChange={(e) => setReqType(e.target.value)} style={inputStyle}>
              {["hire", "strategy", "budget", "task", "deploy"].map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
            <input value={reqSubject} onChange={(e) => setReqSubject(e.target.value)} placeholder="Subject ID *" style={inputStyle} />
            <input value={reqRequester} onChange={(e) => setReqRequester(e.target.value)} placeholder="Requester" style={inputStyle} />
            <input value={reqReason} onChange={(e) => setReqReason(e.target.value)} placeholder="Reason" style={inputStyle} />
          </div>
          <button onClick={requestApproval} disabled={!reqSubject.trim()} style={{ ...btnStyle, padding: "4px 14px", opacity: reqSubject.trim() ? 1 : 0.5 }}>
            Submit Request
          </button>
        </div>
      )}

      {/* Approvals list */}
      <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 14, minHeight: 120 }}>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>
            {output || "No pending approvals."}
          </pre>
        )}
      </div>

      {/* Decision form */}
      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>DECIDE</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <input
          value={approvalId}
          onChange={(e) => setApprovalId(e.target.value)}
          placeholder="Approval ID"
          style={{ ...inputStyle, flex: 2 }}
        />
        <input
          value={decider}
          onChange={(e) => setDecider(e.target.value)}
          placeholder="Your name"
          style={{ ...inputStyle, flex: 1 }}
        />
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
        <button
          onClick={() => decide("approve")}
          disabled={!approvalId.trim()}
          style={{ ...btnStyle, padding: "5px 14px", border: "1px solid var(--success, #27ae60)", color: "var(--success, #27ae60)", opacity: approvalId.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
        >
          <Check size={13} strokeWidth={2} style={{ marginRight: 4 }} /> Approve
        </button>
        <button
          onClick={() => decide("reject")}
          disabled={!approvalId.trim()}
          style={{ ...btnStyle, padding: "5px 14px", border: "1px solid var(--danger, #e74c3c)", color: "var(--danger, #e74c3c)", opacity: approvalId.trim() ? 1 : 0.5, display: "inline-flex", alignItems: "center" }}
        >
          <X size={13} strokeWidth={2} style={{ marginRight: 4 }} /> Reject
        </button>
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 4, padding: 8, fontSize: 12 }}>
          {cmdResult}
          <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}
    </div>
  );
}
