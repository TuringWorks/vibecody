// Custom React Flow node used by both the designer (type-colored) and the run
// visualizer (status-colored).

import { Handle, Position, type NodeProps } from "@xyflow/react";
import { colorForType } from "./dsl";
import type { FlowNode } from "./graph";

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "var(--success-color)",
  FAILED: "var(--text-danger)",
  TIMED_OUT: "var(--text-danger)",
  TERMINATED: "var(--text-danger)",
  FAILED_WITH_TERMINAL_ERROR: "var(--text-danger)",
  RUNNING: "var(--accent-blue)",
  IN_PROGRESS: "var(--accent-blue)",
  SCHEDULED: "var(--text-secondary)",
  SKIPPED: "var(--text-secondary)",
};

export function TaskNode({ data, selected }: NodeProps<FlowNode>) {
  const accent = data.status
    ? STATUS_COLORS[data.status] ?? colorForType(data.type)
    : colorForType(data.type);
  return (
    <div
      style={{
        display: "flex",
        minWidth: 170,
        maxWidth: 210,
        borderRadius: "var(--radius-sm, 6px)",
        border: `1px solid ${selected ? "var(--accent-color)" : "var(--border-color)"}`,
        background: "var(--bg-secondary)",
        boxShadow: selected ? "0 0 0 2px var(--accent-bg)" : "none",
        overflow: "hidden",
        fontSize: "var(--font-size-sm)",
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: accent }} />
      <div style={{ width: 4, background: accent }} />
      <div style={{ padding: "6px 8px", minWidth: 0, flex: 1 }}>
        <div
          style={{
            color: "var(--text-primary)",
            fontWeight: 600,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {data.label}
        </div>
        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>
          {data.ref}
        </div>
        <div style={{ color: accent, fontSize: "var(--font-size-xs)", marginTop: 2 }}>
          {data.type}
          {data.iterations ? ` ·×${data.iterations}` : ""}
          {data.status ? ` · ${data.status}` : ""}
        </div>
      </div>
      <Handle type="source" position={Position.Bottom} style={{ background: accent }} />
    </div>
  );
}

export const nodeTypes = { task: TaskNode };
