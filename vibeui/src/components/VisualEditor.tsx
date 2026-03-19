/**
 * VisualEditor — floating "AI Edit" bar overlay for the BrowserPanel.
 *
 * When the user enables "Visual Edit" mode, this component listens for
 * postMessage events from the injected inspector.js and shows a floating
 * toolbar near the selected element for AI-powered edits.
 */
import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SelectedElement {
  selector: string;
  outerHTML: string;
  tagName: string;
  reactComponent?: string;
  boundingRect: { top: number; left: number; width: number; height: number };
  styles: Record<string, string>;
}

interface VisualEditorProps {
  /** Called when the user submits an AI edit instruction */
  onEdit: (element: SelectedElement, instruction: string) => void;
  workspacePath: string;
  iframeOffset?: { top: number; left: number };
}

const QUICK_ACTIONS = [
  "Make it larger",
  "Change to primary color",
  "Add hover animation",
  "Make responsive",
  "Add border",
];

export function VisualEditor({ onEdit, workspacePath, iframeOffset = { top: 0, left: 0 } }: VisualEditorProps) {
  const [selected, setSelected] = useState<SelectedElement | null>(null);
  const [_hovered, setHovered] = useState<SelectedElement | null>(null);
  const [instruction, setInstruction] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [lastResult, setLastResult] = useState<string>("");

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (!event.data || typeof event.data.type !== "string") return;
      if (event.data.type === "vibe:element-selected") {
        setSelected(event.data.data as SelectedElement);
        setInstruction("");
        setLastResult("");
      } else if (event.data.type === "vibe:element-hovered") {
        setHovered(event.data.data as SelectedElement);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, []);

  const handleEdit = useCallback(async () => {
    if (!selected || !instruction.trim()) return;
    setIsGenerating(true);
    try {
      onEdit(selected, instruction);
      // Optionally call Tauri for AI-powered edit
      const result = await invoke<string>("visual_edit_element", {
        workspacePath,
        selector: selected.selector,
        instruction,
        currentHtml: selected.outerHTML,
        reactComponent: selected.reactComponent ?? null,
      }).catch(() => "");
      setLastResult(result || "Edit applied.");
    } finally {
      setIsGenerating(false);
    }
  }, [selected, instruction, onEdit, workspacePath]);

  if (!selected) {
    return (
      <div className="visual-editor-hint">
        <span>Click an element in the preview to select it</span>
      </div>
    );
  }

  const { boundingRect } = selected;
  const top = iframeOffset.top + boundingRect.top + boundingRect.height + 8;
  const left = Math.max(8, iframeOffset.left + boundingRect.left);
  const elementName = selected.reactComponent ?? `<${selected.tagName}>`;

  return (
    <>
      {/* Floating toolbar */}
      <div
        className="visual-editor-toolbar"
        style={{
          position: "absolute",
          top,
          left,
          zIndex: 9999,
          background: "var(--bg-secondary)",
          border: "1px solid var(--border-color)",
          borderRadius: 8,
          padding: "10px 12px",
          boxShadow: "0 4px 24px rgba(0,0,0,0.4)",
          minWidth: 320,
          maxWidth: 480,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: 8, gap: 6 }}>
          <span style={{ fontSize: 11, opacity: 0.7, fontFamily: "var(--font-mono)" }}>
            {elementName}
          </span>
          <button
            onClick={() => setSelected(null)}
            style={{ marginLeft: "auto", background: "none", border: "none", cursor: "pointer", opacity: 0.6, fontSize: 14 }}
            title="Close"
          >
            ✕
          </button>
        </div>

        {/* Instruction input */}
        <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
          <input
            type="text"
            value={instruction}
            onChange={(e) => setInstruction(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && handleEdit()}
            placeholder="Change this to..."
            style={{
              flex: 1,
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              borderRadius: 4,
              color: "var(--text-primary)",
              padding: "5px 8px",
              fontSize: 13,
            }}
            autoFocus
          />
          <button
            onClick={handleEdit}
            disabled={isGenerating || !instruction.trim()}
            style={{
              background: "var(--accent-color)",
              color: "var(--text-primary)",
              border: "none",
              borderRadius: 4,
              padding: "5px 12px",
              cursor: "pointer",
              fontSize: 13,
              fontWeight: 600,
            }}
          >
            {isGenerating ? "…" : "Edit"}
          </button>
        </div>

        {/* Quick actions */}
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 8 }}>
          {QUICK_ACTIONS.map((action) => (
            <button
              key={action}
              onClick={() => setInstruction(action)}
              style={{
                background: "var(--bg-tertiary)",
                border: "1px solid var(--border-color)",
                borderRadius: 12,
                padding: "2px 8px",
                fontSize: 11,
                cursor: "pointer",
                color: "var(--text-secondary)",
              }}
            >
              {action}
            </button>
          ))}
        </div>

        {/* Computed styles preview */}
        <details style={{ fontSize: 11, opacity: 0.7 }}>
          <summary style={{ cursor: "pointer", marginBottom: 4 }}>Computed styles</summary>
          <div style={{ fontFamily: "var(--font-mono)", display: "grid", gridTemplateColumns: "1fr 1fr", gap: "2px 8px" }}>
            {Object.entries(selected.styles).map(([k, v]) => (
              <React.Fragment key={k}>
                <span style={{ opacity: 0.6 }}>{k}:</span>
                <span>{v}</span>
              </React.Fragment>
            ))}
          </div>
        </details>

        {lastResult && (
          <div style={{ marginTop: 8, fontSize: 12, color: "var(--success-color)", fontFamily: "var(--font-mono)" }}>
            {lastResult}
          </div>
        )}
      </div>
    </>
  );
}
