import { useState } from "react";
import { FileText, Pencil, Play, SearchCode, Globe, ChevronRight, ChevronDown } from "lucide-react";

interface ToolUseBlockProps {
  tool: string;
  summary: string;
  detail?: string;
  durationMs?: number;
  /** Track A: edits expose inline accept/reject (VX-203). */
  onAccept?: () => void;
  onReject?: () => void;
}

const TOOL_ICON: Record<string, typeof FileText> = {
  Read: FileText,
  Edit: Pencil,
  Run: Play,
  Search: SearchCode,
  Web: Globe,
};

/**
 * VX-104 — structured, named, collapsible, timestamped tool-use block.
 * Mirrors Codex's inline action rendering + Claude Code's tool-use blocks.
 * Edit blocks carry accept/reject (the Track-A conversation+Review edit path;
 * targeted edits use the ⌘. DiffCompleteModal surface — see pdm/08 §1).
 */
export function ToolUseBlock({ tool, summary, detail, durationMs, onAccept, onReject }: ToolUseBlockProps) {
  const [open, setOpen] = useState(false);
  const Icon = TOOL_ICON[tool] ?? FileText;
  const duration = durationMs != null ? `${(durationMs / 1000).toFixed(0)}s` : null;
  const isEdit = tool === "Edit";

  return (
    <div className="vx-tool">
      <button className="vx-tool__header" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
        {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        <Icon size={14} className="vx-tool__icon" />
        <span className="vx-tool__label">{tool}</span>
        <span className="vx-tool__summary">{summary}</span>
        {duration && <span className="vx-tool__duration">{duration}</span>}
      </button>
      {open && detail && <div className="vx-tool__detail">{detail}</div>}
      {isEdit && (onAccept || onReject) && (
        <div className="vx-tool__actions">
          <button className="panel-btn panel-btn-primary" onClick={onAccept}>Accept</button>
          <button className="panel-btn panel-btn-secondary" onClick={onReject}>Reject</button>
        </div>
      )}
    </div>
  );
}
