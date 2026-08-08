import { useState } from "react";
import { Brain, ChevronDown, ChevronRight } from "lucide-react";

interface ReasoningBlockProps {
  /** Reasoning blocks from one turn, tags already removed. */
  reasoning: string[];
  /** True when the turn was reasoning and nothing else. */
  only?: boolean;
}

/**
 * Model reasoning, collapsed by default.
 *
 * Reasoning is context, not an answer, so it stays folded away — but it is not
 * discarded: when a turn is *only* reasoning (the model narrated an intention
 * and never acted) this block is the sole explanation of what happened, and
 * hiding it entirely leaves the run looking silently stuck.
 */
export function ReasoningBlock({ reasoning, only }: ReasoningBlockProps) {
  const [open, setOpen] = useState(false);
  if (reasoning.length === 0) return null;

  const label = only ? "Thought out loud — no action taken" : "Reasoning";

  return (
    <div className="vx-reasoning">
      <button
        className="vx-reasoning__header"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        <Brain size={13} className="vx-reasoning__icon" />
        <span className="vx-reasoning__label">{label}</span>
        {reasoning.length > 1 && (
          <span className="vx-reasoning__count">{reasoning.length}</span>
        )}
      </button>
      {open && (
        <div className="vx-reasoning__body">
          {reasoning.map((block, i) => (
            <p key={i} className="vx-reasoning__para">
              {block}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}
