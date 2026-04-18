/**
 * SnippetPanel — Snippet Library & Templates.
 *
 * List/create/delete code snippets (shared with VibeCLI at ~/.vibecli/snippets/).
 * AI-powered snippet generation. Search/filter by language and tags.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SnippetMeta {
  name: string;
  description: string;
  language: string;
  tags: string[];
  created_at: string;
}

interface SnippetPanelProps {
  workspacePath: string | null;
}

const LANG_OPTIONS = ["", "rust", "typescript", "javascript", "python", "go", "java", "ruby", "bash", "sql", "html", "css"];

export function SnippetPanel({ workspacePath: _workspacePath }: SnippetPanelProps) {
  const [snippets, setSnippets] = useState<SnippetMeta[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [search, setSearch] = useState("");
  const [langFilter, setLangFilter] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create form
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newLang, setNewLang] = useState("");
  const [newTags, setNewTags] = useState("");
  const [newContent, setNewContent] = useState("");
  const [generating, setGenerating] = useState(false);
  const [genDesc, setGenDesc] = useState("");

  const loadSnippets = async () => {
    try {
      const result = await invoke<SnippetMeta[]>("list_snippets");
      setSnippets(result);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  useEffect(() => { loadSnippets(); }, []);

  const selectSnippet = async (name: string) => {
    setSelected(name);
    setCreating(false);
    try {
      const c = await invoke<string>("get_snippet", { name });
      setContent(c);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleSave = async () => {
    if (!newName.trim()) { setError("Name is required"); return; }
    setLoading(true);
    setError(null);
    try {
      await invoke("save_snippet", { name: newName.trim(), content: newContent, language: newLang, tags: newTags });
      setCreating(false);
      setNewName(""); setNewLang(""); setNewTags(""); setNewContent("");
      await loadSnippets();
    } catch (e: unknown) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleDelete = async (name: string) => {
    try {
      await invoke("delete_snippet", { name });
      if (selected === name) { setSelected(null); setContent(""); }
      await loadSnippets();
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleGenerate = async () => {
    if (!genDesc.trim()) return;
    setGenerating(true);
    setError(null);
    try {
      const result = await invoke<string>("generate_snippet", { description: genDesc, language: newLang || "typescript" });
      setNewContent(result);
    } catch (e: unknown) {
      setError(String(e));
    }
    setGenerating(false);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(content).catch(() => {});
  };

  const handleInsert = () => {
    window.dispatchEvent(new CustomEvent("vibeui:inject-context", { detail: content }));
  };

  const filtered = snippets.filter((s) => {
    if (search && !s.name.toLowerCase().includes(search.toLowerCase()) && !s.description.toLowerCase().includes(search.toLowerCase())) return false;
    if (langFilter && s.language !== langFilter) return false;
    return true;
  });

  // Extract content without frontmatter for display
  const displayContent = (raw: string) => {
    const lines = raw.split("\n");
    let inFm = false;
    let pastFm = false;
    const out: string[] = [];
    for (const line of lines) {
      if (line.trim() === "---") {
        if (!inFm && !pastFm) { inFm = true; continue; }
        if (inFm) { inFm = false; pastFm = true; continue; }
      }
      if (!inFm) out.push(line);
    }
    return out.join("\n").trim();
  };

  return (
    <div className="panel-container" style={{ flexDirection: "row" }}>
      {/* Left: snippet list */}
      <div style={{
        width: "40%", borderRight: "1px solid var(--border-color)",
        display: "flex", flexDirection: "column", overflow: "hidden",
      }}>
        <div style={{ display: "flex", gap: 4, padding: "8px 8px 4px", flexWrap: "wrap" }}>
          <input
            placeholder="Search..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="panel-input" style={{ flex: 1 }}
          />
          <select value={langFilter} onChange={(e) => setLangFilter(e.target.value)} className="panel-select">
            <option value="">All langs</option>
            {LANG_OPTIONS.filter(Boolean).map((l) => <option key={l} value={l}>{l}</option>)}
          </select>
        </div>
        <div style={{ padding: "4px 8px" }}>
          <button onClick={() => { setCreating(true); setSelected(null); }} className="panel-btn panel-btn-primary" style={{ width: "100%" }}>
            + New Snippet
          </button>
        </div>

        <div style={{ flex: 1, overflowY: "auto" }}>
          {filtered.map((s) => (
            <div role="button" tabIndex={0}
              key={s.name}
              onClick={() => selectSnippet(s.name)}
              style={{
                padding: "8px 8px", cursor: "pointer", fontSize: "var(--font-size-sm)",
                borderBottom: "1px solid var(--border-color)",
                background: selected === s.name ? "color-mix(in srgb, var(--accent-blue) 10%, transparent)" : "transparent",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ fontWeight: 600 }}>{s.name}</span>
                {s.language && (
                  <span style={{
                    padding: "1px 4px", borderRadius: 3, fontSize: 9, fontWeight: 600,
                    background: "var(--accent-color)", color: "var(--bg-tertiary)",
                  }}>
                    {s.language}
                  </span>
                )}
              </div>
              {s.description && (
                <div style={{ fontSize: "var(--font-size-xs)", opacity: 0.6, marginTop: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {s.description}
                </div>
              )}
              {s.tags.length > 0 && (
                <div style={{ display: "flex", gap: 3, marginTop: 2 }}>
                  {s.tags.map((t) => (
                    <span key={t} style={{ fontSize: 9, padding: "0 4px", borderRadius: 2, background: "rgba(203,166,247,0.15)", color: "var(--text-accent)" }}>
                      {t}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
          {filtered.length === 0 && (
            <div className="panel-empty" style={{ fontSize: "var(--font-size-sm)" }}>
              No snippets found.
            </div>
          )}
        </div>
      </div>

      {/* Right: detail or create */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {error && (
          <div className="panel-error">{error}</div>
        )}

        {/* Detail view */}
        {selected && !creating && (
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
            <div style={{ display: "flex", gap: 6, padding: "8px 8px", alignItems: "center", borderBottom: "1px solid var(--border-color)" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{selected}</span>
              <div style={{ flex: 1 }} />
              <button onClick={handleCopy} className="panel-btn panel-btn-secondary">Copy</button>
              <button onClick={handleInsert} className="panel-btn panel-btn-secondary" style={{ color: "var(--text-info)" }}>Insert</button>
              <button onClick={() => handleDelete(selected)} className="panel-btn panel-btn-danger">Delete</button>
            </div>
            <pre style={{
              flex: 1, margin: 0, padding: "8px 12px", overflowY: "auto",
              fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", lineHeight: 1.5,
              background: "var(--bg-primary)", color: "var(--text-primary)",
              whiteSpace: "pre-wrap", wordBreak: "break-all",
            }}>
              {displayContent(content)}
            </pre>
          </div>
        )}

        {/* Create view */}
        {creating && (
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "auto", padding: "8px 12px", gap: 8 }}>
            <div style={{ display: "flex", gap: 6 }}>
              <input placeholder="Snippet name..." value={newName} onChange={(e) => setNewName(e.target.value)} className="panel-input" style={{ flex: 1 }} />
              <select value={newLang} onChange={(e) => setNewLang(e.target.value)} className="panel-select">
                <option value="">Language</option>
                {LANG_OPTIONS.filter(Boolean).map((l) => <option key={l} value={l}>{l}</option>)}
              </select>
            </div>
            <input placeholder="Tags (comma-separated)..." value={newTags} onChange={(e) => setNewTags(e.target.value)} className="panel-input panel-input-full" />

            {/* AI Generate */}
            <div style={{ display: "flex", gap: 6 }}>
              <input placeholder="Describe what to generate..." value={genDesc} onChange={(e) => setGenDesc(e.target.value)} className="panel-input" style={{ flex: 1 }} />
              <button onClick={handleGenerate} disabled={generating} className="panel-btn panel-btn-secondary" style={{ color: "var(--text-info)" }}>
                {generating ? "..." : "AI Generate"}
              </button>
            </div>

            <textarea
              value={newContent}
              onChange={(e) => setNewContent(e.target.value)}
              placeholder="Snippet code..."
              className="panel-input panel-textarea"
              style={{
                flex: 1, minHeight: 120,
                fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", lineHeight: 1.5,
                resize: "vertical",
              }}
            />

            <div style={{ display: "flex", gap: 6 }}>
              <button onClick={handleSave} disabled={loading} className="panel-btn panel-btn-primary" style={{ flex: 1 }}>
                {loading ? "Saving..." : "Save Snippet"}
              </button>
              <button onClick={() => setCreating(false)} className="panel-btn panel-btn-secondary">Cancel</button>
            </div>
          </div>
        )}

        {/* Empty state */}
        {!selected && !creating && (
          <div className="panel-empty">
            Select a snippet or create a new one.
          </div>
        )}
      </div>
    </div>
  );
}

