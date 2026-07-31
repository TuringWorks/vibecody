import { ShieldAlert } from "lucide-react";
import type { PendingApproval } from "../hooks/useApprovals";

interface ApprovalPromptProps {
  pending: PendingApproval[];
  onRespond: (requestId: string, approve: boolean) => void;
}

/** `ToolCallArgument` → "About to run a tool with untrusted input". */
function describeSink(sink: string): string {
  switch (sink) {
    case "ToolCallArgument":
      return "About to run a tool with untrusted input";
    case "McpArgument":
      return "About to call an MCP tool with untrusted input";
    case "ShellCommand":
      return "About to run a shell command built from untrusted input";
    default:
      return `Gated action: ${sink}`;
  }
}

/**
 * Blocking approval banner for the daemon's tainted-content gate. Sits directly
 * above the composer, because the run is stopped until it is answered — a
 * dismissible toast would be the wrong shape for something the agent is
 * actively waiting on.
 *
 * Deny is the safe default and is styled as the primary escape: these prompts
 * fire precisely when a value of unclear provenance is about to reach a tool
 * call, so approving is the deliberate act.
 */
export function ApprovalPrompt({ pending, onRespond }: ApprovalPromptProps) {
  if (pending.length === 0) return null;
  const [next, ...rest] = pending;

  return (
    <div className="vx-approval" role="alertdialog" aria-label="Approval required">
      <div className="vx-approval__icon">
        <ShieldAlert size={16} />
      </div>
      <div className="vx-approval__body">
        <div className="vx-approval__head">
          {describeSink(next.sink)}
          {rest.length > 0 && (
            <span className="vx-approval__queue">+{rest.length} more waiting</span>
          )}
        </div>
        <div className="vx-approval__summary" title={next.summary}>
          {next.summary}
        </div>
      </div>
      <div className="vx-approval__actions">
        <button
          className="vx-approval__btn vx-approval__btn--deny"
          onClick={() => onRespond(next.request_id, false)}
        >
          Deny
        </button>
        <button
          className="vx-approval__btn vx-approval__btn--allow"
          onClick={() => onRespond(next.request_id, true)}
        >
          Allow
        </button>
      </div>
    </div>
  );
}
