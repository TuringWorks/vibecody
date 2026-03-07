/**
 * BookmarkPanel — TODO/FIXME/HACK marker scanner + user bookmarks.
 *
 * Two sub-tabs: "Markers" (auto-scanned code annotations) and
 * "Bookmarks" (user-saved locations). Click any row to jump to file.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CodeMarker {
  file: string;
  line: number;
  marker_type: string;
  text: string;
  context_line: string;
}

interface Bookmark {
  id: string;
  workspace: string;
  file: string;
  line: number;
  label: string;
  created_at: number;
}

interface BookmarkPanelProps {
  workspacePath: string | null;
}

const MARKER_TYPES = ["ALL", "TODO", "FIXME", "HACK", "BUG", "NOTE", "XXX"];

const markerColor: Record<string, string> = {
  TODO: "#89b4fa",
  FIXME: "#f38ba8",
  HACK: "#fab387",
  BUG: "#f38ba8",
  NOTE: "#6c7086",
  XXX: "#cba6f7",
};

export function BookmarkPanel({ workspacePath }: BookmarkPanelProps) {
  const [tab, setTab] = useState<"markers" | "bookmarks">("markers");
  const [markers, setMarkers] = useState<CodeMarker[]>([]);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [typeFilter, setTypeFilter] = useState("ALL");
  const [fileFilter, setFileFilter] = useState("");
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addLabel, setAddLabel] = useState("");

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to scan markers.</p>
      </div>
    );
  }

  const scanMarkers = async () => {
    setScanning(true);
    setError(null);
    try {
      const result = await invoke<CodeMarker[]>("scan_code_markers", { workspace: workspacePath });
      setMarkers(result);
    } catch (e: unknown) {
      setError(String(e));
    }
    setScanning(false);
  };

  const loadBookmarks = async () => {
    try {
      const result = await invoke<Bookmark[]>("get_bookmarks", { workspace: workspacePath });
      setBookmarks(result);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleAddBookmark = async (file: string, line: number, label: string) => {
    try {
      await invoke("add_bookmark", { workspace: workspacePath, file, line, label: label || `${file}:${line}` });
      await loadBookmarks();
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleRemoveBookmark = async (id: string) => {
    try {
      await invoke("remove_bookmark", { workspace: workspacePath, id });
      await loadBookmarks();
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const openFile = (file: string, line?: number) => {
    const fullPath = file.startsWith("/") ? file : `${workspacePath}/${file}`;
    window.dispatchEvent(new CustomEvent("vibeui:open-file", { detail: { path: fullPath, line } }));
  };

  const filteredMarkers = markers.filter((m) => {
    if (typeFilter !== "ALL" && m.marker_type !== typeFilter) return false;
    if (fileFilter && !m.file.toLowerCase().includes(fileFilter.toLowerCase())) return false;
    return true;
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Sub-tabs */}
      <div style={{ display: "flex", gap: 4, padding: "8px 12px", borderBottom: "1px solid var(--border-color)" }}>
        {(["markers", "bookmarks"] as const).map((t) => (
          <button
            key={t}
            onClick={() => { setTab(t); if (t === "bookmarks") loadBookmarks(); }}
            style={{
              padding: "4px 12px", fontSize: 11, fontWeight: 600, borderRadius: 4, cursor: "pointer",
              border: "1px solid var(--border-color)",
              background: tab === t ? "var(--accent, #6366f1)" : "var(--bg-secondary)",
              color: tab === t ? "#fff" : "var(--text-primary)",
            }}
          >
            {t === "markers" ? "Markers" : "Bookmarks"}
          </button>
        ))}
      </div>

      {error && (
        <div style={{ padding: "6px 12px", fontSize: 11, color: "var(--text-danger, #f38ba8)", background: "rgba(243,139,168,0.05)" }}>
          {error}
        </div>
      )}

      {/* Markers tab */}
      {tab === "markers" && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <div style={{ display: "flex", gap: 6, padding: "8px 12px", alignItems: "center", flexWrap: "wrap" }}>
            <button onClick={scanMarkers} disabled={scanning} style={btnStyle}>
              {scanning ? "Scanning..." : "Scan"}
            </button>
            <input
              placeholder="Filter by file..."
              value={fileFilter}
              onChange={(e) => setFileFilter(e.target.value)}
              style={inputStyle}
            />
            <span style={{ fontSize: 10, opacity: 0.5 }}>{filteredMarkers.length} results</span>
          </div>
          {/* Type filter chips */}
          <div style={{ display: "flex", gap: 4, padding: "0 12px 6px", flexWrap: "wrap" }}>
            {MARKER_TYPES.map((t) => (
              <button
                key={t}
                onClick={() => setTypeFilter(t)}
                style={{
                  padding: "2px 8px", fontSize: 10, fontWeight: 600, borderRadius: 3, cursor: "pointer",
                  border: typeFilter === t ? "1px solid var(--accent, #6366f1)" : "1px solid var(--border-color)",
                  background: typeFilter === t ? "rgba(99,102,241,0.2)" : "transparent",
                  color: t === "ALL" ? "var(--text-primary)" : (markerColor[t] || "var(--text-primary)"),
                }}
              >
                {t}
              </button>
            ))}
          </div>
          {/* Results */}
          <div style={{ flex: 1, overflowY: "auto", padding: "0 12px" }}>
            {filteredMarkers.map((m, i) => (
              <div
                key={`${m.file}:${m.line}:${i}`}
                onClick={() => openFile(m.file, m.line)}
                style={{
                  display: "flex", gap: 8, alignItems: "center", padding: "4px 6px",
                  borderBottom: "1px solid var(--border-color)", cursor: "pointer",
                  fontSize: 11,
                }}
              >
                <span style={{
                  padding: "1px 5px", borderRadius: 3, fontWeight: 600, fontSize: 9,
                  background: markerColor[m.marker_type] || "#6c7086", color: "var(--bg-tertiary)",
                  whiteSpace: "nowrap",
                }}>
                  {m.marker_type}
                </span>
                <span style={{ color: "var(--text-info, #89b4fa)", whiteSpace: "nowrap" }}>{m.file}:{m.line}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", opacity: 0.7 }}>
                  {m.text}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleAddBookmark(m.file, m.line, m.text); }}
                  style={{ ...cellBtn, color: "var(--text-warning, #f9e2af)" }}
                  title="Bookmark this"
                >
                  +
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Bookmarks tab */}
      {tab === "bookmarks" && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <div style={{ display: "flex", gap: 6, padding: "8px 12px", alignItems: "center" }}>
            <input
              placeholder="Label for new bookmark..."
              value={addLabel}
              onChange={(e) => setAddLabel(e.target.value)}
              style={{ ...inputStyle, flex: 1 }}
            />
            <button onClick={loadBookmarks} style={btnStyle}>Refresh</button>
            <span style={{ fontSize: 10, opacity: 0.5 }}>{bookmarks.length}</span>
          </div>
          <div style={{ flex: 1, overflowY: "auto", padding: "0 12px" }}>
            {bookmarks.length === 0 && (
              <div style={{ padding: 16, opacity: 0.5, textAlign: "center", fontSize: 12 }}>
                No bookmarks yet. Add from the Markers tab or scan results.
              </div>
            )}
            {bookmarks.map((b) => (
              <div
                key={b.id}
                onClick={() => openFile(b.file, b.line)}
                style={{
                  display: "flex", gap: 8, alignItems: "center", padding: "4px 6px",
                  borderBottom: "1px solid var(--border-color)", cursor: "pointer",
                  fontSize: 11,
                }}
              >
                <span style={{ color: "var(--text-info, #89b4fa)", whiteSpace: "nowrap" }}>{b.file}:{b.line}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {b.label}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleRemoveBookmark(b.id); }}
                  style={{ ...cellBtn, color: "var(--text-danger, #f38ba8)" }}
                  title="Remove bookmark"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "4px 10px", fontSize: 11, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};

const cellBtn: React.CSSProperties = {
  background: "none", border: "none", cursor: "pointer",
  fontSize: 12, padding: "0 3px", color: "var(--text-primary)", opacity: 0.7,
};
