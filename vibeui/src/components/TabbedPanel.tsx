import { useState, useRef, type ReactNode } from "react";

export interface SubTab {
  id: string;
  label: string;
  content: ReactNode;
}

interface TabbedPanelProps {
  tabs: SubTab[];
  defaultTab?: string;
}

/**
 * Reusable sub-tab panel with keep-alive behavior.
 * Sub-panels are only mounted once visited, then kept alive (hidden) when inactive.
 */
export function TabbedPanel({ tabs, defaultTab }: TabbedPanelProps) {
  const [active, setActive] = useState(defaultTab || tabs[0]?.id || "");
  const visitedRef = useRef<Set<string>>(new Set([defaultTab || tabs[0]?.id || ""]));
  visitedRef.current.add(active);

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
      <div
        style={{
          display: "flex",
          gap: 2,
          borderBottom: "1px solid var(--border-color)",
          background: "var(--bg-secondary)",
          overflowX: "auto",
          flexShrink: 0,
        }}
      >
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setActive(t.id)}
            style={{
              padding: "8px 14px",
              border: "none",
              background: "transparent",
              cursor: "pointer",
              borderBottom:
                active === t.id
                  ? "2px solid var(--accent-color)"
                  : "2px solid transparent",
              color:
                active === t.id
                  ? "var(--accent-color)"
                  : "var(--text-secondary)",
              fontSize: 13,
              fontFamily: "inherit",
              whiteSpace: "nowrap",
            }}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column" }}>
        {tabs.map((t) =>
          visitedRef.current.has(t.id) ? (
            <div
              key={t.id}
              style={{
                display: active === t.id ? "flex" : "none",
                flexDirection: "column",
                flex: 1,
                minHeight: 0,
              }}
            >
              {t.content}
            </div>
          ) : null,
        )}
      </div>
    </div>
  );
}
