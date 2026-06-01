import { useState } from "react";
import { Plus, ArrowUp } from "lucide-react";
import { ApprovalPill, type ApprovalTier } from "./ApprovalPill";
import { ProviderPill } from "./ProviderPill";
import { ReasoningPill, type ReasoningEffort } from "./ReasoningPill";
import { QuickActionDrawer, type QuickAction } from "./QuickActionDrawer";

/** The composer's submit payload, bubbled up to SessionStream for orchestration. */
export interface ComposerSubmit {
  task: string;
  provider: string;
  model?: string;
  approval: ApprovalTier;
  reasoning: ReasoningEffort;
}

interface TaskPromptProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** True while a run is in flight — disables submit. */
  busy: boolean;
  onSubmit: (payload: ComposerSubmit) => void;
  onQuickAction: (action: QuickAction) => void;
}

/**
 * VX-105 — the composer (Codex screenshots 1, 2, 7). Carries all run controls
 * inline: + quick-action drawer, approval pill, provider pill, reasoning pill,
 * submit. This is the only primary input (P3: conversation is the interface).
 * Orchestration (create task → run agent → link session) lives in the parent
 * SessionStream; this component only gathers input and bubbles it up.
 * NOTE: there is intentionally NO Cmd+K inline edit — targeted edits use the
 * ⌘. diffcomplete surface (see pdm/08 §1).
 */
export function TaskPrompt({ daemonOnline, busy, onSubmit, onQuickAction }: TaskPromptProps) {
  const [text, setText] = useState("");
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState<string | undefined>(undefined);
  const [approval, setApproval] = useState<ApprovalTier>("default");
  const [reasoning, setReasoning] = useState<ReasoningEffort>("medium");
  const [drawerOpen, setDrawerOpen] = useState(false);

  const canSubmit = !!text.trim() && !busy && daemonOnline;

  function submit() {
    if (!canSubmit) return;
    onSubmit({ task: text.trim(), provider, model, approval, reasoning });
    setText("");
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  return (
    <div className="vx-composer">
      {drawerOpen && (
        <QuickActionDrawer
          onAction={(a) => {
            setDrawerOpen(false);
            onQuickAction(a);
          }}
          onClose={() => setDrawerOpen(false)}
        />
      )}
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
          disabled={!canSubmit}
          onClick={submit}
        >
          <ArrowUp size={16} />
        </button>
      </div>
    </div>
  );
}
