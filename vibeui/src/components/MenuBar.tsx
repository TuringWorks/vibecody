/**
 * MenuBar — Application dropdown menu bar (File, Edit, View, Tools, Help).
 *
 * Renders a horizontal bar of top-level menus. Clicking a menu name opens its
 * dropdown; hovering between menus while one is open switches instantly.
 */
import { useState, useRef, useEffect, useCallback } from "react";

export interface MenuItem {
  label: string;
  shortcut?: string;
  action?: () => void;
  separator?: boolean;
  disabled?: boolean;
}

export interface MenuGroup {
  label: string;
  items: MenuItem[];
}

interface MenuBarProps {
  menus: MenuGroup[];
}

export const MenuBar: React.FC<MenuBarProps> = ({ menus }) => {
  const [openIdx, setOpenIdx] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setOpenIdx(null), []);

  // Close on outside click
  useEffect(() => {
    if (openIdx === null) return;
    const handler = (e: MouseEvent) => {
      if (barRef.current && !barRef.current.contains(e.target as Node)) {
        close();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [openIdx, close]);

  // Close on Escape
  useEffect(() => {
    if (openIdx === null) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [openIdx, close]);

  return (
    <div ref={barRef} style={barStyle}>
      {menus.map((menu, idx) => (
        <div key={menu.label} style={{ position: "relative" }}>
          <button
            style={{
              ...menuBtnStyle,
              background: openIdx === idx ? "var(--bg-hover)" : "transparent",
              color: openIdx === idx ? "var(--text-primary)" : "var(--text-secondary)",
            }}
            onClick={() => setOpenIdx(openIdx === idx ? null : idx)}
            onMouseEnter={() => { if (openIdx !== null) setOpenIdx(idx); }}
          >
            {menu.label}
          </button>

          {openIdx === idx && (
            <div style={dropdownStyle}>
              {menu.items.map((item, i) =>
                item.separator ? (
                  <div key={`sep-${i}`} style={separatorStyle} />
                ) : (
                  <button
                    key={item.label}
                    style={{
                      ...itemStyle,
                      opacity: item.disabled ? 0.4 : 1,
                      cursor: item.disabled ? "default" : "pointer",
                    }}
                    disabled={item.disabled}
                    onClick={() => {
                      if (item.action) {
                        item.action();
                        close();
                      }
                    }}
                    onMouseEnter={(e) => {
                      if (!item.disabled) (e.currentTarget.style.background = "var(--bg-hover)");
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget.style.background = "transparent");
                    }}
                  >
                    <span>{item.label}</span>
                    {item.shortcut && (
                      <span style={shortcutStyle}>{item.shortcut}</span>
                    )}
                  </button>
                )
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

const barStyle: React.CSSProperties & Record<string, unknown> = {
  display: "flex",
  alignItems: "center",
  gap: 0,
  height: "100%",
  position: "relative",
  zIndex: 10000,
  WebkitAppRegion: "no-drag",
};

const menuBtnStyle: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  fontWeight: 500,
  border: "none",
  borderRadius: 4,
  cursor: "pointer",
  color: "var(--text-secondary)",
  background: "transparent",
  whiteSpace: "nowrap",
};

const dropdownStyle: React.CSSProperties = {
  position: "absolute",
  top: "calc(100% + 2px)",
  left: 0,
  minWidth: 220,
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 6,
  boxShadow: "0 8px 24px rgba(0,0,0,0.25)",
  padding: "4px 0",
  zIndex: 9999,
};

const itemStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  width: "100%",
  padding: "6px 12px",
  fontSize: 12,
  border: "none",
  background: "transparent",
  color: "var(--text-primary)",
  cursor: "pointer",
  textAlign: "left",
  borderRadius: 0,
};

const shortcutStyle: React.CSSProperties = {
  fontSize: 11,
  color: "var(--text-muted)",
  fontFamily: "monospace",
  marginLeft: 24,
};

const separatorStyle: React.CSSProperties = {
  height: 1,
  background: "var(--border-color)",
  margin: "4px 8px",
};
