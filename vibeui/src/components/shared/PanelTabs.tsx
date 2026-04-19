import { CSSProperties, ReactNode } from "react";

export interface PanelTabsProps<T extends string> {
  tabs: readonly T[];
  value: T;
  onChange: (tab: T) => void;
  /** Optional label → rendered inside each tab. Falls back to the tab id. */
  renderLabel?: (tab: T) => ReactNode;
  className?: string;
  style?: CSSProperties;
  /** Accessible name for the tablist (WCAG 4.1.2). */
  ariaLabel?: string;
}

/**
 * WCAG-compliant tab bar primitive shared across VibeUI panels.
 *
 * Renders:
 *   <div role="tablist" aria-label=…>
 *     <button role="tab" aria-selected={active}>…</button>
 *     …
 *   </div>
 *
 * Keyboard handling deliberately stays on the browser default for `<button>`
 * (Enter / Space activate); panels that need arrow-key navigation can wrap
 * their own tabs. This primitive is the common 80% case.
 */
export function PanelTabs<T extends string>({
  tabs,
  value,
  onChange,
  renderLabel,
  className,
  style,
  ariaLabel,
}: PanelTabsProps<T>) {
  return (
    <div
      className={className ?? "panel-tab-bar"}
      role="tablist"
      aria-label={ariaLabel}
      style={style}
    >
      {tabs.map((tab) => (
        <button
          key={tab}
          role="tab"
          aria-selected={value === tab}
          className={`panel-tab${value === tab ? " active" : ""}`}
          onClick={() => onChange(tab)}
        >
          {renderLabel ? renderLabel(tab) : tab}
        </button>
      ))}
    </div>
  );
}
