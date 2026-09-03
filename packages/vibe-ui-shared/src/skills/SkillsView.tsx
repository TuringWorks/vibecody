import { useEffect, useMemo, useState } from "react";
import { Check, Sparkles, X } from "lucide-react";
import { Markdown } from "../markdown/Markdown";
import {
  skillPromptSeed,
  type SkillCatalog,
  type SkillDetail,
  type SkillRow,
} from "./catalog";

export interface SkillsViewProps {
  /** Where the rows come from — see `skilllensCatalog` / `proxiedSkillCatalog`. */
  catalog: SkillCatalog;
  /** Omit in a host that renders this as a panel rather than an overlay. */
  onClose?: () => void;
  /**
   * Hand the picked skills to the host's composer. `text` is the shared
   * wording from `skillPromptSeed`; `names` is the same selection unformatted,
   * for a host that wants to record what was picked.
   */
  onUse?: (text: string, names: string[]) => void;
  /** One line under the title saying where "Use" puts the text. */
  hint?: string;
}

/**
 * Browser over the daemon's skill catalogue, shared by all three Tauri shells.
 *
 * `GET /v1/skilllens/skills` has served the full catalogue all along; until
 * this was shared, only VibeDesk rendered it, and VibeAIChat registered the
 * two Tauri commands without ever calling them.
 *
 * Two panes: a filterable list on the left, the previewed skill on the right.
 * Selecting skills only writes to the composer — it does not activate anything
 * daemon-side, because there is nothing to activate: `AgentRequest` has no
 * `skills` field, so a skill reaches a run as prompt text or not at all. A
 * checkbox that implied otherwise would be a claim about the run that nothing
 * enforces.
 */
export function SkillsView({ catalog, onClose, onUse, hint }: SkillsViewProps) {
  const [skills, setSkills] = useState<SkillRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [preview, setPreview] = useState<string | null>(null);
  const [detail, setDetail] = useState<SkillDetail | null>(null);
  const [picked, setPicked] = useState<readonly string[]>([]);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const rows = await catalog.list();
        if (alive) setSkills(rows);
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [catalog]);

  useEffect(() => {
    if (!preview) {
      setDetail(null);
      return;
    }
    let alive = true;
    setDetail(null);
    (async () => {
      try {
        const res = await catalog.get(preview);
        if (alive) setDetail(res);
      } catch (e) {
        if (alive) setDetail({ body: `Failed to load skill: ${String(e)}` });
      }
    })();
    return () => {
      alive = false;
    };
  }, [catalog, preview]);

  // Grouped by category so a thousand-odd skills stay navigable.
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
  const isPicked = (name: string) => picked.includes(name);
  const togglePick = (name: string) =>
    setPicked((prev) => (prev.includes(name) ? prev.filter((n) => n !== name) : [...prev, name]));

  const use = (names: readonly string[]) => {
    if (!onUse || names.length === 0) return;
    const list = [...names];
    onUse(skillPromptSeed(list), list);
    setPicked([]);
  };

  return (
    <div className="vsk">
      <div className="vsk__head">
        <Sparkles size={14} />
        <span className="vsk__title">Skills</span>
        {skills && (
          <span className="vsk__count">
            {shownCount} of {skills.length}
          </span>
        )}
        {hint && <span className="vsk__hint">{hint}</span>}
        <div className="vsk__spacer" />
        {onClose && (
          <button className="vsk__icon-btn" aria-label="Close skills" onClick={onClose}>
            <X size={14} />
          </button>
        )}
      </div>

      <div className="vsk__body">
        <div className="vsk__list-pane">
          <input
            className="vsk__filter"
            placeholder="Filter skills…"
            // Every host mounts this view because the user just asked for the
            // catalogue, so the filter is where they are already headed.
            autoFocus
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <div className="vsk__list">
            {error && <div className="vsk__empty">Failed to load skills: {error}</div>}
            {!error && skills === null && <div className="vsk__empty">Loading…</div>}
            {!error && skills !== null && shownCount === 0 && (
              <div className="vsk__empty">
                {skills.length === 0
                  ? "The daemon reports no skills."
                  : `No skill matches “${filter}”.`}
              </div>
            )}
            {groups.map(([cat, rows]) => (
              <div key={cat}>
                <div className="vsk__group">{cat}</div>
                {rows.map((s) => (
                  <div
                    key={s.name}
                    className={`vsk__row${preview === s.name ? " is-active" : ""}`}
                  >
                    {onUse && (
                      <button
                        className={`vsk__pick${isPicked(s.name) ? " is-picked" : ""}`}
                        role="checkbox"
                        aria-checked={isPicked(s.name)}
                        aria-label={`Select ${s.name}`}
                        onClick={() => togglePick(s.name)}
                      >
                        {isPicked(s.name) && <Check size={11} />}
                      </button>
                    )}
                    <button
                      className="vsk__item"
                      onClick={() => setPreview(s.name)}
                      title={s.summary}
                    >
                      <span className="vsk__name">{s.name}</span>
                      {s.source !== "builtin" && <span className="vsk__src">{s.source}</span>}
                    </button>
                  </div>
                ))}
              </div>
            ))}
          </div>
          {onUse && picked.length > 0 && (
            <div className="vsk__tray">
              <span className="vsk__tray-count">
                {picked.length} selected
              </span>
              <button className="vsk__btn" onClick={() => setPicked([])}>
                Clear
              </button>
              <button className="vsk__btn is-primary" onClick={() => use(picked)}>
                Use {picked.length === 1 ? "skill" : `${picked.length} skills`}
              </button>
            </div>
          )}
        </div>

        <div className="vsk__detail">
          {!preview && <div className="vsk__empty">Pick a skill to read it.</div>}
          {preview && (
            <>
              <div className="vsk__detail-head">
                <span className="vsk__detail-name">{preview}</span>
                <div className="vsk__spacer" />
                {onUse && (
                  <button
                    className="vsk__btn"
                    onClick={() => use([preview])}
                    title="Reference this skill in the composer"
                  >
                    Use in composer
                  </button>
                )}
                {onUse && detail?.body && (
                  <button
                    className="vsk__btn"
                    onClick={() => onUse(detail.body ?? "", [preview])}
                    title="Paste the skill's full text into the composer"
                  >
                    Insert full text
                  </button>
                )}
              </div>
              {detail === null ? (
                <div className="vsk__empty">Loading…</div>
              ) : (
                <>
                  {detail.triggers && detail.triggers.length > 0 && (
                    <div className="vsk__meta">
                      <span className="vsk__meta-label">Triggers</span>
                      {detail.triggers.map((t) => (
                        <span key={t} className="vsk__tag">
                          {t}
                        </span>
                      ))}
                    </div>
                  )}
                  {detail.tools_allowed && detail.tools_allowed.length > 0 && (
                    <div className="vsk__meta">
                      <span className="vsk__meta-label">Tools</span>
                      {detail.tools_allowed.map((t) => (
                        <span key={t} className="vsk__tag">
                          {t}
                        </span>
                      ))}
                    </div>
                  )}
                  <Markdown text={detail.body || detail.summary || "(no body)"} />
                </>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
