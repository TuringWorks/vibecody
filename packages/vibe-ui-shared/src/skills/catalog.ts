/**
 * Skill catalogue access, one shape for every shell.
 *
 * The daemon serves the catalogue at `GET /v1/skilllens/skills` (authenticated,
 * like nearly everything else), but the three Tauri shells reach it through
 * differently-named commands: VibeDesk proxies a caller-supplied daemon URL
 * (`list_skills` / `get_skill`), while VibeCoder and VibeAIChat go through the
 * SkillForge commands that resolve the local daemon themselves
 * (`skilllens_list_skills` / `skilllens_get_skill`).
 *
 * `SkillsView` takes one of these rather than calling `invoke` itself, so the
 * component does not have to know which shell it is rendering in — the same
 * reason `Transcriber` exists in `voice/transcribers.ts`.
 */
import { invoke } from "@tauri-apps/api/core";

/** One row of `GET /v1/skilllens/skills`. */
export interface SkillRow {
  name: string;
  category: string;
  summary: string;
  source: string;
}

/**
 * `GET /v1/skilllens/skills/:name` — the row plus the markdown body and the
 * frontmatter the catalogue row omits.
 *
 * Every field is optional because a detail fetch that half-succeeds should
 * render what it has rather than assert defaults for what it does not: a skill
 * with no `triggers` block and a skill whose triggers failed to parse are
 * different facts, and `[]` would say the first about both.
 */
export interface SkillDetail extends Partial<SkillRow> {
  body?: string;
  triggers?: string[];
  tools_allowed?: string[];
}

/** What `SkillsView` needs from a host shell. */
export interface SkillCatalog {
  list(): Promise<SkillRow[]>;
  get(name: string): Promise<SkillDetail>;
}

/** Narrow an `invoke` result without trusting its shape. */
function rows(value: unknown): SkillRow[] {
  if (typeof value !== "object" || value === null) return [];
  const list = (value as { skills?: unknown }).skills;
  if (!Array.isArray(list)) return [];
  return list.flatMap((r) => {
    if (typeof r !== "object" || r === null) return [];
    const row = r as Record<string, unknown>;
    if (typeof row.name !== "string") return [];
    return [
      {
        name: row.name,
        category: typeof row.category === "string" ? row.category : "",
        summary: typeof row.summary === "string" ? row.summary : "",
        source: typeof row.source === "string" ? row.source : "builtin",
      },
    ];
  });
}

/** Narrow the detail payload, keeping absent fields absent. */
function detail(value: unknown): SkillDetail {
  if (typeof value !== "object" || value === null) return {};
  const d = value as Record<string, unknown>;
  const strings = (v: unknown): string[] | undefined =>
    Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : undefined;
  return {
    name: typeof d.name === "string" ? d.name : undefined,
    category: typeof d.category === "string" ? d.category : undefined,
    summary: typeof d.summary === "string" ? d.summary : undefined,
    source: typeof d.source === "string" ? d.source : undefined,
    body: typeof d.body === "string" ? d.body : undefined,
    triggers: strings(d.triggers),
    tools_allowed: strings(d.tools_allowed),
  };
}

/**
 * VibeCoder and VibeAIChat: the SkillForge commands, which resolve the local
 * daemon's port and bearer token themselves.
 */
export function skilllensCatalog(): SkillCatalog {
  return {
    list: async () => rows(await invoke("skilllens_list_skills")),
    get: async (name: string) => detail(await invoke("skilllens_get_skill", { name })),
  };
}

/**
 * VibeDesk: a proxy against an explicit daemon URL, since that shell can be
 * pointed at a remote daemon. `token` may be omitted — the Rust side falls
 * back to the port-scoped token file and retries once on a 401.
 */
export function proxiedSkillCatalog(daemonUrl: string, token?: string): SkillCatalog {
  return {
    list: async () => rows(await invoke("list_skills", { url: daemonUrl, token })),
    get: async (name: string) =>
      detail(await invoke("get_skill", { url: daemonUrl, name, token })),
  };
}

/**
 * The composer text for a set of picked skills.
 *
 * Lives here rather than in each shell so the three of them ask for a skill in
 * the same words. The name is the catalogue stem, which is what `get_skill`
 * takes, so the sentence is something the agent can act on rather than a label
 * only a human can resolve.
 */
export function skillPromptSeed(names: string[]): string {
  if (names.length === 0) return "";
  const quoted = names.map((n) => `\`${n}\``);
  const list =
    quoted.length === 1
      ? quoted[0]
      : `${quoted.slice(0, -1).join(", ")} and ${quoted[quoted.length - 1]}`;
  const noun = names.length === 1 ? "skill" : "skills";
  return `Load the ${noun} ${list}, then: `;
}
