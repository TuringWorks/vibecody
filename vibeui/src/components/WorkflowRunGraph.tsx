// Read-only DAG of a run, nodes colored by (aggregated) task status. Live data
// comes from the polled run in WorkflowsPanel.

import { Background, Controls, MiniMap, ReactFlow } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useMemo } from "react";
import type { WorkflowDef } from "../workflow/dsl";
import { runToGraph } from "../workflow/graph";
import { nodeTypes } from "../workflow/nodes";

export function WorkflowRunGraph({
  def,
  tasks,
}: {
  def: WorkflowDef;
  tasks: Array<{ referenceName: string; status: string }>;
}) {
  const { nodes, edges } = useMemo(() => runToGraph(def, tasks, "TB"), [def, tasks]);
  return (
    <div style={{ height: "100%", minHeight: 260 }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable />
      </ReactFlow>
    </div>
  );
}
