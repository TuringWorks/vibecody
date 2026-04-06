/**
 * CompanyApprovalsPanel — Approval workflows dashboard.
 *
 * Shows pending and historical approval requests (hire, strategy,
 * budget, task, deploy). Supports approve/reject actions.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyApprovalsPanelProps {
  workspacePath?: string | null;
}

export function CompanyApprovalsPanel({ workspacePath: _wp }: CompanyApprovalsPanelProps) {
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [approvalId, setApprovalId] = useState("");
  const [reason, setReason] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const args = showAll ? "approval list" : "approval list --pending";
      const out = await invoke<string>("company_cmd", { args });
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
      const args = `approval decide ${approvalId.trim()} ${decision}${reason ? ` "${reason}"` : ""}`;
      const out = await invoke<string>("company_cmd", { args });
      setCmdResult(out);
      setApprovalId("");
      setReason("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const btnStyle: React.CSSProperties = {
    fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
    background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
  };
  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Approvals</span>
        <div style={{ display: "flex", gap: 8 }}>
          <label style={{ fontSize: 11, cursor: "pointer" }}>
            <input type="checkbox" checked={showAll} onChange={(e) => setShowAll(e.target.checked)} />
            {" "}Show all
          </label>
          <button onClick={load} style={btnStyle}>
            Refresh
          </button>
        </div>
      </div>

      <div style={{
        background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)",
        borderRadius: 6, padding: 12, marginBottom: 16, minHeight: 160,
      }}>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
            {output || "No pending approvals."}
          </pre>
        )}
      </div>

      {/* Decision form */}
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Decide on an Approval</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <input
          value={approvalId}
          onChange={(e) => setApprovalId(e.target.value)}
          placeholder="Approval ID"
          style={{
            flex: 1, fontSize: 12, padding: "4px 8px",
            background: "var(--bg-primary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
            color: "var(--text-primary)",
          }}
        />
        <input
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Reason (optional)"
          style={{
            flex: 1, fontSize: 12, padding: "4px 8px",
            background: "var(--bg-primary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
            color: "var(--text-primary)",
          }}
        />
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <button
          onClick={() => decide("approve")}
          style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer", color: "var(--success)" }}
        >
          ✓ Approve
        </button>
        <button
          onClick={() => decide("reject")}
          style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer", color: "var(--danger, #e74c3c)" }}
        >
          ✗ Reject
        </button>
      </div>

      {cmdResult && (
        <div style={{
          background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)",
          borderRadius: 4, padding: 8, fontSize: 12,
        }}>
          {cmdResult}
        </div>
      )}
    </div>
  );
}
