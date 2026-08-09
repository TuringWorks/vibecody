import type { ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

interface EnvSectionProps {
  /** Stable id — the key the open/closed state is persisted under. */
  id: string;
  title: string;
  /** Count shown next to the title, as in the reference design. Omitted when
   *  the section has no meaningful tally. */
  count?: number;
  open: boolean;
  onToggle: (id: string) => void;
  /** Right-aligned affordance in the header (e.g. a "browse" button). */
  action?: ReactNode;
  children: ReactNode;
}

/**
 * A collapsible group in the Environment rail.
 *
 * The rail is narrow and always visible, so everything in it competes with the
 * conversation for attention. Folding a section is how the user says "not now"
 * without losing the panel — and the state persists, because re-collapsing the
 * same section on every launch is the kind of small tax that makes a panel feel
 * like it is fighting you.
 */
export function EnvSection({ id, title, count, open, onToggle, action, children }: EnvSectionProps) {
  return (
    <section className="vx-envsec">
      <div className="vx-envsec__bar">
        <button
          className="vx-envsec__header"
          onClick={() => onToggle(id)}
          aria-expanded={open}
          aria-controls={`envsec-${id}`}
        >
          {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
          <span className="vx-envsec__title">{title}</span>
          {count != null && <span className="vx-envsec__count">{count}</span>}
        </button>
        {action}
      </div>
      {open && (
        <div className="vx-envsec__body" id={`envsec-${id}`}>
          {children}
        </div>
      )}
    </section>
  );
}
