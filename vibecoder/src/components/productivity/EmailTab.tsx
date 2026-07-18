import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Inbox,
  MailOpen,
  PenSquare,
  RefreshCw,
  Search as SearchIcon,
  Terminal,
  Loader2,
} from "lucide-react";
import type { Email } from "../../types/productivity";
import { EmailList } from "./EmailList";
import { EmailDetail } from "./EmailDetail";
import { EmailComposer } from "./EmailComposer";
import { ProviderStatusStrip } from "./ProviderStatusStrip";

type Filter = "inbox" | "unread" | "search";

interface Props {
  initialEmailId?: string;
  /** Provider from the toolbar dropdown — forwarded to EmailComposer for the
   *  AI-draft-reply feature. When unset, AI Draft is disabled with a hint. */
  provider?: string;
}

export function EmailTab({ initialEmailId, provider }: Props = {}) {
  const [filter, setFilter] = useState<Filter>(initialEmailId ? "unread" : "inbox");
  const [searchTerm, setSearchTerm] = useState("");
  const [emails, setEmails] = useState<Email[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(initialEmailId ?? null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState<string>("");
  const [cmdBusy, setCmdBusy] = useState(false);
  const [composing, setComposing] = useState(false);

  const fetchEmails = useCallback(
    async (f: Filter, q: string) => {
      setLoading(true);
      setErr(null);
      try {
        const query = f === "unread" ? "is:unread" : f === "search" ? q : "";
        const list = await invoke<Email[]>("productivity_email_list", {
          query,
          max: 30,
        });
        setEmails(list);
      } catch (e) {
        setErr(String(e));
        setEmails([]);
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    fetchEmails(filter, "");
    // Only on mount — subsequent changes go through selectFilter / runSearch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function selectFilter(f: Filter) {
    setFilter(f);
    setSelectedId(null);
    if (f !== "search") fetchEmails(f, "");
  }

  function runSearch() {
    if (!searchTerm.trim()) return;
    setFilter("search");
    setSelectedId(null);
    fetchEmails("search", searchTerm.trim());
  }

  const handleArchived = useCallback((id: string) => {
    setEmails((prev) => prev.filter((e) => e.id !== id));
    setSelectedId(null);
  }, []);

  const handleReadChanged = useCallback((id: string, read: boolean) => {
    setEmails((prev) =>
      prev.map((e) => (e.id === id ? { ...e, is_read: read } : e)),
    );
  }, []);

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_email_command", { args: cmd });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        overflow: "hidden",
      }}
    >
      <ProviderStatusStrip tab="email" />

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          flexWrap: "wrap",
        }}
      >
        <button
          className={`panel-btn panel-btn-secondary${filter === "inbox" ? " active" : ""}`}
          onClick={() => selectFilter("inbox")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Inbox size={12} />
          Inbox
        </button>
        <button
          className={`panel-btn panel-btn-secondary${filter === "unread" ? " active" : ""}`}
          onClick={() => selectFilter("unread")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <MailOpen size={12} />
          Unread
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => fetchEmails(filter, searchTerm)}
          disabled={loading}
          title="Refresh"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {loading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <RefreshCw size={12} />
          )}
        </button>
        <button
          className="panel-btn panel-btn-primary"
          onClick={() => setComposing(true)}
          title="Compose a new email"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <PenSquare size={12} />
          Compose
        </button>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            marginLeft: "auto",
          }}
        >
          <SearchIcon size={12} color="var(--text-secondary)" />
          <input
            className="panel-input"
            placeholder="Search mail…"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") runSearch();
            }}
            style={{ minWidth: 180 }}
          />
          <button
            className="panel-btn panel-btn-secondary"
            onClick={() => setShowAdvanced((s) => !s)}
            title="Advanced: run raw /email commands"
            style={{ display: "flex", alignItems: "center", gap: 4 }}
          >
            <Terminal size={12} />
          </button>
        </div>
      </div>

      {err && (
        <div
          style={{
            padding: "6px 10px",
            background: "var(--bg-secondary)",
            color: "var(--color-error, #d63e3e)",
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
            width: selectedId ? "40%" : "100%",
            minWidth: 260,
            borderRight: selectedId ? "1px solid var(--border-color)" : "none",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {loading && emails.length === 0 ? (
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
              Loading messages…
            </div>
          ) : (
            <EmailList
              emails={emails}
              selectedId={selectedId}
              onSelect={setSelectedId}
            />
          )}
        </div>
        {selectedId && (
          <EmailDetail
            key={selectedId}
            id={selectedId}
            onClose={() => setSelectedId(null)}
            onArchived={handleArchived}
            onReadChanged={handleReadChanged}
            provider={provider}
          />
        )}
      </div>

      {composing && (
        <EmailComposer
          onClose={() => setComposing(false)}
          onSent={() => fetchEmails(filter, searchTerm)}
          provider={provider}
        />
      )}

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
              placeholder="unread | inbox | read <id> | send <to> <subject> <body> | search <q> | triage | archive <id>"
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
