import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Folder } from "lucide-react";
import type { Task } from "../hooks/useTasks";

interface ChatSearchProps {
  tasks: Task[];
  onPick: (id: string) => void;
  onClose: () => void;
}

/** Rank a chat against the query: title matches beat project-path matches. */
function score(task: Task, needle: string): number {
  const title = task.title.toLowerCase();
  const path = task.project_path.toLowerCase();
  if (!needle) return 1;
  if (title.startsWith(needle)) return 4;
  if (title.includes(needle)) return 3;
  if (path.includes(needle)) return 2;
  return 0;
}

/**
 * ⌘P — jump to any chat by name across every project. The left rail lists chats
 * grouped by project, which stops scaling the moment there are more than a
 * screenful; the "Search" nav item advertised this and did nothing. Keyboard
 * driven: type to filter, ↑/↓ to move, Enter to open, Esc to dismiss.
 *
 * (⌘K is deliberately not used — see the VX-013 locked decision.)
 */
export function ChatSearch({ tasks, onPick, onClose }: ChatSearchProps) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  const results = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return tasks
      .map((t) => ({ task: t, s: score(t, needle) }))
      .filter((r) => r.s > 0)
      .sort((a, b) => b.s - a.s || b.task.updated_at - a.task.updated_at)
      .slice(0, 50)
      .map((r) => r.task);
  }, [tasks, query]);

  // A shrinking result list must not leave the cursor pointing past the end.
  useEffect(() => setCursor(0), [query]);

  // Keep the highlighted row in view while arrowing through a long list.
  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>("[data-active='true']")?.scrollIntoView({
      block: "nearest",
    });
  }, [cursor]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setCursor((c) => Math.min(results.length - 1, c + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setCursor((c) => Math.max(0, c - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const picked = results[cursor];
      if (picked) onPick(picked.id);
    }
  }

  return (
    <div className="vx-palette__scrim" onMouseDown={onClose}>
      <div
        className="vx-palette"
        role="dialog"
        aria-label="Search chats"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="vx-palette__input-row">
          <Search size={15} />
          <input
            className="vx-palette__input"
            placeholder="Search chats…"
            value={query}
            autoFocus
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
          />
          <kbd className="vx-palette__kbd">esc</kbd>
        </div>
        <ul className="vx-palette__list" ref={listRef}>
          {results.length === 0 && (
            <li className="vx-palette__empty">
              {tasks.length === 0 ? "No chats yet." : `No chat matches “${query}”.`}
            </li>
          )}
          {results.map((t, i) => (
            <li key={t.id}>
              <button
                className={`vx-palette__item${i === cursor ? " is-active" : ""}`}
                data-active={i === cursor}
                onMouseEnter={() => setCursor(i)}
                onClick={() => onPick(t.id)}
              >
                <span className="vx-palette__title">{t.title}</span>
                <span className="vx-palette__meta">
                  <Folder size={11} />
                  {t.project_path.split("/").filter(Boolean).pop() || "workspace"}
                  <span className={`vx-palette__status vx-palette__status--${t.status}`}>
                    {t.status}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
