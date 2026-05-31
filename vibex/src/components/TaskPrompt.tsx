import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, ArrowUp } from "lucide-react";
import { ApprovalPill, type ApprovalTier } from "./ApprovalPill";
import { ProviderPill } from "./ProviderPill";
import { ReasoningPill, type ReasoningEffort } from "./ReasoningPill";
import { QuickActionDrawer } from "./QuickActionDrawer";

interface TaskPromptProps {
  daemonUrl: string;
  daemonOnline: boolean;
}

/**
 * VX-105 — the composer (Codex screenshots 1, 2, 7). Carries all run controls
 * inline: + quick-action drawer, approval pill, provider pill, reasoning pill,
 * submit. This is the only primary input (P3: conversation is the interface).
 * NOTE: there is intentionally NO Cmd+K inline edit — targeted edits use the
 * ⌘. diffcomplete surface (see pdm/08 §1).
 */
export function TaskPrompt({ daemonUrl, daemonOnline }: TaskPromptProps) {
  const [text, setText] = useState("");
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState<string | undefined>(undefined);
  const [approval, setApproval] = useState<ApprovalTier>("default");
  const [reasoning, setReasoning] = useState<ReasoningEffort>("medium");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  async function submit() {
    const task = text.trim();
    if (!task || submitting || !daemonOnline) return;
    setSubmitting(true);
    try {
      await invoke<string>("start_agent_session", {
        url: daemonUrl,
        task,
        provider,
        model,
        approval,
        reasoning,
      });
      setText("");
    } catch (e) {
      // Surfaced in the stream by VX-105 wiring; log for now.
      console.error("submit failed", e);
    } finally {
      setSubmitting(false);
    }
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  return (
    <div className="vx-composer">
      {drawerOpen && <QuickActionDrawer daemonUrl={daemonUrl} onClose={() => setDrawerOpen(false)} />}
      <textarea
        className="vx-composer__input"
        placeholder="Ask for follow-up changes"
        value={text}
        rows={2}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={onKeyDown}
      />
      <div className="vx-composer__bar">
        <button
          className="vx-icon-btn"
          aria-label="Quick actions"
          title="Quick actions"
          onClick={() => setDrawerOpen((v) => !v)}
        >
          <Plus size={16} />
        </button>
        <ApprovalPill value={approval} onChange={setApproval} />
        <div className="vx-composer__spacer" />
        <ProviderPill
          provider={provider}
          model={model}
          onProvider={setProvider}
          onModel={setModel}
        />
        <ReasoningPill provider={provider} value={reasoning} onChange={setReasoning} />
        <button
          className="vx-composer__submit"
          aria-label="Submit task"
          disabled={!text.trim() || submitting || !daemonOnline}
          onClick={submit}
        >
          <ArrowUp size={16} />
        </button>
      </div>
    </div>
  );
}
