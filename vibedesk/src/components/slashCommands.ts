/** Actions a slash command can trigger. Handled by the shell, not the composer. */
export type SlashAction =
  | "new"
  | "review"
  | "files"
  | "skills"
  | "plugins"
  | "terminal"
  | "automations"
  | "search"
  | "settings"
  | "trash"
  | "branch"
  | "stop"
  | "help";

export interface SlashCommand {
  id: SlashAction;
  /** What the user types after the slash. */
  name: string;
  hint: string;
  /** Only offered while a run is in flight. */
  runningOnly?: boolean;
}

/**
 * The composer's slash commands. Every entry maps to something the shell can
 * already do via mouse; the point is that a keyboard-first user shouldn't have
 * to leave the composer to reach it. Deliberately small — a slash menu that
 * lists things the app can't do is worse than none.
 */
export const SLASH_COMMANDS: SlashCommand[] = [
  { id: "new", name: "new", hint: "Start a new chat" },
  { id: "review", name: "review", hint: "Review working-tree changes" },
  { id: "files", name: "files", hint: "Browse project files" },
  { id: "skills", name: "skills", hint: "Browse the skill catalog" },
  { id: "plugins", name: "plugins", hint: "Show enabled plugin components" },
  { id: "terminal", name: "terminal", hint: "Run a command in the project" },
  { id: "automations", name: "automations", hint: "Scheduled prompts the daemon runs" },
  { id: "search", name: "search", hint: "Search chats" },
  { id: "branch", name: "branch", hint: "Toggle worktree isolation for the next run" },
  { id: "stop", name: "stop", hint: "Stop the running task", runningOnly: true },
  { id: "settings", name: "settings", hint: "Open settings" },
  { id: "trash", name: "trash", hint: "Open Trash & Archive" },
  { id: "help", name: "help", hint: "List these commands" },
];

/**
 * The live `/…` token, if the composer holds one.
 *
 * A slash command is only recognised as the entire content of the composer —
 * `/review` is a command, but "see /usr/bin and report back" is a prompt. That
 * keeps paths and URLs in ordinary messages from opening the menu.
 */
export function findSlash(text: string): string | null {
  if (!text.startsWith("/")) return null;
  const rest = text.slice(1);
  return /\s/.test(rest) ? null : rest;
}

/** Commands matching the typed prefix, filtered by whether a run is active. */
export function matchSlash(query: string, running: boolean): SlashCommand[] {
  const needle = query.toLowerCase();
  return SLASH_COMMANDS.filter(
    (c) => (!c.runningOnly || running) && c.name.startsWith(needle)
  );
}

/** The `/help` output, rendered as a system message in the stream. */
export function helpText(): string {
  const rows = SLASH_COMMANDS.map((c) => `- \`/${c.name}\` — ${c.hint}`).join("\n");
  return `**Slash commands**\n\n${rows}\n\nShortcuts: ⌘N new chat · ⌘P search · ⌘, settings · ⌘1–9 jump to chat · Esc close overlay · @ to reference a file.`;
}
