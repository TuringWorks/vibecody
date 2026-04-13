/**
 * ChatMemoryPanel — collapsible panel showing facts extracted from the
 * current chat tab, with pin / edit / delete controls.
 *
 * Pinned facts are injected into the AI system prompt for every message.
 */

import { useState, useRef } from "react";
import { Icon } from "./Icon";
import type { MemoryFact } from "../hooks/useSessionMemory";

interface ChatMemoryPanelProps {
  facts: MemoryFact[];
  tabId: string;
  onPin: (id: string) => void;
  onUnpin: (id: string) => void;
  onDelete: (id: string) => void;
  onEdit: (id: string, newText: string) => void;
  onAddManual: (text: string) => void;
  /** When provided, renders in dialog mode: no toggle, always expanded, shows close button */
  onClose?: () => void;
}

export function ChatMemoryPanel({
  facts,
  tabId,
  onPin,
  onUnpin,
  onDelete,
  onEdit,
  onAddManual,
  onClose,
}: ChatMemoryPanelProps) {
  const [open, setOpen] = useState(false);
  const isDialog = onClose !== undefined;
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [newText, setNewText] = useState("");
  const editInputRef = useRef<HTMLInputElement>(null);

  const pinnedFacts = facts.filter((f) => f.pinned);
  const sessionFacts = facts.filter((f) => !f.pinned && f.tabId === tabId);
  const total = pinnedFacts.length + sessionFacts.length;

  const startEdit = (fact: MemoryFact) => {
    setEditingId(fact.id);
    setEditText(fact.text);
    setTimeout(() => editInputRef.current?.select(), 0);
  };

  const commitEdit = () => {
    if (editingId && editText.trim()) onEdit(editingId, editText);
    setEditingId(null);
  };

  const handleAddManual = () => {
    if (newText.trim()) {
      onAddManual(newText.trim());
      setNewText("");
    }
  };

  return (
    <div style={{
      borderTop: isDialog ? "none" : "1px solid var(--border-color)",
      flexShrink: 0,
      background: "var(--bg-secondary)",
      display: "flex", flexDirection: "column",
    }}>
      {/* Dialog header (when opened as a dialog) */}
      {isDialog ? (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
          flexShrink: 0,
        }}>
          <span style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>
            Memory
            {total > 0 && (
              <span style={{
                background: pinnedFacts.length > 0 ? "var(--accent-blue, #3b82f6)" : "var(--bg-tertiary)",
                color: pinnedFacts.length > 0 ? "#fff" : "var(--text-secondary)",
                borderRadius: 10, padding: "0 5px", fontSize: 10, lineHeight: "16px",
              }}>
                {total}
              </span>
            )}
          </span>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            {pinnedFacts.length > 0 && (
              <span style={{ fontSize: 10, color: "var(--accent-blue, #3b82f6)" }}>
                {pinnedFacts.length} pinned · in every message
              </span>
            )}
            <button
              onClick={onClose}
              title="Close"
              style={{
                background: "none", border: "none", cursor: "pointer",
                color: "var(--text-secondary)", fontSize: 16, lineHeight: 1,
                padding: "0 2px",
              }}
            >
              ×
            </button>
          </div>
        </div>
      ) : (
        /* Inline toggle header */
        <button
          onClick={() => setOpen((p) => !p)}
          style={{
            width: "100%", display: "flex", alignItems: "center", justifyContent: "space-between",
            padding: "5px 10px", background: "none", border: "none",
            color: total > 0 ? "var(--text-primary)" : "var(--text-secondary)",
            cursor: "pointer", fontSize: 11, textAlign: "left",
          }}
        >
          <span style={{ display: "flex", alignItems: "center", gap: 5 }}>
            <Icon name={open ? "chevron-down" : "chevron-right"} size={10} />
            <span>Memory</span>
            {total > 0 && (
              <span style={{
                background: pinnedFacts.length > 0 ? "var(--accent-blue, #3b82f6)" : "var(--bg-tertiary)",
                color: pinnedFacts.length > 0 ? "#fff" : "var(--text-secondary)",
                borderRadius: 10, padding: "0 5px", fontSize: 10, lineHeight: "16px",
              }}>
                {total}
              </span>
            )}
          </span>
          {pinnedFacts.length > 0 && (
            <span style={{ fontSize: 10, color: "var(--accent-blue, #3b82f6)" }}>
              {pinnedFacts.length} pinned · in every message
            </span>
          )}
        </button>
      )}

      {/* Panel body — always shown in dialog mode, toggled in inline mode */}
      {(isDialog || open) && (
        <div style={{ maxHeight: isDialog ? "calc(100vh - 120px)" : 220, overflowY: "auto", padding: "0 8px 8px" }}>

          {/* Pinned facts */}
          {pinnedFacts.length > 0 && (
            <div style={{ marginBottom: 6 }}>
              <SectionLabel>PINNED — injected into every message</SectionLabel>
              {pinnedFacts.map((f) => (
                <FactRow
                  key={f.id}
                  fact={f}
                  isEditing={editingId === f.id}
                  editText={editText}
                  editInputRef={editInputRef}
                  onStartEdit={startEdit}
                  onEditChange={setEditText}
                  onCommitEdit={commitEdit}
                  onCancelEdit={() => setEditingId(null)}
                  onPin={onPin}
                  onUnpin={onUnpin}
                  onDelete={onDelete}
                  isPinned
                />
              ))}
            </div>
          )}

          {/* Session facts */}
          {sessionFacts.length > 0 && (
            <div style={{ marginBottom: 6 }}>
              {pinnedFacts.length > 0 && <SectionLabel>THIS SESSION</SectionLabel>}
              {sessionFacts.map((f) => (
                <FactRow
                  key={f.id}
                  fact={f}
                  isEditing={editingId === f.id}
                  editText={editText}
                  editInputRef={editInputRef}
                  onStartEdit={startEdit}
                  onEditChange={setEditText}
                  onCommitEdit={commitEdit}
                  onCancelEdit={() => setEditingId(null)}
                  onPin={onPin}
                  onUnpin={onUnpin}
                  onDelete={onDelete}
                  isPinned={false}
                />
              ))}
            </div>
          )}

          {total === 0 && (
            <div style={{ color: "var(--text-secondary)", fontSize: 11, padding: "6px 2px", opacity: 0.7 }}>
              No facts yet. Facts are picked up from AI responses automatically.
            </div>
          )}

          {/* Add manual note */}
          <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
            <input
              value={newText}
              onChange={(e) => setNewText(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleAddManual(); } }}
              placeholder="Add a note to memory..."
              style={{
                flex: 1, background: "var(--bg-primary)", border: "1px solid var(--border-color)",
                color: "var(--text-primary)", borderRadius: 3, padding: "3px 6px",
                fontSize: 11, outline: "none",
              }}
            />
            <button
              onClick={handleAddManual}
              disabled={!newText.trim()}
              style={{
                background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                color: "var(--text-primary)", borderRadius: 3, padding: "3px 8px",
                fontSize: 11, cursor: newText.trim() ? "pointer" : "default",
                opacity: newText.trim() ? 1 : 0.4,
              }}
            >
              Add
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 3, padding: "2px 2px", letterSpacing: "0.05em" }}>
      {children}
    </div>
  );
}

