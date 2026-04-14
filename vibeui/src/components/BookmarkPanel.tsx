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
  TODO: "var(--accent-color)",
  FIXME: "var(--error-color)",
  HACK: "var(--warning-color)",
  BUG: "var(--error-color)",
  NOTE: "var(--text-secondary)",
  XXX: "var(--accent-purple)",
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
    <div className="panel-container">
      {/* Sub-tabs */}
      <div className="panel-tab-bar" style={{ padding: "8px 12px" }}>
        {(["markers", "bookmarks"] as const).map((t) => (
          <button
            key={t}
            onClick={() => { setTab(t); if (t === "bookmarks") loadBookmarks(); }}
            className={`panel-tab ${tab === t ? "active" : ""}`}
          >
            {t === "markers" ? "Markers" : "Bookmarks"}
          </button>
        ))}
      </div>

      {error && (
        <div style={{ padding: "6px 12px", fontSize: "var(--font-size-sm)", color: "var(--text-danger)", background: "color-mix(in srgb, var(--accent-rose) 5%, transparent)" }}>
          {error}
        </div>
      )}

      {/* Markers tab */}
      {tab === "markers" && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <div style={{ display: "flex", gap: 6, padding: "8px 12px", alignItems: "center", flexWrap: "wrap" }}>
            <button onClick={scanMarkers} disabled={scanning} className="panel-btn panel-btn-secondary">
              {scanning ? "Scanning..." : "Scan"}
            </button>
            <input
              placeholder="Filter by file..."
              value={fileFilter}
              onChange={(e) => setFileFilter(e.target.value)}
              className="panel-input"
            />
            <span style={{ fontSize: "var(--font-size-xs)", opacity: 0.5 }}>{filteredMarkers.length} results</span>
          </div>
          {/* Type filter chips */}
          <div style={{ display: "flex", gap: 4, padding: "0 12px 6px", flexWrap: "wrap" }}>
            {MARKER_TYPES.map((t) => (
              <button
                key={t}
                onClick={() => setTypeFilter(t)}
                style={{
                  padding: "2px 8px", fontSize: "var(--font-size-xs)", fontWeight: 600, borderRadius: 3, cursor: "pointer",
                  border: typeFilter === t ? "1px solid var(--accent)" : "1px solid var(--border-color)",
                  background: typeFilter === t ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "transparent",
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
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <span style={{
                  padding: "1px 5px", borderRadius: 3, fontWeight: 600, fontSize: 9,
                  background: markerColor[m.marker_type] || "var(--text-secondary)", color: "var(--bg-tertiary)",
                  whiteSpace: "nowrap",
                }}>
                  {m.marker_type}
                </span>
                <span style={{ color: "var(--text-info)", whiteSpace: "nowrap" }}>{m.file}:{m.line}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", opacity: 0.7 }}>
                  {m.text}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleAddBookmark(m.file, m.line, m.text); }}
                  style={{ background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-base)", padding: "0 3px", color: "var(--text-warning)", opacity: 0.7 }}
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
              style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", outline: "none", flex: 1 }}
            />
            <button onClick={loadBookmarks} className="panel-btn panel-btn-secondary">Refresh</button>
            <span style={{ fontSize: "var(--font-size-xs)", opacity: 0.5 }}>{bookmarks.length}</span>
          </div>
          <div style={{ flex: 1, overflowY: "auto", padding: "0 12px" }}>
            {bookmarks.length === 0 && (
              <div style={{ padding: 16, opacity: 0.5, textAlign: "center", fontSize: "var(--font-size-base)" }}>
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
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <span style={{ color: "var(--text-info)", whiteSpace: "nowrap" }}>{b.file}:{b.line}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {b.label}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleRemoveBookmark(b.id); }}
                  style={{ background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-base)", padding: "0 3px", color: "var(--text-danger)", opacity: 0.7 }}
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

