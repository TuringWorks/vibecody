import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { BookOpen, Loader2, Plus, Search, Terminal } from "lucide-react";
import type { NotionPage } from "../../types/productivity";
import { ProviderStatusStrip } from "./ProviderStatusStrip";

export function NotionTab() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<NotionPage[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [selected, setSelected] = useState<NotionPage | null>(null);
  const [body, setBody] = useState("");
  const [bodyLoading, setBodyLoading] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState("");
  const [cmdBusy, setCmdBusy] = useState(false);
  const [appendText, setAppendText] = useState("");
  const [appending, setAppending] = useState(false);
  const [appendMsg, setAppendMsg] = useState<string | null>(null);

  async function runSearch() {
    setLoading(true);
    setErr(null);
    try {
      const list = await invoke<NotionPage[]>("productivity_notion_search", { query });
      setResults(list);
    } catch (e) {
      setErr(String(e));
      setResults([]);
    } finally {
      setLoading(false);
    }
  }

  async function openPage(p: NotionPage) {
    setSelected(p);
    setBody("");
    setBodyLoading(true);
    setAppendText("");
    setAppendMsg(null);
    try {
      const text = await invoke<string>("productivity_notion_page", { id: p.id });
      setBody(text);
    } catch (e) {
      setBody(`Error: ${e}`);
    } finally {
      setBodyLoading(false);
    }
  }

  async function appendToPage() {
    if (!selected || !appendText.trim()) return;
    setAppending(true);
    setAppendMsg(null);
    try {
      await invoke("productivity_notion_append", {
        pageId: selected.id,
        text: appendText,
      });
      const appended = appendText;
      setAppendText("");
      setBody((prev) => `${prev}${prev.endsWith("\n") ? "" : "\n"}${appended}\n`);
      setAppendMsg("Appended.");
    } catch (e) {
      setAppendMsg(`Error: ${e}`);
    } finally {
      setAppending(false);
    }
  }

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_productivity_command", {
        args: `notion ${cmd}`,
      });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <ProviderStatusStrip tab="notion" />
      <div
        style={{
          display: "flex",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          alignItems: "center",
        }}
      >
        <Search size={13} color="var(--text-secondary)" />
        <input
          className="panel-input"
          style={{ flex: 1 }}
          placeholder="Search Notion pages…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") runSearch();
          }}
          disabled={loading}
        />
        <button
          className="panel-btn panel-btn-primary"
          onClick={runSearch}
          disabled={loading}
        >
          {loading ? "Searching…" : "Search"}
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setShowAdvanced((s) => !s)}
          title="Advanced: raw /notion commands"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Terminal size={12} />
        </button>
      </div>
      {err && (
        <div
          style={{
            padding: "6px 10px",
            color: "var(--color-error, #d63e3e)",
            background: "var(--bg-secondary)",
            fontSize: "var(--font-size-sm)",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          {err}
        </div>
      )}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <div
          style={{
            width: selected ? "40%" : "100%",
            overflowY: "auto",
            borderRight: selected ? "1px solid var(--border-color)" : undefined,
          }}
        >
          {loading && results.length === 0 ? (
            <div
              style={{
                padding: 20,
                display: "flex",
                alignItems: "center",
                gap: 6,
                color: "var(--text-secondary)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
              Searching…
            </div>
          ) : results.length === 0 ? (
            <div
              style={{
                padding: 20,
                color: "var(--text-secondary)",
                textAlign: "center",
                fontSize: "var(--font-size-sm)",
              }}
            >
              {query ? "No pages found." : "Enter a query and press Search."}
            </div>
          ) : (
            results.map((p) => (
              <button
                key={p.id}
                onClick={() => openPage(p)}
                className={`panel-card panel-card--clickable${selected?.id === p.id ? " active" : ""}`}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "8px 10px",
                  width: "100%",
                  textAlign: "left",
                  background: selected?.id === p.id ? "var(--bg-tertiary)" : "transparent",
                  border: "none",
                  borderBottom: "1px solid var(--border-color)",
                  cursor: "pointer",
                  color: "inherit",
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <span style={{ width: 16, textAlign: "center" }}>
                  {p.icon ?? <BookOpen size={12} color="var(--text-secondary)" />}
                </span>
                <span
                  style={{
                    flex: 1,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {p.title || "Untitled"}
                </span>
                <span
                  style={{
                    color: "var(--text-secondary)",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {p.last_edited?.slice(0, 10)}
                </span>
              </button>
            ))
          )}
        </div>
        {selected && (
          <div style={{ flex: 1, overflowY: "auto", padding: 12 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 8,
                paddingBottom: 8,
                borderBottom: "1px solid var(--border-color)",
              }}
            >
              <strong style={{ flex: 1 }}>{selected.title || "Untitled"}</strong>
              <a
                href={selected.url}
                target="_blank"
                rel="noreferrer"
                className="panel-btn panel-btn-secondary"
                style={{ textDecoration: "none", fontSize: "calc(var(--font-size-sm) - 1px)" }}
              >
                Open in Notion
              </a>
              <button
                className="panel-btn panel-btn-secondary"
                onClick={() => setSelected(null)}
                style={{ fontSize: "calc(var(--font-size-sm) - 1px)" }}
              >
                Close
              </button>
            </div>
            {bodyLoading ? (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  color: "var(--text-secondary)",
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
                Loading page…
              </div>
            ) : (
              <>
                <pre
                  style={{
                    margin: 0,
                    whiteSpace: "pre-wrap",
                    fontFamily: "var(--font-family)",
                    fontSize: "var(--font-size-sm)",
                    lineHeight: 1.5,
                  }}
                >
                  {body || "(empty)"}
                </pre>
                <div
                  style={{
                    marginTop: 12,
                    paddingTop: 10,
                    borderTop: "1px solid var(--border-color)",
                    display: "flex",
                    flexDirection: "column",
                    gap: 6,
                  }}
                >
                  <span
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 5,
                      color: "var(--text-secondary)",
                      fontSize: "var(--font-size-sm)",
                    }}
                  >
                    <Plus size={12} />
                    Append to page
                  </span>
                  <textarea
                    className="panel-input"
                    rows={3}
                    placeholder="Write a note to append…"
                    value={appendText}
                    onChange={(e) => setAppendText(e.target.value)}
                    disabled={appending}
                    style={{ resize: "vertical", fontFamily: "inherit" }}
                  />
                  <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <button
                      className="panel-btn panel-btn-primary"
                      onClick={appendToPage}
                      disabled={appending || !appendText.trim()}
                      style={{ display: "flex", alignItems: "center", gap: 4 }}
                    >
                      {appending ? (
                        <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} />
                      ) : (
                        <Plus size={11} />
                      )}
                      {appending ? "Appending…" : "Append"}
                    </button>
                    {appendMsg && (
                      <span
                        style={{
                          color: appendMsg.startsWith("Error")
                            ? "var(--color-error, #d63e3e)"
                            : "var(--text-secondary)",
                          fontSize: "calc(var(--font-size-sm) - 1px)",
                        }}
                      >
                        {appendMsg}
                      </span>
                    )}
                  </div>
                </div>
              </>
            )}
          </div>
        )}
      </div>
      {showAdvanced && (
        <div
          style={{
            borderTop: "1px solid var(--border-color)",
            padding: 10,
            background: "var(--bg-secondary)",
            display: "flex",
            flexDirection: "column",
            gap: 6,
            maxHeight: "35%",
          }}
        >
          <div style={{ display: "flex", gap: 6 }}>
            <input
              className="panel-input"
              style={{ flex: 1 }}
              placeholder="notion databases | notion search <q> | notion get <page-id>"
              value={cmd}
              onChange={(e) => setCmd(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") runAdvancedCmd();
              }}
              disabled={cmdBusy}
            />
            <button
              className="panel-btn panel-btn-primary"
              onClick={runAdvancedCmd}
              disabled={cmdBusy || !cmd.trim()}
            >
              {cmdBusy ? "Running…" : "Run"}
            </button>
          </div>
          {cmdOutput && (
            <pre
              style={{
                margin: 0,
                padding: 8,
                background: "var(--bg-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-sm)",
                whiteSpace: "pre-wrap",
                overflowY: "auto",
                flex: 1,
              }}
            >
              {cmdOutput}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}
