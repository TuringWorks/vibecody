import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import { TAB_GROUPS } from "../constants/tabGroups";
import { TAB_META, DEFAULT_TAB_META } from "../constants/tabMeta";
import { Search, ChevronDown, ChevronRight } from "lucide-react";

interface Props {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function GroupedTabBar({ activeTab, onTabChange }: Props) {
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const searchRef = useRef<HTMLInputElement>(null);
  const activeRef = useRef<HTMLButtonElement>(null);

  // Scroll active tab into view on mount
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [activeTab]);

  const filteredGroups = useMemo(() => {
    if (!search.trim()) return TAB_GROUPS;
    const q = search.toLowerCase();
    return TAB_GROUPS.map((g) => ({
      ...g,
      tabs: g.tabs.filter((t) => {
        const meta = TAB_META[t] || DEFAULT_TAB_META;
        return (
          t.includes(q) ||
          meta.label.toLowerCase().includes(q) ||
          g.label.toLowerCase().includes(q)
        );
      }),
    })).filter((g) => g.tabs.length > 0);
  }, [search]);

  const toggleGroup = (label: string) => {
    setCollapsed((prev) => ({ ...prev, [label]: !prev[label] }));
  };

  const isSearching = search.trim().length > 0;

  // Flat list of visible (non-collapsed, filtered) tab ids for arrow-key nav
  const visibleTabs = useMemo(() => {
    return filteredGroups.flatMap((g) =>
      (!isSearching && collapsed[g.label]) ? [] : g.tabs
    );
  }, [filteredGroups, collapsed, isSearching]);

  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const idx = visibleTabs.indexOf(activeTab);
      let next: number | null = null;
      if (e.key === "ArrowDown") next = Math.min(idx + 1, visibleTabs.length - 1);
      else if (e.key === "ArrowUp") next = Math.max(idx - 1, 0);
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = visibleTabs.length - 1;
      if (next !== null && next !== idx) {
        e.preventDefault();
        onTabChange(visibleTabs[next]);
      }
    },
    [activeTab, visibleTabs, onTabChange],
  );

  return (
    <div className="grouped-tab-bar">
      {/* Search */}
      <div className="tab-search">
        <Search size={12} strokeWidth={1.5} />
        <input
          ref={searchRef}
          type="text"
          placeholder="Filter panels..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setSearch("");
              searchRef.current?.blur();
              activeRef.current?.focus();
            }
          }}
          aria-label="Filter AI panels"
        />
        {search && (
          <button
            className="tab-search-clear"
            onClick={() => { setSearch(""); searchRef.current?.focus(); }}
            aria-label="Clear search"
          >
            &times;
          </button>
        )}
      </div>

      {/* Groups */}
      <div className="tab-groups-scroll" role="tablist" aria-label="AI Panel tabs">
        {filteredGroups.map((group) => {
          const isCollapsed = !isSearching && collapsed[group.label];
          return (
            <div key={group.label} className="tab-group">
              <button
                className="tab-group-header"
                onClick={() => toggleGroup(group.label)}
                aria-expanded={!isCollapsed}
              >
                {isCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
                <span>{group.label}</span>
                <span className="tab-group-count">{group.tabs.length}</span>
              </button>
              {!isCollapsed && (
                <div className="tab-group-items">
                  {group.tabs.map((tab) => {
                    const meta = TAB_META[tab] || DEFAULT_TAB_META;
                    const Icon = meta.icon;
                    const isActive = activeTab === tab;
                    return (
                      <button
                        key={tab}
                        ref={isActive ? activeRef : undefined}
                        role="tab"
                        aria-selected={isActive}
                        tabIndex={isActive ? 0 : -1}
                        id={`ai-tab-${tab}`}
                        className={`tab-item${isActive ? " tab-item--active" : ""}`}
                        onClick={() => onTabChange(tab)}
                        onKeyDown={handleTabKeyDown}
                      >
                        <Icon size={14} strokeWidth={1.5} />
                        <span>{meta.label}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
        {filteredGroups.length === 0 && (
          <div className="tab-no-results">No matching panels</div>
        )}
      </div>
    </div>
  );
}