interface FactRowProps {
  fact: MemoryFact;
  isEditing: boolean;
  editText: string;
  editInputRef: React.RefObject<HTMLInputElement | null>;
  onStartEdit: (f: MemoryFact) => void;
  onEditChange: (t: string) => void;
  onCommitEdit: () => void;
  onCancelEdit: () => void;
  onPin: (id: string) => void;
  onUnpin: (id: string) => void;
  onDelete: (id: string) => void;
  isPinned: boolean;
}

function FactRow({
  fact, isEditing, editText, editInputRef,
  onStartEdit, onEditChange, onCommitEdit, onCancelEdit,
  onPin, onUnpin, onDelete, isPinned,
}: FactRowProps) {
  return (
    <div style={{
      display: "flex", alignItems: "flex-start", gap: 4,
      padding: "3px 4px", borderRadius: 3, marginBottom: 2,
      background: isPinned ? "rgba(59,130,246,0.06)" : "transparent",
    }}>
      {isPinned && (
        <span style={{ marginTop: 2, color: "var(--accent-blue, #3b82f6)", flexShrink: 0, display: "flex" }}>
          <Icon name="map-pin" size={10} />
        </span>
      )}

      {isEditing ? (
        <input
          ref={editInputRef}
          value={editText}
          onChange={(e) => onEditChange(e.target.value)}
          onBlur={onCommitEdit}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); onCommitEdit(); }
            if (e.key === "Escape") { e.preventDefault(); onCancelEdit(); }
          }}
          autoFocus
          style={{
            flex: 1, background: "var(--bg-primary)", border: "1px solid var(--accent-blue, #3b82f6)",
            color: "var(--text-primary)", borderRadius: 3, padding: "1px 4px",
            fontSize: 11, outline: "none",
          }}
        />
      ) : (
        <span
          onClick={() => onStartEdit(fact)}
          title="Click to edit"
          style={{
            flex: 1, fontSize: 11, color: "var(--text-primary)", cursor: "text",
            lineHeight: 1.4, wordBreak: "break-word",
          }}
        >
          {fact.text}
        </span>
      )}

      {!isEditing && (
        <div style={{ display: "flex", gap: 2, flexShrink: 0 }}>
          <button
            onClick={() => isPinned ? onUnpin(fact.id) : onPin(fact.id)}
            title={isPinned ? "Unpin" : "Pin to every message"}
            style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: "0 2px", display: "flex", alignItems: "center" }}
          >
            <Icon name={isPinned ? "unlock" : "map-pin"} size={12} />
          </button>
          <button
            onClick={() => onDelete(fact.id)}
            title="Delete"
            style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: "0 2px", display: "flex", alignItems: "center" }}
          >
            <Icon name="x" size={12} />
          </button>
        </div>
      )}
    </div>
  );
}
