/**
 * VisualEditOverlay — Click-to-edit overlay for selected DOM elements.
 *
 * Appears over the BrowserPanel when inspect mode is active and an element
 * is selected. Allows inline text editing and style adjustments that
 * are sent to the AI for code generation.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SelectedElement {
  selector: string;
  outerHTML: string;
  tagName: string;
  reactComponent: string | null;
  styles: Record<string, string>;
  parentChain?: string[];
}

interface VisualEditOverlayProps {
  element: SelectedElement;
  onClose: () => void;
  onApply: (editDescription: string) => void;
}

export function VisualEditOverlay({ element, onClose, onApply }: VisualEditOverlayProps) {
  const [editMode, setEditMode] = useState<"text" | "style" | "ai">("text");
  const [textValue, setTextValue] = useState(extractText(element.outerHTML));
  const [aiPrompt, setAiPrompt] = useState("");
  const [styleEdits, setStyleEdits] = useState<Record<string, string>>({});
  const [applying, setApplying] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  const handleApplyText = async () => {
    const original = extractText(element.outerHTML);
    if (textValue === original) return;
    const description = `Change text content of <${element.tagName}> (${element.selector}) from "${original}" to "${textValue}"`;
    setApplying(true);
    try {
      await invoke("visual_edit_element", {
        selector: element.selector,
        editType: "text",
        newValue: textValue,
      });
      setResult("Applied text change");
      onApply(description);
    } catch (e) {
      setResult(`Error: ${e}`);
    }
    setApplying(false);
  };

  const handleApplyStyle = async () => {
    const entries = Object.entries(styleEdits).filter(([, v]) => v.trim());
    if (entries.length === 0) return;
    const description = `Update styles on <${element.tagName}> (${element.selector}): ${entries.map(([k, v]) => `${k}: ${v}`).join("; ")}`;
    setApplying(true);
    try {
      await invoke("visual_edit_element", {
        selector: element.selector,
        editType: "style",
        newValue: JSON.stringify(styleEdits),
      });
      setResult("Applied style changes");
      onApply(description);
    } catch (e) {
      setResult(`Error: ${e}`);
    }
    setApplying(false);
  };

  const handleAiEdit = async () => {
    if (!aiPrompt.trim()) return;
    const description = `AI edit on <${element.tagName}> (${element.selector}): ${aiPrompt}`;
    setApplying(true);
    try {
      await invoke("visual_edit_element", {
        selector: element.selector,
        editType: "ai",
        newValue: aiPrompt,
      });
      setResult("AI edit applied");
      onApply(description);
    } catch (e) {
      setResult(`Error: ${e}`);
    }
    setApplying(false);
  };

  const commonStyles = [
    "color", "background-color", "font-size", "font-weight",
    "padding", "margin", "border-radius", "opacity",
  ];

  return (
    <div style={{
      position: "absolute", bottom: 0, left: 0, right: 0,
      background: "var(--bg-secondary)", borderTop: "2px solid var(--accent-color)",
      padding: "10px 12px", fontSize: "12px",
      maxHeight: "260px", overflowY: "auto", zIndex: 10,
    }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
        <span style={{ fontWeight: 700, color: "var(--accent-primary)" }}>
          Edit &lt;{element.tagName}&gt;
          {element.reactComponent && (
            <span style={{ color: "var(--text-secondary)", marginLeft: "6px", fontSize: "11px" }}>
              &lt;{element.reactComponent}&gt;
            </span>
          )}
        </span>
        <div style={{ display: "flex", gap: "4px" }}>
          {(["text", "style", "ai"] as const).map((m) => (
            <button
              key={m}
              onClick={() => setEditMode(m)}
              style={{
                padding: "2px 8px", fontSize: "10px", fontWeight: 600, borderRadius: "3px",
                border: editMode === m ? "1px solid var(--accent-color)" : "1px solid var(--border-color)",
                background: editMode === m ? "color-mix(in srgb, var(--accent-blue) 15%, transparent)" : "transparent",
                color: "var(--text-primary)", cursor: "pointer",
              }}
            >
              {m === "text" ? "Text" : m === "style" ? "Style" : "AI Edit"}
            </button>
          ))}
          <button onClick={onClose} style={{
            background: "none", border: "none", cursor: "pointer",
            color: "var(--text-secondary)", fontSize: "14px", padding: "0 4px",
          }}>✕</button>
        </div>
      </div>

      {/* Text edit mode */}
      {editMode === "text" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <textarea
            value={textValue}
            onChange={(e) => setTextValue(e.target.value)}
            rows={2}
            style={{
              width: "100%", padding: "6px 8px", fontSize: "12px",
              background: "var(--bg-primary)", border: "1px solid var(--border-color)",
              borderRadius: "4px", color: "var(--text-primary)", outline: "none",
              resize: "vertical", fontFamily: "inherit", boxSizing: "border-box",
            }}
          />
          <button onClick={handleApplyText} disabled={applying} style={applyBtnStyle}>
            {applying ? "Applying..." : "Apply Text Change"}
          </button>
        </div>
      )}

      {/* Style edit mode */}
      {editMode === "style" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          {commonStyles.map((prop) => (
            <div key={prop} style={{ display: "flex", gap: "6px", alignItems: "center" }}>
              <span style={{ width: "120px", fontSize: "11px", color: "var(--text-secondary)" }}>
                {prop}:
              </span>
              <input
                type="text"
                value={styleEdits[prop] || element.styles?.[prop] || ""}
                onChange={(e) => setStyleEdits((prev) => ({ ...prev, [prop]: e.target.value }))}
                placeholder={element.styles?.[prop] || ""}
                style={{
                  flex: 1, padding: "3px 6px", fontSize: "11px",
                  background: "var(--bg-primary)", border: "1px solid var(--border-color)",
                  borderRadius: "3px", color: "var(--text-primary)", outline: "none",
                  fontFamily: "var(--font-mono)",
                }}
              />
            </div>
          ))}
          <button onClick={handleApplyStyle} disabled={applying} style={applyBtnStyle}>
            {applying ? "Applying..." : "Apply Style Changes"}
          </button>
        </div>
      )}

      {/* AI edit mode */}
      {editMode === "ai" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
            Describe the change you want to make to this element:
          </div>
          <textarea
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            rows={2}
            placeholder="e.g., Make this button larger with rounded corners and a gradient background"
            style={{
              width: "100%", padding: "6px 8px", fontSize: "12px",
              background: "var(--bg-primary)", border: "1px solid var(--border-color)",
              borderRadius: "4px", color: "var(--text-primary)", outline: "none",
              resize: "vertical", fontFamily: "inherit", boxSizing: "border-box",
            }}
          />
          <button onClick={handleAiEdit} disabled={applying || !aiPrompt.trim()} style={applyBtnStyle}>
            {applying ? "Generating..." : "Apply with AI"}
          </button>
        </div>
      )}

      {result && (
        <div style={{
          marginTop: "6px", padding: "4px 8px", fontSize: "11px", borderRadius: "3px",
          background: result.startsWith("Error") ? "color-mix(in srgb, var(--accent-rose) 10%, transparent)" : "color-mix(in srgb, var(--accent-green) 10%, transparent)",
          color: result.startsWith("Error") ? "var(--error-color)" : "var(--success-color)",
        }}>
          {result}
        </div>
      )}
    </div>
  );
}

function extractText(html: string): string {
  // Simple text extraction — strip tags
  return html.replace(/<[^>]*>/g, "").trim().slice(0, 200);
}

const applyBtnStyle: React.CSSProperties = {
  padding: "5px 12px", fontSize: "11px", fontWeight: 600,
  background: "var(--accent-primary)", color: "var(--text-primary)", border: "none",
  borderRadius: "4px", cursor: "pointer", alignSelf: "flex-start",
};
