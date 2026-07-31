import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Sparkles } from "lucide-react";
import { Markdown } from "./Markdown";

interface SkillsViewProps {
  daemonUrl: string;
  onClose: () => void;
  /** Drop a skill's name into the composer so the next task can invoke it. */
  onUse?: (name: string) => void;
}

/** One catalog row from `GET /v1/skilllens/skills`. */
interface SkillRow {
  name: string;
  category: string;
  summary: string;
  source: string;
}

/** `GET /v1/skilllens/skills/:name` — the row plus its markdown body. */
interface SkillDetail extends Partial<SkillRow> {
  body?: string;
}

/**
 * Browser over the daemon's skill catalog. "Skills" was a disabled nav item
 * even though `/v1/skilllens/skills` has been serving the full catalog all
 * along — several hundred skills the user had no way to see.
 *
 * Two panes: a filterable list on the left, the selected skill's markdown on
 * the right. Picking "Use" drops the name into the composer rather than
 * running anything, since a skill is context for a task, not a task itself.
 */
export function SkillsView({ daemonUrl, onClose, onUse }: SkillsViewProps) {
  const [skills, setSkills] = useState<SkillRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [detail, setDetail] = useState<SkillDetail | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const res = await invoke<{ skills: SkillRow[] }>("list_skills", { url: daemonUrl });
        if (alive) setSkills(res.skills ?? []);
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [daemonUrl]);

  useEffect(() => {
    if (!selected) {
      setDetail(null);
      return;
    }
    let alive = true;
    setDetail(null);
    (async () => {
      try {
        const res = await invoke<SkillDetail>("get_skill", { url: daemonUrl, name: selected });
        if (alive) setDetail(res);
      } catch (e) {
        if (alive) setDetail({ body: `Failed to load skill: ${String(e)}` });
      }
    })();
    return () => {
      alive = false;
    };
  }, [daemonUrl, selected]);

  // Grouped by category so a few hundred skills stay navigable.
  const groups = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    const byCat = new Map<string, SkillRow[]>();
    for (const s of skills ?? []) {
      if (needle && !`${s.name} ${s.category} ${s.summary}`.toLowerCase().includes(needle)) continue;
      const cat = s.category || "uncategorized";
      byCat.set(cat, [...(byCat.get(cat) ?? []), s]);
    }
    return [...byCat.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [skills, filter]);

  const shownCount = groups.reduce((n, [, rows]) => n + rows.length, 0);

  return (
    <div className="vx-skills">
      <div className="vx-skills__head">
        <Sparkles size={14} />
        <span>Skills</span>
        {skills && (
          <span className="vx-skills__count">
            {shownCount} of {skills.length}
          </span>
        )}
        <div className="vx-skills__spacer" />
        <button className="vx-icon-btn" aria-label="Close skills" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-skills__body">
        <div className="vx-skills__list-pane">
          <input
            className="vx-files__filter"
            placeholder="Filter skills…"
            value={filter}
            autoFocus
            onChange={(e) => setFilter(e.target.value)}
          />
          <div className="vx-skills__list">
            {error && <div className="vx-files__empty">Failed to load skills: {error}</div>}
            {!error && skills === null && <div className="vx-files__empty">Loading…</div>}
            {!error && skills !== null && shownCount === 0 && (
              <div className="vx-files__empty">
                {skills.length === 0 ? "The daemon reports no skills." : `No skill matches “${filter}”.`}
              </div>
            )}
            {groups.map(([cat, rows]) => (
              <div key={cat}>
                <div className="vx-pill-menu__group">{cat}</div>
                {rows.map((s) => (
                  <button
                    key={s.name}
                    className={`vx-skills__item${selected === s.name ? " is-active" : ""}`}
                    onClick={() => setSelected(s.name)}
                    title={s.summary}
                  >
                    <span className="vx-skills__name">{s.name}</span>
                    {s.source !== "builtin" && <span className="vx-skills__src">{s.source}</span>}
                  </button>
                ))}
              </div>
            ))}
          </div>
        </div>

        <div className="vx-skills__detail">
          {!selected && <div className="vx-files__empty">Pick a skill to read it.</div>}
          {selected && (
            <>
              <div className="vx-skills__detail-head">
                <span className="vx-skills__detail-name">{selected}</span>
                {onUse && (
                  <button
                    className="vx-trash__btn"
                    onClick={() => onUse(selected)}
                    title="Reference this skill in the composer"
                  >
                    Use in composer
                  </button>
                )}
              </div>
              {detail === null ? (
                <div className="vx-files__empty">Loading…</div>
              ) : (
                <Markdown text={detail.body || detail.summary || "(no body)"} />
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
