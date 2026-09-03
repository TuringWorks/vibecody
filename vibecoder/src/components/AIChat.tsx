import { memo, useRef, useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceInput } from "@vibe/shared/voice/useVoiceInput";
import { useVoiceDuplex } from "@vibe/shared/voice/useVoiceDuplex";
import { parseProviderSelection } from "../hooks/useModelRegistry";
import { DuplexVoiceButton } from "@vibe/shared/voice/DuplexVoiceButton";
import { VoiceTranscript } from "@vibe/shared/voice/VoiceTranscript";
import { VoiceApproval } from "@vibe/shared/voice/VoiceApproval";
import { useVoiceDuplexPreference } from "@vibe/shared/voice/useVoiceDuplexPreference";
import { buildVoiceContext, findReadme, VOICE_CONTEXT_LIMITS } from "@vibe/shared/voice/voiceContext";
import { tauriTranscriber } from "@vibe/shared/voice/transcribers";
import { ComposerDrawer, type ComposerGroup } from "@vibe/shared/composer/ComposerDrawer";
import { useClickAway } from "@vibe/shared/hooks/useClickAway";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useToast } from "../hooks/useToast";
import { useWatchSync, WatchMessage as WatchSyncMessage } from "../hooks/useWatchSync";
import { ContextPicker } from "./ContextPicker";
import { McpAppEmbed, type McpAppPayload } from "./McpAppEmbed";
import { flowContext } from "../utils/FlowContext";
import { getSelectedEffort } from "../utils/effort";
import { openPanelTab } from "../lib/panelDeepLink";
import { Mic, User, Paperclip, X, FileText, Loader2, Download, ZoomIn, AtSign, AudioLines, Plus } from "lucide-react";
// The same Markdown renderer VibeDesk and VibeAIChat use. Chat replies are
// markdown — headings, lists, bold and above all tables — and rendering them
// as a raw string made every structured answer an unreadable wall of pipes and
// asterisks. Shared rather than a third local implementation.
import { Markdown } from "@vibe/shared/markdown/Markdown";
import "@vibe/shared/markdown/markdown.css";
import "@vibe/shared/composer/composer.css";
// The stylesheet for the mic button, the full-duplex control and the live
// caption. VibeDesk and VibeAIChat import it; VibeCoder never did, so every
// one of those controls rendered here as bare unstyled markup — a button with
// no border sitting next to the styled toolbar buttons, and a status dot with
// no colour, which is exactly the state anyone would describe as "the voice UI
// looks right in VibeDesk and wrong here".
import "@vibe/shared/voice/voice.css";
import "./AIChat.css";

// Voice input lives in packages/vibe-ui-shared/src/voice — the same hook
// VibeDesk and VibeAIChat use. It previously existed here as an inline copy
// *and* as an unused src/hooks/useVoiceInput.ts, and the two had already
// drifted (only one reported errors at all).

// ── Types ────────────────────────────────────────────────────────────────────

export interface ToolCallInfo {
  tool: string;
  path?: string;
  status: "running" | "success" | "error";
  output?: string;
  duration_ms?: number;
}

export interface MessageMetrics {
  prompt_tokens?: number;
  completion_tokens?: number;
  provider?: string;
  model?: string;
  latency_ms?: number;
  tokens_per_sec?: number;
}

/** Attachment sent with a chat message. */
export interface ChatAttachment {
  name: string;
  mime_type: string;
  /** Base64-encoded file content (for images/binary). */
  data: string;
  size: number;
  /** Plain text content for text/code files (avoids base64 round-trip). */
  text_content?: string;
  /** Object URL for local preview (images). Not serialized to backend. */
  previewUrl?: string;
}

export interface Message {
 role: "user" | "assistant";
 content: string;
 timestamp?: number;
 thinking?: string;
 toolCalls?: ToolCallInfo[];
 metrics?: MessageMetrics;
 isError?: boolean;
 isRetry?: boolean;
 /** Attachments sent with this message (for display in chat history). */
 attachments?: ChatAttachment[];
 /** True when this message is a synthetic compaction summary of earlier messages. */
 isSummary?: boolean;
 /**
  * True when this message carries the output of tools the backend executed for
  * the preceding assistant turn. Rendered like an assistant message, but sent
  * back to the provider as a *user* turn — see `toBackendMessage`.
  */
 isToolOutput?: boolean;
 /**
  * Assistant content with the tool tags still in place (reasoning removed).
  * `content` is the display form with tags stripped; replaying that back would
  * hide the model's own tool calls from it on the next turn.
  */
 rawContent?: string;
}

// ── Attachment constants (module scope for stable references) ─────────────────
const MAX_ATTACHMENT_SIZE = 20 * 1024 * 1024; // 20 MB
const MAX_ATTACHMENTS = 10;
const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];
const TEXT_MIME_PREFIXES = ["text/", "application/json", "application/xml", "application/javascript"];
const TEXT_EXTS = new Set(["rs","py","js","ts","tsx","jsx","go","java","c","cpp","h","rb","php","swift","kt","scala",
  "sh","bash","sql","yaml","yml","toml","ini","cfg","conf","env","css","scss","less","vue","svelte",
  "md","txt","log","csv","json","xml","html","htm","svg"]);
function isTextFile(mime: string, name: string): boolean {
  if (TEXT_MIME_PREFIXES.some(p => mime.startsWith(p))) return true;
  const ext = name.split(".").pop()?.toLowerCase() || "";
  return TEXT_EXTS.has(ext);
}

interface PendingWrite {
 path: string;
 content: string;
}

interface ChatResponse {
 message: string;
 tool_output: string;
 pending_write?: PendingWrite;
 /** Highest sessions.db row this turn wrote; the watch-sync cursor skips past it. */
 session_msg_id?: number | null;
 /**
  * Why the provider stopped, when it says so. Absent means the provider does
  * not report one -- never "it finished cleanly".
  */
 stop_reason?: StopReason | null;
}

/**
 * Why a provider stopped generating. Mirrors `vibe_ai::StopReason`.
 *
 * `length` is the one that changes behaviour: it is the provider stating the
 * reply is cut short, which is what lets the panel continue the work by itself
 * instead of leaving the user to type "continue".
 */
type StopReason =
  | "natural"
  | "length"
  | "tool_use"
  | "filtered"
  | { other: string };

interface AIChatProps {
 provider: string;
 context?: string;
 fileTree?: string[];
 currentFile?: string | null;
 /** Root of the open workspace, for the voice context block. */
 workspacePath?: string | null;
 onFileAction?: () => void;
 onPendingWrite?: (path: string, content: string) => void;
 /** When set, appends this text to the current input (Cascade flow inject). */
 pendingInput?: string;
 /** Called once after pendingInput is consumed. */
 onPendingInputConsumed?: () => void;
 /** Available provider names for the inline provider selector. */
 availableProviders?: string[];
 /** Callback when the user changes the provider via the inline selector. */
 onProviderChange?: (provider: string) => void;
 /** Controlled messages from parent (for persistence across tab switches). */
 messages?: Message[];
 /** Called when messages change (controlled mode). */
 onMessagesChange?: (msgs: Message[]) => void;
 /**
  * Pinned memory facts formatted as a system-prompt prefix.
  * Injected into every outgoing message's context.
  */
 pinnedMemory?: string;
 /** Stable tab/conversation ID — written to sessions.db so Watch can see it. */
 sessionId?: string;
 /** Human-readable tab title (e.g. "Ember Ridge") used as the session task. */
 sessionTitle?: string;
 /**
  * When true, sendMessage routes through start_agent_task (full multi-step
  * agent loop with planning, tool execution, and approval) instead of the
  * single-turn stream_chat_message path. Phase-1 opt-in flag.
  */
 useAgentLoop?: boolean;
 /** Called when the user toggles the agent-loop switch in the chat header. */
 onUseAgentLoopChange?: (on: boolean) => void;
 /**
  * How much the agent may do without asking. Fixed for the whole run — the
  * backend reads it once when the run starts. Uncontrolled when omitted.
  */
 approvalMode?: ApprovalMode;
 /** Called when the user picks a different approval mode. */
 onApprovalModeChange?: (mode: ApprovalMode) => void;
 /** `/goals` slash command — open the advanced Goals panel. */
 onSwitchToGoals?: () => void;
 /** Show a file in the editor, by absolute path.
  *
  * Used by the spoken path, which is the one that needs it: a typed answer
  * naming a file leaves the user something to click, and "can you open the
  * config" said out loud leaves nothing. Supplying it also declares the
  * capability to the daemon — see `useVoiceDuplex`'s `onOpenFile`. */
 onOpenFile?: (path: string) => void;
}

// ── Approval modes ───────────────────────────────────────────────────────────

/**
 * Wire values for `ApprovalPolicy::from_str` in `vibe-ai/src/agent.rs`. The
 * backend also has `chat-only` (blocks every tool), which is pointless with the
 * agent loop switched on, so it is not offered here.
 */
export type ApprovalMode = "suggest" | "read-only" | "auto-edit" | "full-auto";

export const APPROVAL_MODES: ReadonlyArray<{
  value: ApprovalMode;
  label: string;
  hint: string;
}> = [
  {
    value: "suggest",
    label: "Ask once per session",
    hint: "Ask before the first change, then allow later actions until this chat tab closes.",
  },
  {
    value: "read-only",
    label: "Read-only",
    hint: "Run reads and searches automatically; block writes, shell commands, and sub-agents. Good for reviews and audits.",
  },
  {
    value: "auto-edit",
    label: "Auto-edit",
    hint: "Apply file edits automatically; ask before running shell commands.",
  },
  {
    value: "full-auto",
    label: "Autonomous",
    hint: "Run everything without asking, including shell commands that modify files.",
  },
];

const DEFAULT_APPROVAL_MODE: ApprovalMode = "suggest";

// ── Slash commands ───────────────────────────────────────────────────────────

/**
 * Slash command shape. Two kinds:
 *  - `prefix` (default): selecting the command replaces the input with the prefix string.
 *  - `action`: selecting the command runs a side-effect (e.g. switch tab, open modal)
 *    and clears the input.
 *
 * The `kind` field is optional for backward compatibility — entries without
 * it default to `prefix` semantics.
 */
type SlashCommandAction =
  | "switch-to-goals"
  | "open-skills";

interface SlashCommand {
  command: string;
  label: string;
  description: string;
  /** Text to insert into the input when selected (default behavior). */
  prefix?: string;
  /** When set, the command triggers a side-effect instead of a prefix. */
  action?: SlashCommandAction;
}

const SLASH_COMMANDS: SlashCommand[] = [
  { command: "/fix",      label: "Fix",        description: "Fix errors in the current file",    prefix: "Fix the following errors:\n" },
  { command: "/explain",  label: "Explain",    description: "Explain selected code",             prefix: "Explain the following code in detail:\n" },
  { command: "/test",     label: "Test",       description: "Generate tests",                    prefix: "Generate comprehensive tests for:\n" },
  { command: "/doc",      label: "Doc",        description: "Generate documentation",            prefix: "Generate documentation for:\n" },
  { command: "/refactor", label: "Refactor",   description: "Refactor code",                     prefix: "Refactor the following code for better readability, performance, and maintainability:\n" },
  { command: "/review",   label: "Review",     description: "Code review",                       prefix: "Perform a thorough code review of:\n" },
  { command: "/compact",  label: "Compact",    description: "Summarize conversation",            prefix: "Summarize our conversation so far into key points and action items:\n" },
  { command: "/goal",     label: "Goal",       description: "Start or control one durable goal", prefix: "/goal " },
  { command: "/goals",    label: "Goal history", description: "Open advanced goal history and details", action: "switch-to-goals" },
  // The catalogue is a panel, not a prompt: picking a skill there writes the
  // reference back into this composer. `/skills` is the keyboard route to it,
  // so a user who knows what they want never has to find AI/ML in the rail.
  { command: "/skills",   label: "Skills",     description: "Browse the skill catalogue and pick skills for this task", action: "open-skills" },

  // ── Developer Excellence ──────────────────────────────────────────────────
  // Each of these names its skill file *with the extension*. That is not
  // decoration: `get_skill` takes the catalogue name, and spelling the file out
  // is what lets a user (or a model reading the transcript) go straight to the
  // source of the guidance rather than guessing which of 1,153 skills the
  // command meant. The prefixes also name the exact `vibecli --devex` command,
  // so the answer is measured rather than recalled.
  { command: "/devex",      label: "Devex",       description: "Developer Excellence scorecard for this workspace",
    prefix: "Load the skill `devex-director-operating-system.md`, then measure this workspace: run `vibecli --devex scorecard --path <workspace>` and `vibecli --devex report --path <workspace>`. Report the measured metrics, and report the `unmeasured` block in full — an absent metric is a gap in instrumentation, not a zero. Finish with the three highest-leverage next actions.\n" },
  { command: "/dora",       label: "DORA",        description: "The four keys, with the proxy each was derived from",
    prefix: "Load the skill `dora-metrics-program.md`, then run `vibecli --devex dora --path <workspace> --json`. For each of the four keys report the value, its band, its sample size and the proxy it came from. List every unmeasured key with its reason and its remedy. Do not substitute a plausible default for a missing metric.\n" },
  { command: "/practices",  label: "Practices",   description: "Engineering-practice maturity, with the missing signals named",
    prefix: "Load the skill `engineering-practices-program.md`, then run `vibecli --devex practices --path <workspace> --json`. Report each practice's detected level, the signals that are missing by name, and any detection caveat. Remember the scan caps at level 3 — level 4 is attested by people, so do not infer it.\n" },
  { command: "/onboarding", label: "Onboarding",  description: "Bootstrap readiness and first-time contributors",
    prefix: "Load the skill `developer-onboarding-day-one.md`, then run `vibecli --devex onboarding --path <workspace> --json`. Report bootstrap readiness signal by signal, and state plainly that time-to-first-commit is not derivable from git alone and why.\n" },
  { command: "/space",      label: "SPACE",       description: "The five SPACE dimensions: what this repo answers, and what it cannot",
    prefix: "Load the skill `space-framework-productivity.md`, then run `vibecli --devex space --path <workspace> --json`. Report each of the five dimensions with the measures available and, for the dimensions with none, the system that actually holds their data. Never present Activity on its own, and never produce an aggregate SPACE score — there deliberately is not one.\n" },
  { command: "/devex-plan", label: "Devex plan",  description: "Turn a scorecard into a sequenced improvement plan",
    prefix: "Load the skills `devex-director-operating-system.md` and `engineering-productivity-dashboards.md`. Run `vibecli --devex scorecard --path <workspace> --json`, then produce a sequenced improvement plan: each item with the measured finding it addresses, an owner role, the metric that will confirm it, and the date it will be checked. Order instrumentation gaps before performance work — a team cannot improve a number they cannot see.\n" },
];

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Extract the `@query` fragment at the cursor position, or null if none. */
function getAtQuery(text: string, cursorPos: number): { query: string; start: number } | null {
 const beforeCursor = text.slice(0, cursorPos);
 const match = beforeCursor.match(/(?:^|[\s\n])(@(\S*))$/);
 if (!match) return null;
 const fullMatch = match[1];
 const query = match[2];
 const start = beforeCursor.lastIndexOf(fullMatch);
 return { query, start };
}

function formatTime(ts?: number): string {
 if (!ts) return "";
 const d = new Date(ts);
 return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

type AgentMode = "fast" | "chat" | "planning";

/**
 * Max number of automatic continue turns after a chat response that triggered
 * tool execution. Each completion that emits tool_output spends one turn:
 * the assistant message + tool output is fed back so the model can act on
 * the result without the user typing "continue". Capped to bound runaway
 * loops on misbehaving prompts.
 */
/**
 * Conversation size at which the panel compacts when the model's real budget
 * is unknown — the vendor does not publish it, or the lookup failed.
 *
 * A fallback, not a measurement. It is the constant this panel used for every
 * model, kept unchanged so resolving real budgets only alters behaviour where
 * a real budget was found.
 */
const DEFAULT_COMPACTION_CHARS = 80_000;

/** Last element satisfying `pred`, without copying the array. */
function findLast<T>(items: readonly T[], pred: (item: T) => boolean): T | undefined {
  for (let i = items.length - 1; i >= 0; i -= 1) {
    if (pred(items[i])) return items[i];
  }
  return undefined;
}

/**
 * How often the live stream is published to React while a reply arrives.
 *
 * ~60 Hz: fast enough that the text still reads as typing, slow enough that a
 * fast provider does not spend one full re-render per token. A model slower
 * than this renders on every chunk exactly as it always did — the interval is
 * a ceiling on render frequency, not a delay added to every chunk.
 */
const STREAM_FLUSH_MS = 16;

/**
 * Cap on distinct tool-call rejections reported per turn.
 *
 * A model that emits one malformed tag usually emits many with the same
 * defect. The point is to tell the user why their file is missing, not to
 * transcribe every instance.
 */
const MAX_REPORTED_TOOL_REJECTIONS = 5;

const MAX_AGENT_TURNS = 10;

/**
 * How many times a turn may be resumed because the provider cut it at the
 * output cap.
 *
 * Separate from `MAX_AGENT_TURNS` because it bounds a different thing: that cap
 * limits tool round-trips, this one limits how much of the user's budget a
 * single long answer may spend continuing itself. Bounded rather than unlimited
 * so a model that always stops at the cap cannot loop forever -- when it is
 * reached the panel stops and says so, which is the point at which a human
 * should decide whether more is wanted.
 */
const MAX_TRUNCATION_CONTINUES = 3;

/**
 * Appended to the last tool-result turn so the model knows the loop is still
 * running and what is expected of it next.
 */
const CONTINUE_HINT =
  "\n\nContinue the task using these results. Emit more tool tags if you need "
  + "more information; otherwise give your final answer.";

/**
 * Map a UI message to the shape the backend expects.
 *
 * Tool results are shown as assistant bubbles but must be sent as a **user**
 * turn: a request whose last message is from the assistant makes providers
 * (Ollama/GLM in particular) return an empty completion, which silently ended
 * the auto-continue loop after the first tool round.
 */
function toBackendMessage(
  m: Message,
  isLast: boolean,
): { role: "user" | "assistant"; content: string } {
  if (!m.isToolOutput) return { role: m.role, content: m.rawContent ?? m.content };
  return {
    role: "user",
    // Tool bubbles deliberately show only a compact activity summary. The
    // provider still needs the complete read/build output to finish the task.
    content: `[Tool results]\n${m.rawContent ?? m.content}${isLast ? CONTINUE_HINT : ""}`,
  };
}

/** Map a whole conversation for the backend, flagging the final message. */
function toBackendMessages(messages: Message[]): Array<{ role: "user" | "assistant"; content: string }> {
  return messages.map((m, i) => toBackendMessage(m, i === messages.length - 1));
}

/**
 * Extract reasoning blocks from content. Returns [cleanedContent, thinkingText].
 *
 * Handles both `<thinking>` and the `<think>` spelling used by GLM/Qwen/R1, and
 * the two unbalanced shapes those models emit in practice:
 *  - an orphan `</think>` (the provider consumed the opening tag into its own
 *    reasoning field), where everything before it is reasoning;
 *  - an unclosed `<think>` (stream cut short), where everything after it is.
 */
function extractThinking(content: string): [string, string] {
  const parts: string[] = [];
  // Namespaced spellings (minimax-m3's `<mm:think>`) count too — missing one
  // put `</mm:think>` on screen mid-sentence.
  const NS = String.raw`(?:[A-Za-z][\w.-]*:)?`;
  const blockRegex = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*?)</${NS}think(?:ing)?>`, "g");

  // Only prose is scanned. A fenced or inline code span is content the user
  // asked to see — an answer *about* `<thinking>` tags, or any HTML/XML
  // sample, must survive verbatim. Stripping there silently ate the payload.
  const segments = splitOnCode(content);
  const proseIdx = segments.flatMap((s, i) => (s.isCode ? [] : [i]));

  const cleanedSegments = segments.map((seg) => {
    if (seg.isCode) return seg.text;
    let match: RegExpExecArray | null;
    blockRegex.lastIndex = 0;
    while ((match = blockRegex.exec(seg.text)) !== null) {
      parts.push(match[1].trim());
    }
    return seg.text.replace(blockRegex, "");
  });

  // Unbalanced tags are anchored to the message, not to a segment, so they are
  // resolved against the first/last prose run only. Anchoring them to the whole
  // string would let an orphan `</think>` swallow a code block that precedes it.
  const first = proseIdx[0];
  if (first !== undefined) {
    const orphanClose = cleanedSegments[first].match(
      new RegExp(`^([\\s\\S]*?)</${NS}think(?:ing)?>`),
    );
    if (orphanClose) {
      parts.push(orphanClose[1].trim());
      cleanedSegments[first] = cleanedSegments[first].slice(orphanClose[0].length);
    }
  }

  // An unclosed opening tag means everything after it is reasoning — the rest
  // of that run and every run after it, code spans included. So it is anchored
  // to the *first* prose run that still holds one. Anchored to the last run
  // instead, a `<thinking>` opened before an inline code span was never found
  // at all, and the tag rendered verbatim at the top of the message.
  const openRe = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*)$`);
  const openIdx = proseIdx.find((i) => openRe.test(cleanedSegments[i]));

  if (openIdx === undefined) {
    return [cleanedSegments.join("").trim(), parts.filter(Boolean).join("\n")];
  }

  const opened = cleanedSegments[openIdx].match(openRe);
  parts.push([opened?.[1] ?? "", ...cleanedSegments.slice(openIdx + 1)].join("").trim());
  const kept = cleanedSegments.map((text, i) =>
    i < openIdx ? text : i === openIdx ? text.replace(openRe, "") : "",
  );

  return [kept.join("").trim(), parts.filter(Boolean).join("\n")];
}

/**
 * Split content into alternating prose and code runs.
 *
 * An unterminated fence (the normal state mid-stream) counts as code through to
 * the end — otherwise a half-written block flickers as prose while it streams.
 */
function splitOnCode(content: string): { text: string; isCode: boolean }[] {
  const re = /(```[\s\S]*?```|~~~[\s\S]*?~~~|`[^`\n]*`|```[\s\S]*$|~~~[\s\S]*$)/g;
  const segments: { text: string; isCode: boolean }[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(content)) !== null) {
    if (m.index > last) segments.push({ text: content.slice(last, m.index), isCode: false });
    segments.push({ text: m[0], isCode: true });
    last = m.index + m[0].length;
  }
  if (last < content.length) segments.push({ text: content.slice(last), isCode: false });
  return segments;
}

/**
 * Return `msg` with any reasoning block moved out of `content` and into
 * `thinking`, so it renders as the collapsed grey disclosure rather than inline.
 *
 * A no-op for the common case (already parsed on `chat:complete`, or no tags at
 * all) — the original object is returned, so React's identity checks still hold.
 * Tool-output bubbles are left alone: their content is a verbatim code block.
 */
function withExtractedThinking(msg: Message): Message {
  if (msg.thinking || msg.isToolOutput || !msg.content) return msg;
  if (!/<(?:[A-Za-z][\w.-]*:)?think(?:ing)?>|<\/(?:[A-Za-z][\w.-]*:)?think(?:ing)?>/.test(msg.content)) {
    return msg;
  }
  const [cleaned, thinking] = extractThinking(msg.content);
  if (!thinking) return msg;
  return { ...msg, content: cleaned, thinking };
}

/**
 * Return `msg` with any tool markup still in `content` lifted into `toolCalls`.
 *
 * `chat:complete` already parses the turn it just received, but a message can
 * reach the panel by other roads — session history, the watch-sync poll, an
 * agent event — and those carry the raw text. Parsing here, at the render
 * boundary, is what stops `<tool_call name="write_file">…` from being shown to
 * the user as literal markup whichever road it arrived by.
 */
function withParsedToolCalls(msg: Message): Message {
  if (msg.toolCalls?.length || msg.isToolOutput || !msg.content) return msg;
  if (!/<tool_call\s+name=|<write_file path=|<read_file path=|<list_dir path=|<build[\s/>]|<run[\s/>]/.test(msg.content)) {
    return msg;
  }
  const [content, toolCalls] = parseToolCalls(msg.content);
  if (toolCalls.length === 0) return msg;
  return { ...msg, content, toolCalls, rawContent: msg.rawContent ?? msg.content };
}

/**
 * Comparison key for "is this the same message we already have?".
 *
 * sessions.db stores the raw provider text; the bubble on screen has had its
 * reasoning block and tool XML stripped. Both sides go through the same
 * stripping here so the two forms compare equal.
 */
function dedupKey(content: string): string {
  const [cleaned] = extractThinking(content);
  const [withoutTools] = parseToolCalls(cleaned);
  return withoutTools.replace(/\s+/g, " ").trim();
}

/** Decode the XML entities `render_tool_call` escapes on the Rust side. */
function decodeEntities(s: string): string {
  return s
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&");
}

/** Parse tool XML tags from content into ToolCallInfo[], return cleaned content. */
function parseToolCalls(content: string): [string, ToolCallInfo[]] {
  const tools: ToolCallInfo[] = [];
  let cleaned = content;

  // <write_file path="...">...</write_file>
  cleaned = cleaned.replace(/<write_file path="([^"]+)">([\s\S]*?)<\/write_file>/g, (_m, path) => {
    // The payload is the complete new file, not useful user-facing output.
    tools.push({ tool: "write_file", path, status: "success" });
    return "";
  });

  // <read_file path="..." />
  cleaned = cleaned.replace(/<read_file path="([^"]+)"\s*\/>/g, (_m, path) => {
    tools.push({ tool: "read_file", path, status: "success" });
    return "";
  });

  // <list_dir path="..." />
  cleaned = cleaned.replace(/<list_dir path="([^"]+)"\s*\/>/g, (_m, path) => {
    tools.push({ tool: "list_dir", path, status: "success" });
    return "";
  });

  // Canonical <tool_call name="…"><param>…</param></tool_call> — what a native
  // tool call is transcribed into, and what the chat prompt now asks for. Same
  // cards as the tag dialect below, so a local model calling tools natively
  // renders like every other model instead of leaking raw markup into the text.
  cleaned = cleaned.replace(/<tool_call\s+name="([^"]+)">([\s\S]*?)<\/tool_call>/g, (_m, name, body) => {
    const param = (key: string): string | undefined => {
      const hit = new RegExp(`<${key}>([\\s\\S]*?)</${key}>`).exec(body);
      return hit ? decodeEntities(hit[1].trim()) : undefined;
    };
    switch (name) {
      case "write_file":
        tools.push({ tool: "write_file", path: param("path"), status: "success" });
        return "";
      case "read_file":
      case "list_dir":
      case "list_directory":
        tools.push({ tool: name === "read_file" ? "read_file" : "list_dir", path: param("path"), status: "success" });
        return "";
      case "build":
      case "run":
        tools.push({ tool: name, path: param("command"), status: "success" });
        return "";
      default:
        // A tool this panel has no card for. Still not shown as raw markup —
        // the name alone tells the user what the model reached for.
        tools.push({ tool: name, status: "error" });
        return "";
    }
  });

  // <build /> or <build command="..." />
  cleaned = cleaned.replace(/<build\s+command="([^"]+)"\s*\/>/g, (_m, cmd) => {
    tools.push({ tool: "build", path: cmd, status: "success" });
    return "";
  });
  cleaned = cleaned.replace(/<build\s*\/>/g, () => {
    tools.push({ tool: "build", status: "success" });
    return "";
  });

  // <run /> or <run command="..." />
  cleaned = cleaned.replace(/<run\s+command="([^"]+)"\s*\/>/g, (_m, cmd) => {
    tools.push({ tool: "run", path: cmd, status: "success" });
    return "";
  });
  cleaned = cleaned.replace(/<run\s*\/>/g, () => {
    tools.push({ tool: "run", status: "success" });
    return "";
  });

  return [cleaned.trim(), tools];
}

/**
 * Turn verbose executor output into the small status message shown in chat.
 *
 * A read result contains the complete file because the next model turn needs
 * it. Rendering that same payload made a simple edit look as though the model
 * pasted the entire source file into its answer. The raw value is retained in
 * `Message.rawContent`; this function is presentation-only.
 */
function summarizeToolOutput(output: string): string {
  const reads = new Set<string>();
  const writes = new Set<string>();
  const failures: string[] = [];

  for (const match of output.matchAll(/^Read file '([^']+)':/gm)) reads.add(match[1]);
  for (const match of output.matchAll(/^Wrote file '([^']+)'(?: \(from code block\))?\./gm)) writes.add(match[1]);
  for (const match of output.matchAll(/^\s*•\s+(.+)$/gm)) writes.add(match[1].trim());
  for (const match of output.matchAll(/^(?:Failed|Ignored)[^\n]*/gm)) failures.push(match[0]);

  const lines: string[] = [];
  if (reads.size > 0) lines.push(`Inspected ${[...reads].map((path) => `\`${path}\``).join(", ")}.`);
  if (writes.size > 0) lines.push(`Updated ${[...writes].map((path) => `\`${path}\``).join(", ")}.`);
  lines.push(...failures.slice(0, 3).map((failure) => `- ${failure}`));

  return lines.join("\n") || "Tool step completed. Continuing with the result…";
}

/** Hide tool payloads while they stream, including a write block whose closing
 * tag has not arrived yet. The backend continues accumulating and executing
 * the untouched stream; only the live presentation is filtered. */
function summarizeStreamingContent(content: string): string {
  const [withoutCompletedTools] = parseToolCalls(content);
  const incompleteTool = withoutCompletedTools.search(
    /<(?:tool_call\b|write_file\b|read_file\b|list_dir\b|build\b|run\b)/,
  );
  return (incompleteTool >= 0
    ? withoutCompletedTools.slice(0, incompleteTool)
    : withoutCompletedTools).trim();
}

// ── Tool call icon/label helpers ─────────────────────────────────────────────
// Thin-line SVG icons consistent with the app's dark theme.

const svgProps = { width: 14, height: 14, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };

function ToolIcon({ tool }: { tool: string }) {
  switch (tool) {
    case "write_file": return <svg {...svgProps}><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>;
    case "read_file":  return <svg {...svgProps}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"/><path d="M14 2v6h6"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>;
    case "list_dir":   return <svg {...svgProps}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>;
    case "build":      return <svg {...svgProps}><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>;
    case "run":        return <svg {...svgProps}><polygon points="5 3 19 12 5 21 5 3"/></svg>;
    default:           return <svg {...svgProps}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>;
  }
}

function toolLabel(tool: string, path?: string): string {
  switch (tool) {
    case "write_file": return path ? `Writing ${path.split("/").pop()}` : "Writing file";
    case "read_file":  return path ? `Reading ${path.split("/").pop()}` : "Reading file";
    case "list_dir":   return path ? `Listing ${path}` : "Listing directory";
    case "build":      return path ? `Building: ${path}` : "Building project";
    case "run":        return path ? `Running: ${path}` : "Running application";
    default:           return tool;
  }
}

// Label for the per-turn collapsible "Work" section. Counts tools and sums
// their durations so a glance shows how much work a turn did without having
// to expand it.
function workLabel(thinking: string | undefined, toolCalls?: ToolCallInfo[]): string {
  const n = toolCalls?.length ?? 0;
  const totalMs = (toolCalls ?? []).reduce((s, t) => s + (t.duration_ms ?? 0), 0);
  if (n > 0) {
    const dur = totalMs > 0 ? ` · ${totalMs < 1000 ? `${totalMs}ms` : `${(totalMs / 1000).toFixed(1)}s`}` : "";
    return `Work · ${n} tool${n > 1 ? "s" : ""}${dur}`;
  }
  return thinking ? "Work · thinking" : "Work";
}

function ToolStatusIcon({ status }: { status: "running" | "success" | "error" }) {
  switch (status) {
    case "running": return <svg {...svgProps} className="spin-icon" style={{ opacity: 0.7 }}><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg>;
    case "success": return <svg {...svgProps} stroke="var(--success-color, #4ade80)"><polyline points="20 6 9 17 4 12"/></svg>;
    case "error":   return <svg {...svgProps} stroke="var(--error-color, #f87171)"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>;
  }
}

// ── Content renderer ─────────────────────────────────────────────────────────

interface CodeBlockProps {
  language: string;
  code: string;
  /** Explicit filename from the fence info string — e.g. ```ts src/App.tsx */
  filename?: string;
  onApply?: (code: string, filename: string) => void;
}

/** Number of lines shown when a code block is collapsed. */
const CODE_COLLAPSE_LINES = 8;

/** Language → display-only default filename (never used for Apply). */
const LANG_EXT_MAP: Record<string, string> = {
  typescript: ".ts", javascript: ".js", tsx: ".tsx", jsx: ".jsx",
  rust: ".rs", python: ".py", go: ".go", java: ".java",
  css: ".css", html: ".html", json: ".json", yaml: ".yaml",
  yml: ".yml", toml: ".toml", sql: ".sql", bash: ".sh", sh: ".sh",
  markdown: ".md", md: ".md",
  cpp: ".cpp", c: ".c", ruby: ".rb", swift: ".swift",
  kotlin: ".kt", scala: ".scala", php: ".php",
};

function CodeBlock({ language, code, filename, onApply }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [showLines, setShowLines] = useState(false);
  const lines = code.split("\n");
  const collapsible = lines.length > CODE_COLLAPSE_LINES;
  const [expanded, setExpanded] = useState(!collapsible);

  // "Apply to…" path input — shown when user wants to apply a language-only block
  const [showPathInput, setShowPathInput] = useState(false);
  const [customPath, setCustomPath] = useState(() => {
    // Pre-fill with language extension hint if available
    const ext = LANG_EXT_MAP[language?.toLowerCase() ?? ""];
    return ext ? `file${ext}` : "";
  });

  const handleCopy = () => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  };

  const handleApplyWithPath = () => {
    if (onApply && customPath.trim()) {
      onApply(code, customPath.trim());
      setShowPathInput(false);
    }
  };

  const visibleCode = expanded ? code : lines.slice(0, CODE_COLLAPSE_LINES).join("\n");
  // Display label: use explicit filename if available, else show language extension hint
  const displayLabel = filename ?? (LANG_EXT_MAP[language?.toLowerCase() ?? ""] ? `*${LANG_EXT_MAP[language.toLowerCase()]}` : null);

  return (
    <div className="cb-container">
      <div className="cb-header">
        {collapsible && (
          <button
            className="cb-btn cb-btn-collapse"
            onClick={() => setExpanded((v) => !v)}
            title={expanded ? "Collapse code" : "Expand code"}
          >
            {expanded ? "\u25BE" : "\u25B8"} {lines.length} lines
          </button>
        )}
        <span className="cb-lang">{language || "text"}</span>
        {displayLabel && (
          <span className="cb-filename" title={filename ? "Target file" : "Language default — use Apply to… to specify path"}>
            {displayLabel}
          </span>
        )}
        <div className="cb-actions">
          <button className="cb-btn" onClick={() => setShowLines(!showLines)} title="Toggle line numbers">
            #
          </button>
          <button className="cb-btn" onClick={handleCopy} title="Copy code">
            {copied ? "\u2713" : "Copy"}
          </button>
          {onApply && filename && (
            // Explicit filename from fence — safe to apply directly
            <button
              className="cb-btn cb-btn-apply"
              onClick={() => onApply(code, filename)}
              title={`Apply to ${filename}`}
            >
              Apply
            </button>
          )}
          {onApply && !filename && (
            // No explicit filename — ask user to confirm/enter path before applying
            <button
              className="cb-btn cb-btn-apply"
              onClick={() => setShowPathInput((v) => !v)}
              title="Specify file path to apply to"
            >
              Apply to…
            </button>
          )}
        </div>
      </div>

      {/* Path confirmation row — only shown when "Apply to…" is clicked */}
      {showPathInput && onApply && (
        <div className="cb-path-row">
          <input
            className="cb-path-input"
            value={customPath}
            onChange={(e) => setCustomPath(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleApplyWithPath();
              if (e.key === "Escape") setShowPathInput(false);
            }}
            placeholder="Path to apply to, e.g. src/README.md"
            autoFocus
          />
          <button
            className="cb-btn cb-btn-apply"
            onClick={handleApplyWithPath}
            disabled={!customPath.trim()}
            title="Apply to specified path"
          >
            Apply
          </button>
          <button className="cb-btn" onClick={() => setShowPathInput(false)} title="Cancel">
            ✕
          </button>
        </div>
      )}

      <pre className={`cb-code syntax-${language || "text"}`}>
        <code>
          {showLines
            ? (expanded ? lines : lines.slice(0, CODE_COLLAPSE_LINES)).map((line, i) => (
                <span key={i} className="cb-line">
                  <span className="cb-line-num">{i + 1}</span>
                  {line}
                  {i < lines.length - 1 ? "\n" : ""}
                </span>
              ))
            : visibleCode
          }
        </code>
      </pre>
      {collapsible && !expanded && (
        <button className="cb-expand-bar" onClick={() => setExpanded(true)}>
          Show {lines.length - CODE_COLLAPSE_LINES} more lines
        </button>
      )}
    </div>
  );
}

/** Code block shown while the AI is still streaming — interactable (copy works). */
function StreamingCodeBlock({ language, code }: { language: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const lines = code.split("\n");
  const collapsible = lines.length > CODE_COLLAPSE_LINES;
  const [expanded, setExpanded] = useState(true); // expanded by default while streaming

  const handleCopy = () => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  };

  const visibleCode = expanded ? code : lines.slice(0, CODE_COLLAPSE_LINES).join("\n");

  return (
    <div className="cb-container cb-streaming">
      <div className="cb-header">
        {collapsible && (
          <button
            className="cb-btn cb-btn-collapse"
            onClick={() => setExpanded((v) => !v)}
            title={expanded ? "Collapse code" : "Expand code"}
          >
            {expanded ? "\u25BE" : "\u25B8"} {lines.length} lines
          </button>
        )}
        <span className="cb-lang">{language}</span>
        <span className="cb-streaming-badge">streaming...</span>
        <div className="cb-actions">
          <button className="cb-btn" onClick={handleCopy} title="Copy code so far">
            {copied ? "\u2713" : "Copy"}
          </button>
        </div>
      </div>
      <pre className={`cb-code syntax-${language}`}>
        <code>{visibleCode}</code>
      </pre>
      {collapsible && !expanded && (
        <button className="cb-expand-bar" onClick={() => setExpanded(true)}>
          Show {lines.length - CODE_COLLAPSE_LINES} more lines
        </button>
      )}
    </div>
  );
}

/** A1 — try to parse an MCP App fence body into an embed. Returns
 * null when the JSON or schema doesn't validate, so the caller can
 * fall back to a CodeBlock and the user can see the raw payload. */
function renderMcpAppFence(body: string, key: number): React.ReactNode | null {
  try {
    const parsed = JSON.parse(body);
    if (
      !parsed ||
      typeof parsed !== "object" ||
      parsed.type !== "mcp.app" ||
      typeof parsed.title !== "string" ||
      typeof parsed.component !== "string" ||
      typeof parsed.version !== "string"
    ) {
      return null;
    }
    return <McpAppEmbed key={key} payload={parsed as McpAppPayload} />;
  } catch {
    return null;
  }
}

/** Render message content: parse code blocks, file references, plain text.
 *
 * A1 — MCP App payloads (SEP-1865): a fenced code block whose language
 * tag is `mcp.app`, `mcp-app`, or `application/vnd.mcp.app+json`
 * renders as a McpAppEmbed (typed React card with actions) instead of
 * a plain CodeBlock. Malformed payloads fall back to a CodeBlock so the
 * raw bytes are still visible.
 */
function renderContent(
  content: string,
  onApply?: (code: string, filename: string) => void,
): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  // Fence regex updated to allow dots, slashes, plus signs and `@` in
  // the language tag so MCP-Apps MIME-like tags can be matched. The
  // original `\w*` only matched [A-Za-z0-9_], which would have dropped
  // `application/vnd.mcp.app+json`.
  const fenceRegex = /```([\w.+/@-]*)(?:[^\S\n]+(\S+))?\n([\s\S]*?)```/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let key = 0;

  while ((match = fenceRegex.exec(content)) !== null) {
    // Text before this code block
    if (match.index > lastIndex) {
      const textBefore = content.slice(lastIndex, match.index);
      parts.push(<TextSegment key={key++} text={textBefore} />);
    }
    const lang = (match[1] || "").toLowerCase();
    const isMcpApp =
      lang === "mcp.app" ||
      lang === "mcp-app" ||
      lang === "application/vnd.mcp.app+json";
    if (isMcpApp) {
      const embed = renderMcpAppFence(match[3], key);
      if (embed) {
        parts.push(embed);
        lastIndex = match.index + match[0].length;
        key++;
        continue;
      }
      // Fall through to CodeBlock when the payload doesn't parse —
      // user still sees the raw bytes so the failure is visible.
    }
    parts.push(
      <CodeBlock
        key={key++}
        language={match[1]}
        code={match[3]}
        filename={match[2] || undefined}
        onApply={onApply}
      />
    );
    lastIndex = match.index + match[0].length;
  }

  // Remaining text (or if there was a partial unclosed code block during streaming)
  if (lastIndex < content.length) {
    const remaining = content.slice(lastIndex);
    // Check for unclosed code fence (streaming in progress)
    const unfinishedFence = remaining.match(/```(\w*)(?:[^\S\n]+(\S+))?\n([\s\S]*)$/);
    if (unfinishedFence) {
      const beforeFence = remaining.slice(0, remaining.indexOf("```"));
      if (beforeFence) {
        parts.push(<TextSegment key={key++} text={beforeFence} />);
      }
      parts.push(
        <StreamingCodeBlock
          key={key}
          language={unfinishedFence[1] || "text"}
          code={unfinishedFence[3]}
        />
      );
    } else {
      parts.push(<TextSegment key={key} text={remaining} />);
    }
  }

  return parts;
}

/** Render the prose between code fences as markdown.
 *
 * `renderContent` has already pulled fenced blocks out into `CodeBlock` (which
 * carries the Apply/copy affordances), so everything reaching here is the
 * surrounding prose — and models write that as markdown. It used to land in a
 * `<pre>`, so a reply containing a table arrived as raw `| --- |` rows and
 * every emphasis as literal asterisks.
 *
 * GFM is on, which is what makes tables, task lists and strikethrough render;
 * `remark-gfm` was missing from VibeCoder's dependencies entirely, so tables
 * could not have worked even had the renderer been wired up.
 *
 * Note this drops the old bare-path "file chip" highlighting: those chips were
 * non-interactive styling, and there is no way to apply them to text nodes
 * without a rehype plugin. Paths written as `inline code` — how models usually
 * write them — are still styled by the markdown renderer.
 */
function TextSegment({ text }: { text: string }) {
  return <Markdown text={text} />;
}

// ── Thinking block component ─────────────────────────────────────────────────

function ThinkingBlock({ text }: { text: string }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="thinking-block">
      <button className="thinking-toggle" onClick={() => setExpanded(!expanded)}>
        <span className="thinking-icon">{expanded ? "\u25BE" : "\u25B8"}</span>
        <span className="thinking-label">Thinking...</span>
      </button>
      {expanded && (
        <div className="thinking-content">
          <pre>{text}</pre>
        </div>
      )}
    </div>
  );
}

// ── Work section (collapsible per-turn work) ─────────────────────────────────
// Wraps the thinking block + tool-call cards (and the live agent step log)
// for a single assistant turn in one collapsed-by-default disclosure. Keeps
// the chat Q/response scroll clean; click the chevron to see the work.

function WorkSection({ label, children, defaultExpanded = false }: {
  label: string;
  children: React.ReactNode;
  defaultExpanded?: boolean;
}) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  return (
    <div className="work-section">
      <button
        type="button"
        className="work-toggle"
        onClick={() => setExpanded((e) => !e)}
        aria-expanded={expanded}
      >
        <span className="work-chevron">{expanded ? "▾" : "▸"}</span>
        <span className="work-label">{label}</span>
      </button>
      {expanded && <div className="work-content">{children}</div>}
    </div>
  );
}

// ── Tool call card ───────────────────────────────────────────────────────────

function ToolCallCard({ call }: { call: ToolCallInfo }) {
  const [expanded, setExpanded] = useState(false);
  const canExpand = Boolean(call.output);
  const headerChildren = (
    <>
      <span className="tool-card-icon"><ToolIcon tool={call.tool} /></span>
      <span className="tool-card-label">{toolLabel(call.tool, call.path)}</span>
      {call.path && <span className="tool-card-path" title={call.path}>{call.path}</span>}
      <span className="tool-card-status"><ToolStatusIcon status={call.status} /></span>
      {call.duration_ms != null && (
        <span className="tool-card-duration">{call.duration_ms}ms</span>
      )}
      {canExpand && (
        <span className="tool-card-expand">{expanded ? "\u25BE" : "\u25B8"}</span>
      )}
    </>
  );
  return (
    <div className={`tool-card tool-card-${call.status}`}>
      {canExpand ? (
        <button
          type="button"
          className="tool-card-header"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          aria-label={`Tool ${toolLabel(call.tool, call.path)}, toggle output`}
        >
          {headerChildren}
        </button>
      ) : (
        <div className="tool-card-header">{headerChildren}</div>
      )}
      {expanded && call.output && (
        <pre className="tool-card-output">{call.output}</pre>
      )}
    </div>
  );
}

/**
 * Extract fenced code blocks that have an EXPLICIT filename in the fence info
 * string (e.g. ```typescript src/App.tsx). Language-only blocks are excluded
 * because they cannot be safely applied without a known target path.
 *
 * Module scope, not a `useCallback`: `ChatMessageRow` needs it too, and a
 * function that closes over nothing has no business being rebuilt per render.
 */
function extractCodeBlocks(content: string): { language: string; code: string; filename: string }[] {
  const blocks: { language: string; code: string; filename: string }[] = [];
  // Group 1: language, Group 2: explicit filename token, Group 3: code
  const fenceRegex = /```(\w*)(?:[^\S\n]+(\S+))?\n([\s\S]*?)```/g;
  let match: RegExpExecArray | null;
  while ((match = fenceRegex.exec(content)) !== null) {
    if (!match[2]) continue; // skip language-only blocks — no safe target path
    blocks.push({ language: match[1], code: match[3], filename: match[2] });
  }
  return blocks;
}

/**
 * One message bubble, memoized — and the memo is the point.
 *
 * `chat:chunk` fires a state update per streamed chunk. Without this, every
 * chunk re-ran `withExtractedThinking`, `withParsedToolCalls`, `renderContent`
 * (a markdown parse per prose run) and `extractCodeBlocks` — twice — over
 * *every* message in the transcript. The cost of one response was therefore
 * O(transcript x chunks), and the transcript only grows, so each answer was
 * slower than the last for the whole session. That is what "chat gets slower
 * after a few minutes" was. Memoized, a chunk re-renders one bubble.
 *
 * Every prop must stay referentially stable across a chunk for that to hold:
 * the handlers passed in are `useCallback`s that do not depend on streaming
 * state, and `idx` is passed through so the parent needs no per-row closure.
 */
const ChatMessageRow = memo(function ChatMessageRow({
  rawMsg,
  idx,
  isLast,
  copied,
  canApply,
  onCopy,
  onApplyCode,
  onApplyAll,
  onRetry,
  onLightbox,
}: {
  rawMsg: Message;
  idx: number;
  isLast: boolean;
  copied: boolean;
  canApply: boolean;
  onCopy: (idx: number, content: string) => void;
  onApplyCode?: (code: string, filename: string) => void;
  onApplyAll: (content: string) => void;
  onRetry: () => void;
  onLightbox: (src: string) => void;
}) {
  // Normalise at the point of render, not at each append site. Raw model
  // output reaches `messages` from several paths that never parsed it —
  // session restore, the watch bridge, any future injector — and each one that
  // forgets put `<thinking>` on screen. One choke point here means no path
  // can leak it.
  const msg = useMemo(() => withParsedToolCalls(withExtractedThinking(rawMsg)), [rawMsg]);
  const body = useMemo(() => renderContent(msg.content, onApplyCode), [msg.content, onApplyCode]);
  const applyAllCount = useMemo(
    () => (canApply ? extractCodeBlocks(msg.content).length : 0),
    [canApply, msg.content],
  );
  return (
            <div>
            {msg.isSummary && (
              <div className="compaction-divider">
                <span>Conversation compacted</span>
              </div>
            )}
            <div className={`message message-${msg.role}${msg.isError ? " message-error" : ""}`}>
              <div className="message-icon">
                {msg.role === "user" ? <User size={14} strokeWidth={1.5} /> : <span className="assistant-icon">AI</span>}
              </div>
              {msg.timestamp && (
                <time className="message-time" dateTime={new Date(msg.timestamp).toISOString()}>
                  {formatTime(msg.timestamp)}
                </time>
              )}
              <div className="message-content" style={{ position: "relative" }}>
                {/* Work for this turn \u2014 thinking + tool calls, collapsed by default */}
                {(msg.thinking || (msg.toolCalls && msg.toolCalls.length > 0)) && (
                  <WorkSection label={workLabel(msg.thinking, msg.toolCalls)}>
                    {msg.thinking && (
                      <div className="thinking-block">
                        <div className="thinking-content">
                          <pre>{msg.thinking}</pre>
                        </div>
                      </div>
                    )}
                    {msg.toolCalls && msg.toolCalls.length > 0 && (
                      <div className="tool-cards">
                        {msg.toolCalls.map((tc, ti) => (
                          <ToolCallCard key={ti} call={tc} />
                        ))}
                      </div>
                    )}
                  </WorkSection>
                )}

                {/* Attachments on user messages */}
                {msg.attachments && msg.attachments.length > 0 && (
                  <div className="msg-attachments">
                    <div className="msg-attachments-label">
                      <Paperclip size={11} />
                      {msg.attachments.length} file{msg.attachments.length > 1 ? "s" : ""} attached
                    </div>
                    {msg.attachments.map((att, ai) => {
                      const isImage = att.mime_type.startsWith("image/");
                      const imgSrc = att.previewUrl || (att.data ? `data:${att.mime_type};base64,${att.data}` : undefined);
                      const sizeStr = att.size < 1024 ? `${att.size} B`
                        : att.size < 1024 * 1024 ? `${(att.size / 1024).toFixed(1)} KB`
                        : `${(att.size / (1024 * 1024)).toFixed(1)} MB`;
                      return (
                        <div key={ai} className="msg-attachment-chip">
                          {isImage ? (
                            <div className="msg-attachment-image">
                              <img
                                src={imgSrc}
                                alt={att.name}
                                className="msg-attachment-thumb"
                                onClick={() => imgSrc && onLightbox(imgSrc)}
                                title="Click to enlarge"
                              />
                              <div className="msg-attachment-image-actions">
                                <span className="msg-attachment-name">{att.name}</span>
                                <button className="msg-attachment-zoom" onClick={() => imgSrc && onLightbox(imgSrc)} title="View full size">
                                  <ZoomIn size={12} />
                                </button>
                              </div>
                            </div>
                          ) : (
                            <div className="msg-attachment-file">
                              <FileText size={14} />
                              <span className="msg-attachment-name" title={att.name}>{att.name}</span>
                              <span className="msg-attachment-size">{sizeStr}</span>
                              {att.text_content && <span className="msg-attachment-check" title="Content sent to AI">&#10003;</span>}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Main content */}
                {msg.role === "assistant" ? (
                  <div className="msg-rendered">
                    {body}
                  </div>
                ) : (
                  <pre className="msg-text">{msg.content}</pre>
                )}

                {/* Action buttons for assistant messages */}
                {msg.role === "assistant" && !msg.isError && (
                  <div className="msg-actions">
                    <button
                      className="msg-copy-btn"
                      onClick={() => {
                        onCopy(idx, msg.content);
                      }}
                      title="Copy response"
                    >
                      {copied ? "\u2713 Copied" : "Copy"}
                    </button>
                    {applyAllCount > 1 && (
                      <button
                        className="msg-apply-all-btn"
                        onClick={() => onApplyAll(msg.content)}
                        title="Apply all explicitly-named code blocks to their target files"
                      >
                        Apply All ({applyAllCount} files)
                      </button>
                    )}
                  </div>
                )}

                {/* Error retry button */}
                {msg.isError && isLast && (
                  <button className="msg-retry-btn" onClick={onRetry} title="Retry last message">
                    Retry
                  </button>
                )}

                {/* Metrics badge */}
                {msg.metrics && <MetricsBadge metrics={msg.metrics} />}
              </div>
            </div>
            </div>
  );
});

// ── Metrics badge ────────────────────────────────────────────────────────────

function MetricsBadge({ metrics }: { metrics: MessageMetrics }) {
  const parts: string[] = [];
  if (metrics.completion_tokens) parts.push(`${metrics.completion_tokens} tokens`);
  if (metrics.latency_ms) parts.push(`${metrics.latency_ms}ms`);
  if (metrics.tokens_per_sec) parts.push(`${Math.round(metrics.tokens_per_sec)} tok/s`);
  if (metrics.model) parts.push(metrics.model);

  if (parts.length === 0) return null;

  return (
    <div className="metrics-badge">
      {parts.join(" \u00B7 ")}
    </div>
  );
}

// ── Provider health dot ──────────────────────────────────────────────────────

function HealthDot({ score }: { score: number }) {
  const cls = score > 0.8 ? "health-green" : score > 0.5 ? "health-yellow" : "health-red";
  return <span className={`health-dot ${cls}`} title={`Health: ${Math.round(score * 100)}%`} />;
}

// ── Slash command palette ────────────────────────────────────────────────────

function SlashPalette({ query, onSelect, onClose }: {
  query: string;
  onSelect: (cmd: SlashCommand) => void;
  onClose: () => void;
}) {
  const filtered = SLASH_COMMANDS.filter(
    (c) => c.command.startsWith(query.toLowerCase())
  );
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [prevQuery, setPrevQuery] = useState(query);
  if (prevQuery !== query) {
    setPrevQuery(query);
    if (selectedIdx !== 0) setSelectedIdx(0);
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") { onClose(); return; }
      if (e.key === "ArrowDown") { e.preventDefault(); setSelectedIdx((i) => Math.min(i + 1, filtered.length - 1)); }
      if (e.key === "ArrowUp") { e.preventDefault(); setSelectedIdx((i) => Math.max(i - 1, 0)); }
      if (e.key === "Enter" && filtered.length > 0) { e.preventDefault(); onSelect(filtered[selectedIdx]); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [filtered, selectedIdx, onSelect, onClose]);

  if (filtered.length === 0) return null;

  return (
    <div className="slash-palette">
      {filtered.map((cmd, i) => (
        <div
          key={cmd.command}
          className={`slash-item ${i === selectedIdx ? "slash-item-active" : ""}`}
          onClick={() => onSelect(cmd)}
          onMouseEnter={() => setSelectedIdx(i)}
        >
          <span className="slash-cmd">{cmd.command}</span>
          <span className="slash-desc">{cmd.description}</span>
        </div>
      ))}
    </div>
  );
}

// ── Main component ───────────────────────────────────────────────────────────

/**
 * The project's README, read once per workspace, for the voice context block.
 *
 * Read here rather than in the daemon because the shell already has the file
 * tree and a path-guarded `read_file`; the daemon would need a workspace root
 * of its own to do the same safely. Failure is silence, not an error: a repo
 * with no README is normal, and voice must still work without one.
 */
function useProjectReadme(root: string | null | undefined, tree: string[] | undefined): string | null {
  const [readme, setReadme] = useState<string | null>(null);
  const path = useMemo(() => (tree?.length ? findReadme(tree) : undefined), [tree]);

  useEffect(() => {
    if (!path) {
      setReadme(null);
      return;
    }
    let alive = true;
    invoke<string>("read_file", { path })
      .then((text) => {
        if (alive) setReadme(text.slice(0, VOICE_CONTEXT_LIMITS.readme * 2));
      })
      .catch(() => {
        if (alive) setReadme(null);
      });
    return () => {
      alive = false;
    };
  }, [path, root]);

  return readme;
}

export function AIChat({
  provider,
  context,
  fileTree,
  currentFile,
  workspacePath,
  onFileAction,
  onPendingWrite,
  pendingInput,
  onPendingInputConsumed,
  messages: controlledMessages,
  onMessagesChange,
  pinnedMemory,
  sessionId,
  sessionTitle,
  useAgentLoop = false,
  onUseAgentLoopChange,
  approvalMode: controlledApprovalMode,
  onApprovalModeChange,
  onSwitchToGoals,
  onOpenFile,
}: AIChatProps) {
  const [agentMode, setAgentMode] = useState<AgentMode>("chat");

  // Approval mode is controlled by the tab manager when it supplies both the
  // value and the setter; otherwise AIChat keeps its own (uncontrolled panels,
  // tests, embedded uses).
  const [localApprovalMode, setLocalApprovalMode] = useState<ApprovalMode>(DEFAULT_APPROVAL_MODE);
  const approvalMode = controlledApprovalMode ?? localApprovalMode;
  const setApprovalMode = useCallback(
    (mode: ApprovalMode) => {
      setLocalApprovalMode(mode);
      onApprovalModeChange?.(mode);
    },
    [onApprovalModeChange],
  );
  const approvalModeRef = useRef(approvalMode);
  approvalModeRef.current = approvalMode;
  const [localMessages, setLocalMessages] = useState<Message[]>([]);
  const messages = controlledMessages ?? localMessages;

  // Keep refs to the latest values so event-listener closures never go stale
  // and the listener effect doesn't re-run on every render.
  const messagesRef = useRef(messages);
  messagesRef.current = messages;
  const onFileActionRef = useRef(onFileAction);
  onFileActionRef.current = onFileAction;
  const onPendingWriteRef = useRef(onPendingWrite);
  onPendingWriteRef.current = onPendingWrite;
  const onMessagesChangeRef = useRef(onMessagesChange);
  onMessagesChangeRef.current = onMessagesChange;

  // Refs for props the chat:complete listener needs when auto-continuing.
  // The listener is registered once with [setMessages] deps, so reading
  // these props directly would capture stale values across renders.
  const providerRef = useRef(provider);
  providerRef.current = provider;
  const contextRef = useRef(context);
  contextRef.current = context;
  const fileTreeRef = useRef(fileTree);
  fileTreeRef.current = fileTree;
  const currentFileRef = useRef(currentFile);
  currentFileRef.current = currentFile;
  const sessionIdRef = useRef(sessionId);
  sessionIdRef.current = sessionId;
  const sessionTitleRef = useRef(sessionTitle);
  sessionTitleRef.current = sessionTitle;
  const pinnedMemoryRef = useRef(pinnedMemory);
  pinnedMemoryRef.current = pinnedMemory;

  // Auto-continue turn counter. Reset to 0 at the start of every user-initiated
  // sendMessage; bumped for each automatic re-invocation triggered by
  // tool_output coming back from the backend. See MAX_AGENT_TURNS.
  const agentTurnCountRef = useRef(0);
  // Counted separately from agent turns: this bounds how far one answer may
  // continue itself after being cut at the output cap. See
  // MAX_TRUNCATION_CONTINUES.
  const truncationContinueRef = useRef(0);

  // Live ref to useAgentLoop so the listener closures (registered once) can
  // read the current value when deciding whether they're handling agent or
  // chat events. Keeping this in a ref also lets sendMessage pick the route
  // without re-creating the callback on every toggle.
  const useAgentLoopRef = useRef(useAgentLoop);
  useAgentLoopRef.current = useAgentLoop;

  // True only when this AIChat instance invoked start_agent_task and hasn't
  // yet seen a terminal event (complete/error/partial). Gates agent:* events
  // so a chat tab that didn't start the run ignores another tab's events.
  // Cleared on terminal events and on stop. Phase-1 single-slot constraint:
  // only one agent run can be in flight across the whole app.
  const agentRunOwnerRef = useRef(false);

  // In controlled mode, multiple Tauri events can fire before React
  // re-renders (e.g. chat:complete then chat:metrics). Each call to
  // setMessages reads messagesRef.current for the "prev" value, but
  // that ref is only updated on render. Without tracking, the second
  // caller sees stale data and silently drops the first caller's update.
  //
  // pendingMessagesRef tracks the latest value we've committed — to the parent
  // when controlled, to local state when not — so rapid-fire updaters always
  // chain off the most recent state in either mode.
  const pendingMessagesRef = useRef<Message[] | null>(null);

  // Sync: once React renders with that value, clear the pending one.
  useEffect(() => {
    pendingMessagesRef.current = null;
  }, [messages]);

  const setMessages = useCallback((update: Message[] | ((prev: Message[]) => Message[])) => {
    // The updater is resolved here, before the branch, so that callers can read
    // the resulting list synchronously — `messagesRef.current` is the resolved
    // list in both modes, so there is no reason for them to differ.
    //
    // The local branch used to hand the updater straight to `setLocalMessages`,
    // where React runs it during the next render instead. Anything reading the
    // post-update list right after the call therefore saw nothing, and the
    // `chat:complete` auto-continue is exactly that: its `captured.value` stayed
    // null, `shouldAutoContinue` was never true, and the turn ended silently.
    // The Sandbox tab renders this component uncontrolled, so it never resumed
    // after a tool call or after a reply cut off at the model's output cap —
    // while the Chat tab, which is controlled, did.
    //
    // Use pending (most recent uncommitted) value if available, otherwise fall
    // back to the last rendered list.
    const current = pendingMessagesRef.current ?? messagesRef.current;
    const next = typeof update === "function" ? update(current) : update;
    pendingMessagesRef.current = next;
    if (onMessagesChangeRef.current) {
      onMessagesChangeRef.current(next);
    } else {
      setLocalMessages(next);
    }
  }, []);

  // Watch → VibeCoder sync: poll sessions.db for messages sent from the Watch app.
  // When new messages arrive (role=user from Watch, role=assistant from daemon LLM),
  // append them to the chat history so both sides stay in sync.
  //
  // This tab writes its *own* turns to that same table (see write_to_session_store
  // on the Rust side), so the poll would otherwise hand every local reply straight
  // back. Two guards: the cursor is advanced past our own rows on `chat:complete`
  // (watchSync.skipPast), and anything that slips through the ~1s poll window is
  // dropped by content match below.
  const watchSync = useWatchSync(sessionId, (watchMsgs: WatchSyncMessage[]) => {
    setMessages(prev => {
      // Content dedup, normalized: the DB row holds the raw provider text, while
      // the rendered message has had its <thinking> block and tool XML lifted out.
      // Comparing raw-vs-cleaned never matches, which is exactly how a reply with
      // reasoning ended up on screen twice.
      const recentContents = new Set(prev.slice(-20).map(m => dedupKey(m.content)));
      const newMsgs: Message[] = watchMsgs
        .filter(wm => !recentContents.has(dedupKey(wm.content)))
        .map(wm => ({
          role: wm.role === 'assistant' ? 'assistant' as const : 'user' as const,
          content: wm.content,
          timestamp: wm.created_at || undefined,
        }));
      return newMsgs.length > 0 ? [...prev, ...newMsgs] : prev;
    });
  });
  const watchSyncRef = useRef(watchSync);
  watchSyncRef.current = watchSync;

  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  // ── Live stream text ────────────────────────────────────────────────────────
  //
  // `chat:chunk` arrives once per streamed chunk, each as its own Tauri event
  // and so its own React render. Every one of those renders re-runs
  // `extractThinking` and a full markdown parse over the *whole* reply so far,
  // which makes the cost of a response quadratic in its own length: a long
  // answer visibly slows down as it is written.
  //
  // So the accumulator is a ref — the exact, immediate text — and the state is
  // a view of it published at most once per `STREAM_FLUSH_MS`. Chunks arriving
  // faster than that collapse into one render; a slow model, where chunks are
  // further apart than the interval, is unaffected and renders exactly as
  // before.
  //
  // `setStreamingText` keeps the `useState` setter's signature so the sixteen
  // call sites do not have to know any of this. Clearing is the exception: it
  // publishes immediately, because several paths clear the stream and then
  // read or commit `messages` in the same tick, and a pending flush landing
  // afterwards would put the finished reply back on screen a second time.
  const [streamingText, setStreamingTextState] = useState("");
  /** Tool tags the backend refused this turn, reported when the turn ends. */
  const toolRejectionsRef = useRef<string[]>([]);
  const streamAccumRef = useRef("");
  const streamFlushRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const streamFlushedAtRef = useRef(0);

  /**
   * Move the throughput readout to match what was just published.
   *
   * Derived from the character counter rather than pushed per chunk, so a
   * chunk that is coalesced away costs no state update at all — which is what
   * makes the throttle worth anything.
   */
  const publishStreamMetrics = useCallback(() => {
    const approxTokens = Math.round(streamCharsRef.current / 4);
    setStreamTokenCount(approxTokens);
    const started = streamStartMsRef.current;
    if (started !== null) {
      const elapsedSec = (Date.now() - started) / 1000;
      if (elapsedSec > 0) setTokensPerSec(Math.round(approxTokens / elapsedSec));
    }
  }, []);

  const setStreamingText = useCallback(
    (next: string | ((prev: string) => string)) => {
      streamAccumRef.current =
        typeof next === "function" ? next(streamAccumRef.current) : next;

      const publish = () => {
        streamFlushRef.current = null;
        streamFlushedAtRef.current = Date.now();
        setStreamingTextState(streamAccumRef.current);
        publishStreamMetrics();
      };

      if (streamAccumRef.current === "") {
        if (streamFlushRef.current !== null) {
          clearTimeout(streamFlushRef.current);
          streamFlushRef.current = null;
        }
        streamFlushedAtRef.current = 0;
        setStreamingTextState("");
        return;
      }
      if (streamFlushRef.current !== null) return;   // already queued
      // Leading edge: the first chunk of a reply is published at once, so the
      // answer still starts the instant it starts. Only chunks arriving inside
      // the window behind it are coalesced — the throttle is a ceiling on
      // render frequency, never latency added to a chunk that could have been
      // shown immediately.
      const since = Date.now() - streamFlushedAtRef.current;
      if (since >= STREAM_FLUSH_MS) {
        publish();
        return;
      }
      streamFlushRef.current = setTimeout(publish, STREAM_FLUSH_MS - since);
    },
    [publishStreamMetrics],
  );

  /** Extend the live stream by one arriving chunk. */
  const appendStreamChunk = useCallback(
    (chunk: string) => setStreamingText((prev) => prev + chunk),
    [setStreamingText],
  );

  // A pending flush after unmount would set state on a dead component.
  useEffect(
    () => () => {
      if (streamFlushRef.current !== null) clearTimeout(streamFlushRef.current);
    },
    [],
  );
  const [pickerQuery, setPickerQuery] = useState<string | null>(null);
  const [slashQuery, setSlashQuery] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<ChatAttachment[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [isAttachLoading, setIsAttachLoading] = useState(false);
  const [lightboxSrc, setLightboxSrc] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
  const [providerHealth, setProviderHealth] = useState<number>(1.0);
  const [streamStatus, setStreamStatus] = useState<string | null>(null);
  const [retryInfo, setRetryInfo] = useState<{ attempt: number; max: number } | null>(null);

  // Agent-loop UI state (only meaningful when useAgentLoop is true).
  // pendingApproval: the current ToolCallPending awaiting user approve/reject.
  // agentSteps: completed step cards rendered between the user prompt and the
  // final agent:complete summary. Cleared when the run terminates.
  const [pendingApproval, setPendingApproval] = useState<{
    name: string;
    summary: string;
    is_destructive: boolean;
  } | null>(null);
  const [agentSteps, setAgentSteps] = useState<Array<{
    step_num: number;
    tool_name: string;
    tool_summary: string;
    output: string;
    success: boolean;
    approved: boolean;
  }>>([]);
  // verifierResult: PASS / NITS / FAIL summary from the verifier subagent's
  // PostToolUse hook on task_complete. Cleared on new run / terminal events.
  const [verifierResult, setVerifierResult] = useState<{
    status: "pass" | "nits" | "fail";
    message: string;
  } | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const cancelledRef = useRef(false);
  const [isNearBottom, setIsNearBottom] = useState(true);

  // Streaming speed metrics
  const streamStartMsRef = useRef<number | null>(null);
  const streamCharsRef = useRef<number>(0);
  const [tokensPerSec, setTokensPerSec] = useState<number | null>(null);
  const [streamTokenCount, setStreamTokenCount] = useState<number>(0);

  // When chat:complete fires in controlled mode, we defer clearing streaming
  // state until the finalized message actually arrives in `messages`.
  // This counter increments on each completion; the useEffect below watches
  // `messages` and clears streaming state when it catches up.
  const pendingClearRef = useRef(0);

  // "Approve all" — once set, every later tool call in the *current* run is
  // approved without prompting. Reset when a run starts or ends, so it never
  // silently carries into the next task.
  const [autoApprove, setAutoApprove] = useState(false);
  const autoApproveRef = useRef(false);
  autoApproveRef.current = autoApprove;

  useEffect(() => {
    // When messages changes and we have a pending clear, the parent has
    // propagated the new message — safe to clear streaming state now.
    if (pendingClearRef.current > 0) {
      pendingClearRef.current = 0;
      setStreamingText("");
      setTokensPerSec(null);
      setStreamTokenCount(0);
      setStreamStatus(null);
      setRetryInfo(null);
      setIsLoading(false);
    }
  }, [messages, setStreamingText]);

  // ── Auto-compaction ──────────────────────────────────────────────────────────
  // When the conversation outgrows the model's context budget, summarise the
  // older half and splice it in as a single summary message. Never fires while
  // a response is in-flight.
  const COMPACTION_KEEP_LAST = 20;
  const isCompactingRef = useRef(false);
  const lastCompactionLengthRef = useRef(0);

  // The budget the selected model actually has, in characters.
  //
  // Three states, and they are genuinely different things:
  //   `undefined` — not asked yet. Compaction waits, because acting on the
  //                 default here would summarise away a conversation a
  //                 million-token model had ample room for, one render before
  //                 the real answer arrived.
  //   `null`      — asked, and the vendor does not publish the number
  //                 (Anthropic and OpenAI do not). DEFAULT_COMPACTION_CHARS
  //                 applies: the constant this panel always used, so an
  //                 unpublished model behaves exactly as before rather than
  //                 being handed a guess.
  //   a number    — the model's real budget.
  //
  // Before this, one constant governed every model. On a small local model the
  // panel let the conversation grow far past what the server could hold, and
  // Ollama's answer to an oversized prompt is to drop the *front* of it — the
  // system prompt and the tool contract — silently.
  const [contextBudgetChars, setContextBudgetChars] = useState<number | null | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    setContextBudgetChars(undefined);
    if (!provider) {
      // No provider selected is a settled answer, not a pending one — nothing
      // is going to resolve it, so compaction must not wait forever.
      setContextBudgetChars(null);
      return;
    }
    invoke<number | null>("model_context_budget", { provider, model: null })
      .then((tokens) => {
        // Same 4-chars-per-token estimate the Rust side prunes by
        // (`vibe_ai::agent::estimate_tokens`); the two must agree or the panel
        // and the loop compact at different points.
        if (!cancelled) setContextBudgetChars(tokens ? tokens * 4 : null);
      })
      // An unresolvable provider is not a reason to stop compacting — it is a
      // reason to compact on the default.
      .catch(() => { if (!cancelled) setContextBudgetChars(null); });
    return () => { cancelled = true; };
  }, [provider]);

  useEffect(() => {
    if (isLoading) return;                          // never interrupt a stream
    if (isCompactingRef.current) return;
    if (messages.length < COMPACTION_KEEP_LAST + 2) return;
    if (contextBudgetChars === undefined) return;   // budget not resolved yet
    const threshold = contextBudgetChars ?? DEFAULT_COMPACTION_CHARS;
    // Require at least 10k new chars since last compaction to avoid re-triggering
    const totalChars = messages.reduce((s, m) => s + m.content.length, 0);
    if (totalChars < threshold) return;
    if (totalChars - lastCompactionLengthRef.current < 10_000) return;

    isCompactingRef.current = true;
    lastCompactionLengthRef.current = totalChars;

    const toSummarise = messages.slice(0, messages.length - COMPACTION_KEEP_LAST);
    const kept = messages.slice(messages.length - COMPACTION_KEEP_LAST);

    const summaryPrompt = "Summarise the following conversation into a concise paragraph (max 300 words) preserving key facts, decisions, and any important code snippets mentioned:\n\n"
      + toSummarise.map((m) => `${m.role}: ${m.content}`).join("\n\n");

    invoke<string>("summarise_messages", { provider, content: summaryPrompt })
      .then((summaryText) => {
        const summaryMsg: Message = {
          role: "assistant",
          content: `Conversation summary (earlier messages compacted):\n\n${summaryText}`,
          isSummary: true,
          timestamp: Date.now(),
        };
        setMessages([summaryMsg, ...kept]);
      })
      .catch((err) => {
        // Summarising failed, so the earlier turns are being *dropped*, not
        // compacted. Saying "compacted to save context" here — as this did
        // while the backing command did not exist at all — tells the user
        // their conversation was preserved when it was discarded.
        const summaryMsg: Message = {
          role: "assistant",
          content:
            `[Could not summarise the earlier ${toSummarise.length} messages, so they were ` +
            `dropped to stay within the context window. Reason: ${err}]`,
          isSummary: true,
          timestamp: Date.now(),
        };
        setMessages([summaryMsg, ...kept]);
      })
      .finally(() => {
        isCompactingRef.current = false;
      });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages, isLoading, contextBudgetChars]);

  const { toast } = useToast();

  // Voice input. `tauriTranscriber` with no URL targets the local daemon's
  // /voice/transcribe, which prefers a downloaded whisper model over Groq —
  // the old inline hook called Groq directly and could never run offline.
  const appendTranscript = useCallback(
    (transcript: string) =>
      setInput((prev) => (prev ? `${prev.replace(/\s+$/, "")} ` : "") + transcript.trim()),
    [],
  );
  const voiceTranscribe = useMemo(() => tauriTranscriber(), []);
  const {
    isListening,
    isTranscribing,
    interimText,
    toggle: toggleVoice,
    error: voiceError,
    clearError: clearVoiceError,
  } = useVoiceInput({ onTranscript: appendTranscript, transcribe: voiceTranscribe });

  // Full-duplex conversation. The model is resolved here rather than left to
  // the daemon: `chat_provider_for` needs *both* a provider and a model to
  // build an override and silently uses whatever the daemon booted with
  // otherwise — the silent default the provider-agnostic rule exists to
  // prevent.
  //
  // `provider` is the toolbar's selection, and the toolbar lists display names
  // (`"Ollama (gpt-oss:120b-cloud)"`), not provider ids. Indexing
  // PROVIDER_DEFAULT_MODEL with one returns `undefined`, so the model went
  // missing and the fallback fired — the microphone transcribed, the turns
  // appeared, and nothing ever answered. `parseProviderSelection` is the
  // existing helper for exactly this, and it keeps the model the user picked
  // rather than substituting the registry default.
  const duplexSelection = useMemo(() => parseProviderSelection(provider), [provider]);
  const voicePref = useVoiceDuplexPreference();

  // What the spoken path knows about the project.
  //
  // The typed path can go and look — it has `<read_file>` — so a file tree is
  // enough to get it started. Voice has one round trip and no tools, so a tree
  // of paths is all it will ever have: asked to summarise the project it
  // answered "just a collection of directories and files, I couldn't tell what
  // Gbrain is", which is a fair description of a listing with no README in it.
  // Root, README and the open file's *contents* (not just its path) go in too.
  const readme = useProjectReadme(workspacePath, fileTree);
  const duplexContext = useMemo(
    () =>
      buildVoiceContext({
        root: workspacePath,
        pinned: pinnedMemory,
        readme,
        openFile: currentFile,
        // `context` is the editor material the typed path sends — selection
        // and open-file text — so it belongs with the open file, not as a
        // nameless block of prose.
        extra: context,
        tree: fileTree,
      }),
    [workspacePath, pinnedMemory, readme, currentFile, context, fileTree],
  );

  const duplex = useVoiceDuplex({
    enabled: voicePref.enabled,
    provider: duplexSelection.provider,
    model: duplexSelection.model,
    // `auto`, not a pinned language. Pinning one does not merely bias the
    // recogniser — it *suppresses* the detection result, so every turn came
    // back labelled English, the reply rule never fired, and a question asked
    // in Hindi was answered in English. Detection runs per turn because
    // code-switching mid-conversation is normal for multilingual speakers.
    language: "auto",
    context: duplexContext,
    workspaceRoot: workspacePath,
    onTurn: turn =>
      setMessages(prev => [
        ...prev,
        { role: turn.role, content: turn.text, timestamp: Date.now() } satisfies Message,
      ]),
    // Asked to open a file, the assistant used to read it and describe it: the
    // information arrived, the editor never moved. This is the hand-off that
    // was missing — and its presence is what tells the daemon to offer the
    // tool at all, so a host that cannot show a file is never promised one.
    onOpenFile,
  });

  // The hook reports failures as state; VibeCoder already has a toast surface,
  // so mirror them there and clear so the same error can be raised again.
  useEffect(() => {
    if (!voiceError) return;
    toast.warn(voiceError);
    clearVoiceError();
  }, [voiceError, toast, clearVoiceError]);

  // The duplex button reports a failure as the word "Voice error" with the
  // reason in a `title` — so the only way to learn why the assistant stopped
  // answering was to hover a button and wait for a tooltip. The reason is
  // already on the state; show it.
  const duplexError = duplex.state.status === "error" ? duplex.state.message : null;
  const reportedDuplexError = useRef<string | null>(null);
  useEffect(() => {
    if (!duplexError || reportedDuplexError.current === duplexError) return;
    reportedDuplexError.current = duplexError;
    toast.error(`Voice: ${duplexError}`);
  }, [duplexError, toast]);
  useEffect(() => {
    if (!duplexError) reportedDuplexError.current = null;
  }, [duplexError]);

  // ── Attachment handlers ─────────────────────────────────────────────────────

  /** Convert a browser File to a ChatAttachment. */
  const fileToAttachment = useCallback(async (file: File): Promise<ChatAttachment | null> => {
    if (file.size > MAX_ATTACHMENT_SIZE) {
      toast.warn(`File "${file.name}" is too large (max 20 MB).`);
      return null;
    }

    const mime = file.type || "application/octet-stream";

    // For text/code files, read as text directly (no base64 round-trip)
    if (isTextFile(mime, file.name)) {
      try {
        const textContent = await file.text();
        return {
          name: file.name,
          mime_type: mime,
          data: "",  // no base64 needed
          size: file.size,
          text_content: textContent,
        };
      } catch {
        // Fall through to binary path if text read fails
      }
    }

    // For images/binary: base64 encode
    const arrayBuf = await file.arrayBuffer();
    const bytes = new Uint8Array(arrayBuf);
    let binary = "";
    for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
    const data = btoa(binary);

    const att: ChatAttachment = {
      name: file.name,
      mime_type: mime,
      data,
      size: file.size,
    };

    // Generate preview URL for images
    if (IMAGE_TYPES.includes(file.type)) {
      att.previewUrl = URL.createObjectURL(file);
    }

    return att;
  }, [toast]);

  /** Add files from a FileList (drop, paste, or native input). */
  const addFiles = useCallback(async (files: FileList | File[]) => {
    const fileArray = Array.from(files);
    const remaining = MAX_ATTACHMENTS - attachments.length;
    if (remaining <= 0) {
      toast.warn(`Maximum ${MAX_ATTACHMENTS} attachments per message.`);
      return;
    }
    const toProcess = fileArray.slice(0, remaining);
    setIsAttachLoading(true);
    try {
      const results = await Promise.all(toProcess.map(fileToAttachment));
      const valid = results.filter((a): a is ChatAttachment => a !== null);
      if (valid.length > 0) {
        setAttachments((prev) => [...prev, ...valid]);
      }
    } finally {
      setIsAttachLoading(false);
    }
  }, [attachments.length, fileToAttachment, toast]);

  /** Open native file picker via Tauri dialog. */
  const openFilePicker = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        title: "Attach files to chat",
        filters: [
          { name: "All Files", extensions: ["*"] },
          { name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] },
          { name: "Documents", extensions: ["pdf", "csv", "json", "xml", "md", "txt", "log"] },
          { name: "Code", extensions: ["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "rb", "swift", "kt", "sql", "yaml", "toml", "html", "css"] },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      const remaining = MAX_ATTACHMENTS - attachments.length;
      if (remaining <= 0) {
        toast.warn(`Maximum ${MAX_ATTACHMENTS} attachments per message.`);
        return;
      }
      setIsAttachLoading(true);
      try {
        for (const filePath of paths.slice(0, remaining)) {
          try {
            const att = await invoke<ChatAttachment>("read_attachment", { path: filePath });
            // Generate preview for images
            if (att.mime_type.startsWith("image/")) {
              att.previewUrl = `data:${att.mime_type};base64,${att.data}`;
            }
            setAttachments((prev) => [...prev, att]);
          } catch (e) {
            toast.error(`Failed to read "${filePath}": ${e}`);
          }
        }
      } finally {
        setIsAttachLoading(false);
      }
    } catch (e) {
      console.error("File picker error:", e);
    }
  }, [attachments.length, toast]);

  /** Remove an attachment by index. */
  const removeAttachment = useCallback((idx: number) => {
    setAttachments((prev) => {
      const removed = prev[idx];
      if (removed?.previewUrl?.startsWith("blob:")) {
        URL.revokeObjectURL(removed.previewUrl);
      }
      return prev.filter((_, i) => i !== idx);
    });
  }, []);

  /** Handle drag over the chat area. */
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
    if (e.dataTransfer.files.length > 0) {
      addFiles(e.dataTransfer.files);
    }
  }, [addFiles]);

  /** Handle paste — detect images from clipboard. */
  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    const files: File[] = [];
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.kind === "file") {
        const file = item.getAsFile();
        if (file) files.push(file);
      }
    }
    if (files.length > 0) {
      e.preventDefault(); // prevent pasting file name as text
      addFiles(files);
    }
    // If no files, let the default paste behavior handle text
  }, [addFiles]);

  // Cleanup preview URLs on unmount
  useEffect(() => {
    return () => {
      attachments.forEach((a) => {
        if (a.previewUrl?.startsWith("blob:")) URL.revokeObjectURL(a.previewUrl);
      });
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Map agent mode to backend mode string
  const backendMode = useMemo(() => {
    switch (agentMode) {
      case "fast": return "fast";
      case "chat": return "chat";
      case "planning": return "planning";
    }
  }, [agentMode]);
  const backendModeRef = useRef(backendMode);
  backendModeRef.current = backendMode;

  // Track scroll position
  const handleScroll = useCallback(() => {
    const el = messagesContainerRef.current;
    if (!el) return;
    const threshold = 80;
    setIsNearBottom(el.scrollHeight - el.scrollTop - el.clientHeight < threshold);
  }, []);

  // Auto-scroll
  useEffect(() => {
    if (isNearBottom) {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, streamingText, isLoading, isNearBottom]);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    setIsNearBottom(true);
  }, []);

  // Register Tauri event listeners
  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      // chat:chunk
      const u1 = await listen<string>("chat:chunk", (e) => {
        const chunk = e.payload;
        if (streamStartMsRef.current === null) streamStartMsRef.current = Date.now();
        streamCharsRef.current += chunk.length;
        // The throughput readout is derived inside the publish rather than set
        // here. Setting it per chunk re-rendered the panel per chunk on its
        // own, which made throttling the text pointless: the live bubble's
        // markdown was re-parsed every token anyway, by a state update whose
        // only job was to move a number in the status bar.
        appendStreamChunk(chunk);

        // Check for thinking blocks in streaming text for status bar
        if (chunk.includes("<thinking>") || chunk.includes("</thinking>")) {
          setStreamStatus("Thinking...");
        }
      });
      if (cancelled) { u1(); return; }
      unlisteners.push(u1);

      // chat:complete
      const u2 = await listen<ChatResponse>("chat:complete", (e) => {
        const response = e.payload;
        // The backend persisted this turn to sessions.db before emitting. Move
        // the watch-sync cursor past those rows so the poll doesn't hand our own
        // reply back as a second bubble.
        watchSyncRef.current.skipPast(response.session_msg_id);
        const [cleanedContent, thinkingText] = extractThinking(response.message);
        const [finalContent, toolCalls] = parseToolCalls(cleanedContent);
        const hasToolOutput = !!(response.tool_output && response.tool_output.trim());

        // Capture the post-update message list for potential auto-continue.
        // `setMessages` resolves its updater before it commits, in both
        // controlled and local modes, so this ref is populated before the next
        // line executes. That was only true of the controlled path until
        // recently, and the local path's silence was invisible: no error, just
        // a Sandbox tab that never continued. Pinned by
        // `resumes ... in local (uncontrolled) mode too` in AIChat.test.tsx.
        const captured: { value: Message[] | null } = { value: null };
        // A completion with no content, no reasoning and no tools is a dead
        // turn (providers return one when the request ends on an assistant
        // message). Don't append an empty bubble for it.
        const isEmptyTurn =
          !finalContent && !thinkingText && toolCalls.length === 0 && !hasToolOutput;
        setMessages((prev) => {
          const msg: Message = {
            role: "assistant",
            content: finalContent,
            timestamp: Date.now(),
            thinking: thinkingText || undefined,
            toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
            rawContent: cleanedContent !== finalContent ? cleanedContent : undefined,
          };
          const updated = isEmptyTurn ? [...prev] : [...prev, msg];

          if (hasToolOutput) {
            const rawToolOutput = response.tool_output.trim();
            updated.push({
              role: "assistant",
              content: summarizeToolOutput(rawToolOutput),
              rawContent: rawToolOutput,
              timestamp: Date.now(),
              isToolOutput: true,
            });
          }
          // Tool tags the backend refused this turn. Reported here, at the end
          // of the turn, so the user learns why a file they were told about is
          // not on disk — the alternative is silence and a missing file.
          if (toolRejectionsRef.current.length > 0) {
            updated.push({
              role: "assistant",
              content:
                `Some tool calls in this reply were ignored because they could not be acted on:\n\n`
                + toolRejectionsRef.current.map((r) => `- ${r}`).join("\n")
                + `\n\nNothing was written for them. Asking again usually fixes it; `
                + `a model that keeps doing this is emitting malformed tool markup.`,
              timestamp: Date.now(),
              isError: true,
            });
            toolRejectionsRef.current = [];
          }
          captured.value = updated;
          return updated;
        });
        const nextMessages = captured.value;

        if (response.pending_write && onPendingWriteRef.current) {
          onPendingWriteRef.current(response.pending_write.path, response.pending_write.content);
        }
        // If the backend wrote files (tool_output mentions "Wrote file"), refresh
        // the explorer so new/modified files appear immediately.
        if (response.tool_output && /Wrote file/i.test(response.tool_output)) {
          window.dispatchEvent(new Event("vibecoder:refresh-files"));
        }
        if (onFileActionRef.current) onFileActionRef.current();

        // Auto-continue: when the model emits tools, the backend executed them
        // and returned the output as a separate message — but the model itself
        // never sees that output. Re-invoke stream_chat_message with the full
        // updated history so the model can react. Skipped when the user
        // cancelled, when the turn cap is reached, or when no tools ran.
        // The provider said this reply is cut short at the output cap, so the
        // work it was asked to do is not finished. Before this, that arrived as
        // a normal completion: the panel rendered a truncated answer as a
        // finished one and the user had to type "continue" to get the rest.
        const wasTruncated = response.stop_reason === "length";
        const canContinueTruncation =
          wasTruncated &&
          !cancelledRef.current &&
          truncationContinueRef.current < MAX_TRUNCATION_CONTINUES &&
          nextMessages !== null;

        const shouldAutoContinue =
          (hasToolOutput || canContinueTruncation) &&
          !cancelledRef.current &&
          agentTurnCountRef.current < MAX_AGENT_TURNS - 1 &&
          nextMessages !== null;

        if (shouldAutoContinue && nextMessages) {
          agentTurnCountRef.current += 1;
          const turn = agentTurnCountRef.current;
          // Reset stream-progress UI but keep isLoading=true so the user
          // sees one continuous "thinking" state across the loop.
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setRetryInfo(null);
          streamStartMsRef.current = null;
          streamCharsRef.current = 0;
          if (canContinueTruncation) {
            truncationContinueRef.current += 1;
            setStreamStatus(
              `Continuing — the reply hit the model's output cap `
                + `(${truncationContinueRef.current}/${MAX_TRUNCATION_CONTINUES})`,
            );
          } else {
            setStreamStatus(`Continuing (${turn}/${MAX_AGENT_TURNS - 1})`);
          }

          const backendMessages = toBackendMessages(nextMessages);
          const effectiveContext =
            [pinnedMemoryRef.current, contextRef.current].filter(Boolean).join("\n\n") || null;
          const chatRequest = {
            messages: backendMessages,
            provider: providerRef.current,
            context: effectiveContext,
            file_tree: fileTreeRef.current ?? null,
            current_file: currentFileRef.current ?? null,
            mode: backendModeRef.current ?? null,
            attachments: [],
            session_id: sessionIdRef.current ?? null,
            session_title: sessionTitleRef.current ?? null,
            effort: getSelectedEffort(),
          };
          invoke("stream_chat_message", { request: chatRequest }).catch((err) => {
            console.error("[AIChat] auto-continue invoke failed:", err);
            agentTurnCountRef.current = 0;
    truncationContinueRef.current = 0;
            setStreamStatus(null);
            setIsLoading(false);
          });
          return;
        }

        // Terminal completion — clear loading state. An empty turn here means
        // the run stopped without producing anything; say so rather than
        // leaving the user with a chat that just went quiet.
        if (isEmptyTurn) {
          const midLoop = agentTurnCountRef.current > 0;
          setMessages((prev) => [...prev, {
            role: "assistant",
            content: midLoop
              ? "The model returned an empty response while continuing after tool results, so the run stopped. Send \"continue\" to resume."
              : "The model returned an empty response. Try resending, or switch models.",
            timestamp: Date.now(),
            isError: true,
          }]);
        }
        // Still truncated at the terminal completion means the continue budget
        // ran out. Say so, with the reason and the way forward -- the failure
        // this whole change exists to stop is the chat going quiet mid-answer
        // and leaving the user to guess why.
        if (wasTruncated && !shouldAutoContinue && !cancelledRef.current) {
          const spent = truncationContinueRef.current;
          setMessages((prev) => [...prev, {
            role: "assistant",
            content: spent >= MAX_TRUNCATION_CONTINUES
              ? `This reply is still incomplete: the model stopped at its output cap `
                + `${spent} more time${spent === 1 ? "" : "s"} after `
                + `${MAX_TRUNCATION_CONTINUES} automatic continuations, so the panel `
                + `stopped rather than keep spending tokens. Send "continue" to go further, `
                + `or raise the model's output cap in Settings → Harness.`
              : `This reply was cut off at the model's output cap and could not be `
                + `continued automatically. Send "continue" to resume.`,
            timestamp: Date.now(),
            isError: true,
          }]);
        }
        agentTurnCountRef.current = 0;
    truncationContinueRef.current = 0;
        truncationContinueRef.current = 0;
        if (onMessagesChangeRef.current) {
          // Controlled mode: defer clearing streaming UI until the new
          // `messages` prop propagates back, otherwise the response visually
          // disappears for one frame. The useEffect on `messages` does it.
          pendingClearRef.current += 1;
        } else {
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }
      });
      if (cancelled) { u2(); return; }
      unlisteners.push(u2);

      // chat:error
      const u3 = await listen<string>("chat:error", (e) => {
        let errorContent = e.payload;
        // Improve common error messages with actionable guidance
        if (errorContent.includes("Load failed") || errorContent.includes("connection") || errorContent.includes("ECONNREFUSED")) {
          errorContent += "\n\nThe AI provider may not be running. Check that Ollama (`ollama serve`) or your configured provider is reachable.";
        } else if (errorContent.includes("401") || errorContent.includes("Unauthorized") || errorContent.includes("invalid_api_key")) {
          errorContent += "\n\nYour API key may be invalid or expired. Check Settings to update it.";
        } else if (errorContent.includes("429") || errorContent.includes("rate limit")) {
          errorContent += "\n\nRate limited — wait a moment and try again, or switch providers.";
        }
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: errorContent,
          timestamp: Date.now(),
          isError: true,
        }]);
        agentTurnCountRef.current = 0;
    truncationContinueRef.current = 0;
        if (onMessagesChangeRef.current) {
          pendingClearRef.current += 1;
        } else {
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }
      });
      if (cancelled) { u3(); return; }
      unlisteners.push(u3);

      // chat:status — retry, thinking, provider_health, tool_call_rejected
      const u4 = await listen<{ type: string; attempt?: number; max_retries?: number; score?: number; message?: string; reason?: string; tool?: string }>("chat:status", (e) => {
        const payload = e.payload;
        if (payload.type === "retry" && payload.attempt != null && payload.max_retries != null) {
          // Backend clears its accumulator on retry — reset frontend to match
          // so the final message won't be shorter than what was streaming.
          setStreamingText("");
          streamStartMsRef.current = null;
          streamCharsRef.current = 0;
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setRetryInfo({ attempt: payload.attempt, max: payload.max_retries });
          setStreamStatus(`Retrying (${payload.attempt}/${payload.max_retries})...`);
        } else if (payload.type === "thinking") {
          setStreamStatus("Thinking...");
        } else if (payload.type === "provider_health" && payload.score != null) {
          setProviderHealth(payload.score);
        } else if (payload.type === "tool_call_rejected") {
          // The backend refused to act on a tool tag — most often a `path`
          // that cannot be a filename, because the model emitted a malformed
          // tag. Without this the file simply never appears and there is
          // nothing anywhere saying why, which is the failure the rejection
          // was added to replace.
          //
          // Collected rather than shown one by one: a model that emits one
          // malformed tag usually emits several, and the reason is the same
          // every time. They are reported once, when the turn ends.
          const reason = payload.reason ?? "no reason reported";
          const line = `${payload.tool ?? "tool"}: ${reason}`;
          if (!toolRejectionsRef.current.includes(line)) {
            toolRejectionsRef.current = [...toolRejectionsRef.current, line].slice(
              0,
              MAX_REPORTED_TOOL_REJECTIONS,
            );
          }
        }
      });
      if (cancelled) { u4(); return; }
      unlisteners.push(u4);

      // chat:metrics — token/cost data
      const u5 = await listen<MessageMetrics>("chat:metrics", (e) => {
        const metrics = e.payload;
        setMessages((prev) => {
          if (prev.length === 0) return prev;
          const last = prev[prev.length - 1];
          if (last.role !== "assistant") return prev;
          const updated = [...prev];
          updated[updated.length - 1] = { ...last, metrics };
          return updated;
        });
      });
      if (cancelled) { u5(); return; }
      unlisteners.push(u5);

      // Receive file writes from agent/sandbox tool execution and forward to
      // the workspace explorer via the onPendingWrite callback.
      const u6 = await listen<{ path: string; content: string }>("file:written", (e) => {
        onPendingWriteRef.current?.(e.payload.path, e.payload.content);
      });
      if (cancelled) { u6(); return; }
      unlisteners.push(u6);

      // ── Agent loop listeners ────────────────────────────────────────────
      // When this tab has a sessionId, the backend emits per-tab event names
      // like `agent:{sessionId}:chunk` (see start_agent_task in commands.rs).
      // We listen on those scoped names so two tabs running agents in
      // parallel never see each other's events. The agentRunOwnerRef gate
      // only applies when sessionId is absent (legacy global event names),
      // because per-tab scoping already guarantees event ownership.
      const agentEvent = (base: string) =>
        sessionIdRef.current ? `agent:${sessionIdRef.current}:${base}` : `agent:${base}`;

      // agent:chunk — incremental LLM token stream during planning / synthesis.
      // Reuse streamingText so the UI shows the same typing cursor as chat mode.
      const a1 = await listen<string>(agentEvent("chunk"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        const now = Date.now();
        const chunk = e.payload;
        if (streamStartMsRef.current === null) streamStartMsRef.current = now;
        streamCharsRef.current += chunk.length;
        const elapsedSec = (now - streamStartMsRef.current) / 1000;
        const approxTokens = Math.round(streamCharsRef.current / 4);
        if (elapsedSec > 0) {
          setTokensPerSec(Math.round(approxTokens / elapsedSec));
        }
        setStreamTokenCount(approxTokens);
        setStreamingText((prev) => prev + chunk);
      });
      if (cancelled) { a1(); return; }
      unlisteners.push(a1);

      // agent:step — tool execution finished. Push to agentSteps and reset
      // streaming so the next planning chunk renders fresh.
      const a2 = await listen<{
        step_num: number;
        tool_name: string;
        tool_summary: string;
        output: string;
        success: boolean;
        approved: boolean;
      }>(agentEvent("step"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        setAgentSteps((prev) => [...prev, e.payload]);
        setStreamingText("");
        setPendingApproval(null);
        streamStartMsRef.current = null;
        streamCharsRef.current = 0;
        setTokensPerSec(null);
        setStreamTokenCount(0);
      });
      if (cancelled) { a2(); return; }
      unlisteners.push(a2);

      // agent:pending — backend is asking for approval before a tool call.
      // Stop streaming and surface the approval banner above the input.
      const a3 = await listen<{ name: string; summary: string; is_destructive: boolean }>(agentEvent("pending"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        setStreamingText("");
        // "Approve all" was pressed earlier in this run — keep going without
        // stopping to ask again.
        if (autoApproveRef.current) {
          setStreamStatus(`Auto-approved: ${e.payload.name}`);
          invoke("respond_to_agent_approval", { approved: true }).catch((err) =>
            console.error("[AIChat] auto-approve failed:", err),
          );
          return;
        }
        setPendingApproval(e.payload);
        setStreamStatus(`Awaiting approval: ${e.payload.name}`);
      });
      if (cancelled) { a3(); return; }
      unlisteners.push(a3);

      // agent:complete — terminal success. Push the summary as an assistant
      // message and clear all agent / streaming state.
      const a4 = await listen<string>(agentEvent("complete"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        agentRunOwnerRef.current = false;
        // Reasoning models put a `<thinking>` block in the summary; show it in
        // the collapsible slot rather than inline with the answer.
        const [summary, thinkingText] = extractThinking(e.payload ?? "");
        // If the whole summary was reasoning, show it rather than a content-free
        // "Agent task complete." — collapsing it would leave the user with nothing.
        const hasSummary = summary.trim().length > 0;
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: hasSummary ? summary : thinkingText || "Agent task complete.",
          timestamp: Date.now(),
          thinking: hasSummary ? thinkingText || undefined : undefined,
        }]);
        setPendingApproval(null);
        setAgentSteps([]);
        // Session approval survives terminal runs. Once the user grants this
        // chat permission, later tasks in the same tab must not stop again.
        if (onMessagesChangeRef.current) {
          pendingClearRef.current += 1;
        } else {
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }
      });
      if (cancelled) { a4(); return; }
      unlisteners.push(a4);

      // agent:partial — terminal partial completion (turn cap, plan exhausted).
      // Render the partial summary as an assistant message.
      const a5 = await listen<{
        summary: string;
        steps_completed: number;
        steps_planned: number;
        remaining_plan: string[];
      }>(agentEvent("partial"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        agentRunOwnerRef.current = false;
        const { summary, steps_completed, steps_planned, remaining_plan } = e.payload;
        const [cleanSummary, thinkingText] = extractThinking(summary ?? "");
        const body =
          `⚠ Partial completion (${steps_completed}/${steps_planned} steps)\n\n${cleanSummary}` +
          (remaining_plan.length > 0
            ? `\n\nRemaining:\n${remaining_plan.map((s, i) => `  ${steps_completed + i + 1}. ${s}`).join("\n")}`
            : "");
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: body,
          timestamp: Date.now(),
          thinking: thinkingText || undefined,
        }]);
        setPendingApproval(null);
        setAgentSteps([]);
        // Keep the session-level approval grant across agent failures.
        if (onMessagesChangeRef.current) {
          pendingClearRef.current += 1;
        } else {
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }
      });
      if (cancelled) { a5(); return; }
      unlisteners.push(a5);

      // agent:error — terminal failure.
      const a6 = await listen<string>(agentEvent("error"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        agentRunOwnerRef.current = false;
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: `Agent error: ${e.payload}`,
          timestamp: Date.now(),
          isError: true,
        }]);
        setPendingApproval(null);
        setAgentSteps([]);
        // Keep the session-level approval grant across circuit breaks.
        if (onMessagesChangeRef.current) {
          pendingClearRef.current += 1;
        } else {
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }
      });
      if (cancelled) { a6(); return; }
      unlisteners.push(a6);

      // agent:retry — non-terminal; backend is retrying after a transient
      // failure. Update status bar.
      const a7 = await listen<{ error: string; attempt: number; max_attempts: number; backoff_ms: number }>(agentEvent("retry"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        const { error, attempt, max_attempts, backoff_ms } = e.payload;
        setRetryInfo({ attempt: attempt + 1, max: max_attempts });
        setStreamStatus(`Retrying (${attempt + 1}/${max_attempts}) in ${(backoff_ms / 1000).toFixed(1)}s — ${error}`);
      });
      if (cancelled) { a7(); return; }
      unlisteners.push(a7);

      // agent:verifier — verifier subagent reported PASS / NITS / FAIL on
      // the task_complete claim. Non-terminal: surfaced as a step card; the
      // backend has already injected nits / retry context into the next turn.
      const av = await listen<{ status: "pass" | "nits" | "fail"; message: string }>(agentEvent("verifier"), (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        setVerifierResult(e.payload);
      });
      if (cancelled) { av(); return; }
      unlisteners.push(av);

      // agent:circuit_break — the health monitor noticed something and acted.
      //
      // This is a *notice*, not the end of the run. The loop emits it when it
      // compacts context, when it retires a degrading agent and hands the work
      // to a fresh successor, and when it decides the history is already too
      // small to be the cause — and then it keeps going. Only `BLOCKED` is
      // terminal, and the Rust side reports that separately as `agent:error`
      // before it breaks.
      //
      // Treating every one of them as terminal — printing "Agent halted",
      // dropping ownership of the run, clearing the steps and the approval
      // state — meant the two mechanisms that exist to keep a long task alive
      // presented to the user as a crash, and the rest of the run, which was
      // still executing, went unwatched: further tool calls, approvals and the
      // final answer were all discarded by the `agentRunOwnerRef` guard.
      //
      // Payload is `{state, reason}` (see AgentEvent::CircuitBreak);
      // interpolating it whole printed "[object Object]".
      const a8 = await listen<{ state?: string; reason?: string } | string>(
        agentEvent("circuit_break"),
        (e) => {
        if (!sessionIdRef.current && !agentRunOwnerRef.current) return;
        const payload = e.payload;
        const state = typeof payload === "string" ? undefined : payload?.state;
        const detail =
          typeof payload === "string"
            ? payload
            : payload?.reason || "no reason reported";
        // PROGRESS means the harness recovered and the run continues; anything
        // else is a health warning that the run is still trying to work past.
        const recovered = state === "PROGRESS";
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: recovered
            ? detail
            : `Agent health: ${state ?? "unknown"} — ${detail}`,
          timestamp: Date.now(),
        }]);
      },
      );
      if (cancelled) { a8(); return; }
      unlisteners.push(a8);
    })();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [setMessages, setStreamingText, appendStreamChunk]);

  // Consume pendingInput from Cascade
  useEffect(() => {
    if (pendingInput) {
      setInput((prev) => prev ? `${prev}\n${pendingInput}` : pendingInput);
      onPendingInputConsumed?.();
      textareaRef.current?.focus();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingInput]);

  // Workspace changes only affect the file-tree context sent with future
  // messages — they should NOT interrupt an in-progress streaming response
  // or clear any chat state.  The previous implementation cleared messages
  // and streaming state here, which caused the "chat disappears on folder
  // open" bug.  All chat lifecycle is now managed by the event handlers
  // (chat:chunk, chat:complete, chat:error) and the parent (ChatTabManager).

  // ── Send message ─────────────────────────────────────────────────────────

  const sendMessage = useCallback(async (overrideInput?: string) => {
    const text = overrideInput ?? input;
    if (!text.trim() && attachments.length === 0) return;

    // The backend keeps a single pending-approval slot and a single agent
    // abort handle, so starting a second run while one waits for approval
    // orphans the first: its approval channel is replaced and it blocks
    // forever. Make the user resolve the pending call first.
    if (pendingApproval) {
      toast.warn(
        `The agent is waiting on approval for \`${pendingApproval.name}\`. Approve or reject it before sending.`,
      );
      return;
    }
    let messageText = text.trim() || (attachments.length > 0 ? `[Attached ${attachments.length} file(s) — please review]` : "");
    const submittedText = messageText;
    let startsGoalRun = false;

    // `/goal` is intentionally a command, not a route into a management UI.
    // The common path mirrors Codex/Claude-style durable work: state one
    // objective and start working. Bare/status, pause, resume and clear are
    // the complete control surface; the Goals panel remains available as an
    // advanced history/detail view.
    // Same hybrid for `/skills` typed straight into the composer. It takes no
    // argument — the catalogue is where the picking happens — so anything
    // after it is left alone and sent as an ordinary message.
    if (/^\/skills\s*$/i.test(messageText)) {
      openPanelTab("ai-ml/skills");
      setInput("");
      setSlashQuery(null);
      return;
    }

    const goalMatch = messageText.match(/^\/goal(?:\s+(.*))?$/i);
    if (goalMatch) {
      const arg = (goalMatch[1] ?? "").trim();
      const workspace = workspacePath || null;
      const appendGoalResult = (content: string, isError = false) => {
        setMessages((prev) => [
          ...prev,
          { role: "user", content: submittedText, timestamp: Date.now() },
          { role: "assistant", content, timestamp: Date.now(), isError },
        ]);
        setInput("");
        setAttachments([]);
        setSlashQuery(null);
      };
      try {
        const current = async () => (await invoke("exec_goal_current", { workspace })) as {
          goal_id?: string | null;
          goal?: { id: string; title: string; statement?: string; status: string };
        };
        if (!arg || arg === "status") {
          const found = await current();
          appendGoalResult(found.goal
            ? `Current goal: **${found.goal.title}** · ${found.goal.status}\n\n${found.goal.statement || ""}`.trim()
            : "No active goal. Start one with `/goal <objective>`." );
          return;
        }
        if (arg === "pause" || arg === "clear" || arg === "resume") {
          const found = await current();
          if (!found.goal) {
            appendGoalResult("No active goal. Start one with `/goal <objective>`.", true);
            return;
          }
          if (arg === "clear") {
            await invoke("exec_goal_unpin", { workspace });
            window.dispatchEvent(new CustomEvent("vibecoder:pin-changed"));
            appendGoalResult(`Cleared **${found.goal.title}** as the current goal.`);
            return;
          }
          await invoke("exec_goal_update", {
            id: found.goal.id,
            status: arg === "pause" ? "paused" : "active",
          });
          if (arg === "pause") {
            appendGoalResult(`Paused **${found.goal.title}**. Use \`/goal resume\` to continue.`);
            return;
          }
          messageText = found.goal.statement || found.goal.title;
          startsGoalRun = true;
        } else {
          const objective = arg;
          const created = (await invoke("exec_goal_create", {
            title: objective.slice(0, 120),
            statement: objective,
            workspace,
            successCriteria: [],
            tags: [],
            parentGoalId: null,
          })) as { id: string };
          await invoke("exec_goal_pin", { id: created.id, workspace });
          window.dispatchEvent(new CustomEvent("vibecoder:pin-changed"));
          messageText = objective;
          startsGoalRun = true;
        }
      } catch (error) {
        appendGoalResult(`Goal command failed: ${String(error)}`, true);
        return;
      }
    }

    if (!provider) {
      setMessages(prev => [...prev, {
        role: "assistant",
        content: "Please select an AI provider from the dropdown menu first.",
      }]);
      return;
    }

    // Capture current attachments and clear them
    const currentAttachments = [...attachments];
    const userMessage: Message = {
      role: "user",
      content: startsGoalRun ? submittedText : messageText,
      timestamp: Date.now(),
      attachments: currentAttachments.length > 0 ? currentAttachments : undefined,
    };
    // A rejection left over from a turn that errored or was stopped must not
    // be reported against this one.
    toolRejectionsRef.current = [];
    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setAttachments([]);
    setPickerQuery(null);
    setSlashQuery(null);
    setIsNearBottom(true);
    cancelledRef.current = false;
    agentTurnCountRef.current = 0;
    truncationContinueRef.current = 0;
    setIsLoading(true);
    setStreamingText("");
    setTokensPerSec(null);
    setStreamTokenCount(0);
    setStreamStatus(null);
    setRetryInfo(null);
    streamStartMsRef.current = null;
    streamCharsRef.current = 0;

    flowContext.add({
      kind: "chat",
      summary: userMessage.content.slice(0, 100),
      detail: `Q: ${userMessage.content}${currentAttachments.length > 0 ? ` [${currentAttachments.length} file(s)]` : ""}`,
    });

    // ── Agent-loop branch ────────────────────────────────────────────────
    // When the per-tab toggle is on, route through start_agent_task instead
    // of stream_chat_message. Each invocation is self-contained — no chat
    // history is plumbed through; the backend agent gets just `messageText`
    // as the task description. (Phase-1 limitation; Phase 3 will plumb
    // history.) Listeners (agent:chunk/step/pending/complete/...) update
    // the same streamingText / messages state used by the chat path.
    if (startsGoalRun || useAgentLoopRef.current) {
      setAgentSteps([]);
      setVerifierResult(null);
      setPendingApproval(null);
      // Approval is scoped to this mounted chat session/tab, not this run.
      // A grant remains active until the session is closed.
      agentRunOwnerRef.current = true;
      try {
        await invoke("start_agent_task", {
          task: messageText,
          approvalPolicy: approvalModeRef.current,
          provider,
          tabId: sessionId ?? null,
          effort: getSelectedEffort(),
        });
      } catch (error) {
        agentRunOwnerRef.current = false;
        const errStr = String(error);
        setMessages((prev) => [...prev, {
          role: "assistant",
          content: `Failed to start agent: ${errStr}`,
          isError: true,
        }]);
        setIsLoading(false);
      }
      return;
    }

    try {
      // Build request with only the fields the backend expects
      const backendMessages = toBackendMessages([...messages, userMessage]);
      const effectiveContext = [pinnedMemory, context].filter(Boolean).join("\n\n") || null;
      const chatRequest = {
        messages: backendMessages,
        provider,
        context: effectiveContext,
        file_tree: fileTree ?? null,
        current_file: currentFile ?? null,
        mode: backendMode ?? null,
        attachments: currentAttachments.map(({ name, mime_type, data, size, text_content }) => ({
          name, mime_type, data, size, text_content: text_content ?? null,
        })),
        session_id: sessionId ?? null,
        session_title: sessionTitle ?? null,
        effort: getSelectedEffort(),
      };
      console.warn("[AIChat] invoke stream_chat_message:", {
        provider,
        messageCount: backendMessages.length,
        contextLen: (context ?? "").length,
        fileTreeLen: (fileTree ?? []).length,
        attachmentCount: currentAttachments.length,
        payloadSize: JSON.stringify(chatRequest).length,
      });
      // Verify IPC works at all before the main call
      try {
        await invoke("get_workspace_folders");
        console.warn("[AIChat] IPC health check OK");
      } catch (ipcErr) {
        console.error("[AIChat] IPC health check FAILED:", ipcErr);
      }
      await invoke("stream_chat_message", { request: chatRequest });
    } catch (error) {
      console.error("Failed to start chat stream:", error);
      const errStr = String(error);
      let helpText: string;
      if (errStr.includes("Load failed") || errStr.includes("fetch") || errStr.includes("ECONNREFUSED")) {
        helpText = `Connection failed to **${provider}**. Make sure the provider is running and reachable.\n\n`
          + `- **Ollama**: run \`ollama serve\` (default: http://localhost:11434)\n`
          + `- **Cloud providers**: check your API key in Settings\n\n`
          + `Raw error: ${errStr}`;
      } else if (errStr.includes("Provider") && errStr.includes("not found")) {
        helpText = `Provider "${provider}" is not configured. Open Settings to add it.`;
      } else {
        helpText = `Error: ${errStr}\n\nMake sure an AI provider is configured and running.`;
      }
      setMessages((prev) => [...prev, {
        role: "assistant",
        content: helpText,
        isError: true,
      }]);
      setStreamingText("");
      setIsLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [input, provider, context, fileTree, currentFile, workspacePath, messages, backendMode, attachments, pendingApproval, toast]);

  const stopMessage = useCallback(async () => {
    cancelledRef.current = true;
    agentTurnCountRef.current = 0;
    truncationContinueRef.current = 0;
    // If this tab owns an agent run, abort it via stop_agent_task; otherwise
    // (or in addition) stop the chat stream. Both are best-effort.
    if (agentRunOwnerRef.current) {
      agentRunOwnerRef.current = false;
      await invoke("stop_agent_task").catch(() => {});
    } else {
      await invoke("stop_chat_stream").catch(() => {});
    }
    setMessages((prev) => {
      // The ref, not the state: the state lags by up to STREAM_FLUSH_MS, and
      // this is the one reader that must not lose the tail of what arrived.
      const live = streamAccumRef.current;
      if (live) {
        const [cleaned, thinking] = extractThinking(live);
        const [finalContent, toolCalls] = parseToolCalls(cleaned);
        return [...prev, {
          role: "assistant" as const,
          content: finalContent,
          thinking: thinking || undefined,
          toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
        }];
      }
      return prev;
    });
    setPendingApproval(null);
    setAgentSteps([]);
    // Stopping a task does not revoke the session's approval grant.
    setStreamingText("");
    setTokensPerSec(null);
    setStreamTokenCount(0);
    setStreamStatus(null);
    setRetryInfo(null);
    setIsLoading(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [streamingText]);

  // Retry: resend last user message
  const retryLastMessage = useCallback(() => {
    const lastUserMsg = findLast(messages, (m) => m.role === "user");
    if (lastUserMsg) {
      sendMessage(lastUserMsg.content);
    }
  }, [messages, sendMessage]);

  // ── Input handling ─────────────────────────────────────────────────────────

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);
    const cursor = e.target.selectionStart ?? val.length;

    // Check for @ context picker
    const atInfo = getAtQuery(val, cursor);
    setPickerQuery(atInfo ? atInfo.query : null);

    // Check for / slash commands
    if (val.startsWith("/") && !val.includes(" ")) {
      setSlashQuery(val);
    } else {
      setSlashQuery(null);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Bare `/goal` is itself meaningful (show current status), unlike prefix
    // commands such as `/fix`. Do not let the palette turn Enter into another
    // autocomplete step that merely inserts a trailing space.
    if (e.key === "Enter" && !e.shiftKey && input.trim().toLowerCase() === "/goal") {
      e.preventDefault();
      setSlashQuery(null);
      sendMessage();
      return;
    }
    // Let ContextPicker or SlashPalette handle navigation keys when visible
    if ((pickerQuery !== null || slashQuery !== null) && ["ArrowUp", "ArrowDown", "Enter", "Escape"].includes(e.key)) {
      e.preventDefault();
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
    if (e.key === " ") {
      setPickerQuery(null);
    }
    if (e.key === "Escape") {
      setSlashQuery(null);
      setPickerQuery(null);
    }
  };

  const handlePickerSelect = (insertion: string) => {
    if (!textareaRef.current) return;
    const cursor = textareaRef.current.selectionStart ?? input.length;
    const atInfo = getAtQuery(input, cursor);
    if (atInfo === null) return;

    const before = input.slice(0, atInfo.start);
    const after = input.slice(atInfo.start + 1 + atInfo.query.length);
    const newInput = before + insertion + " " + after;
    setInput(newInput);
    setPickerQuery(null);
    setTimeout(() => textareaRef.current?.focus(), 0);
  };

  const handleSlashSelect = (cmd: SlashCommand) => {
    if (cmd.action === "open-skills") {
      openPanelTab("ai-ml/skills");
      setInput("");
      setSlashQuery(null);
      return;
    }
    if (cmd.action === "switch-to-goals") {
      onSwitchToGoals?.();
      setInput("");
      setSlashQuery(null);
      return;
    }
    setInput(cmd.prefix ?? "");
    setSlashQuery(null);
    setTimeout(() => textareaRef.current?.focus(), 0);
  };

  const handleApplyCode = useCallback((code: string, filename: string) => {
    if (onPendingWriteRef.current) {
      onPendingWriteRef.current(filename, code);
    }
  }, []);

  /** Extract all fenced code blocks from a message as {language, code, filename}. */

  /**
   * Copy a message to the clipboard and flash the button.
   *
   * Stable so `ChatMessageRow`'s memo survives a streamed chunk — an inline
   * `onClick` closure per row would invalidate every bubble on every token.
   */
  const handleCopyMessage = useCallback((idx: number, content: string) => {
    navigator.clipboard
      .writeText(content)
      .then(() => {
        setCopiedIdx(idx);
        setTimeout(() => setCopiedIdx(null), 1500);
      })
      .catch(() => {});
  }, []);

  /** Queue of code blocks waiting to be applied one at a time. */
  const applyQueueRef = useRef<Array<{ filename: string; code: string }>>([]);
  /** Guard: prevent concurrent Apply All operations. */
  const applyBusyRef = useRef(false);

  /** Apply all code blocks from a message — queues them and opens the
   *  DiffReviewPanel for the first one. When the user accepts/rejects it,
   *  the next one in the queue is automatically opened. */
  const handleApplyAll = useCallback((content: string) => {
    if (!onPendingWriteRef.current) return;
    if (applyBusyRef.current) return; // already processing
    const blocks = extractCodeBlocks(content);
    if (blocks.length === 0) return;
    applyBusyRef.current = true;
    applyQueueRef.current = blocks.slice(1);
    onPendingWriteRef.current(blocks[0].filename, blocks[0].code);
  }, []);

  /** `undefined` disables the per-block Apply button; memoized so the rows
   *  keep their identity while a response streams. */
  const applyCodeHandler = useMemo(
    () => (onPendingWrite ? handleApplyCode : undefined),
    [onPendingWrite, handleApplyCode],
  );

  /** Listen for diff-resolved events to process the next queued file. */
  useEffect(() => {
    const onDiffResolved = () => {
      if (applyQueueRef.current.length > 0 && onPendingWriteRef.current) {
        const next = applyQueueRef.current.shift()!;
        // Small delay to let React commit the previous state change
        setTimeout(() => {
          onPendingWriteRef.current?.(next.filename, next.code);
        }, 100);
      } else {
        applyBusyRef.current = false;
      }
    };
    window.addEventListener("vibecoder:diff-resolved", onDiffResolved);
    return () => window.removeEventListener("vibecoder:diff-resolved", onDiffResolved);
  }, []);

  // ── Streaming content processing ───────────────────────────────────────────

  const streamingParts = useMemo(() => {
    if (!streamingText) return null;
    const [cleaned, thinking] = extractThinking(streamingText);
    return { cleaned: summarizeStreamingContent(cleaned), thinking };
  }, [streamingText]);

  /**
   * True when the live stream is showing prose that has already been committed
   * as a message.
   *
   * The finalized reply lands in `messages` while `streamingText` may still
   * hold the same text: in controlled mode the stream is not cleared until the
   * new `messages` prop propagates back from the parent, and anything that
   * stops that from arriving — or repopulates the stream afterwards — leaves
   * both on screen. The user then sees the identical answer twice, once with a
   * timestamp and once without.
   *
   * Rather than chase each way the two can desynchronise, the live bubble
   * yields to the committed message whenever they say the same thing. Exact
   * match only, so a partial response mid-stream (which never equals the
   * finished text) is unaffected.
   */
  const streamingAlreadyCommitted = useMemo(() => {
    const live = (streamingParts ? streamingParts.cleaned : streamingText).trim();
    if (!live) return false;
    // Walk backwards rather than `[...messages].reverse().find(...)`. This memo
    // recomputes on every stream flush, and that spelling copied and reversed
    // the entire transcript each time — allocation proportional to the whole
    // conversation, repeatedly, while a reply is arriving. The answer is
    // almost always in the last message or two.
    const lastAssistant = findLast(messages, (m) => m.role === "assistant" && !m.isToolOutput);
    return !!lastAssistant && lastAssistant.content.trim() === live;
  }, [messages, streamingParts, streamingText]);

  // ── The `+` menu ───────────────────────────────────────────────────────────

  // Progressive disclosure, from the VibeDesk composer. Attaching a file,
  // mentioning one, and the standing voice opt-in each had a permanent button
  // on a toolbar that has to fit inside a resizable sidebar; they are things
  // you reach for occasionally, so the toolbar keeps only what is touched every
  // turn. The bare `+` that typed an `@` character was the worst of them — a
  // button whose label described its glyph rather than what it did.
  const [drawerOpen, setDrawerOpen] = useState(false);
  const plusRef = useRef<HTMLDivElement>(null);
  const closeDrawer = useCallback(() => setDrawerOpen(false), []);
  useClickAway(drawerOpen, plusRef, closeDrawer);

  const insertMention = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    const v = input + "@";
    setInput(v);
    ta.focus();
    handleInputChange({
      target: { value: v, selectionStart: v.length },
    } as React.ChangeEvent<HTMLTextAreaElement>);
  }, [input]);

  const drawerGroups: ComposerGroup[] = useMemo(
    () => [
      {
        title: "Add to this message",
        items: [
          {
            id: "attach",
            icon: Paperclip,
            label: "Attach files",
            sub:
              attachments.length > 0
                ? `${attachments.length} attached — send file contents with the prompt`
                : "Send file contents with the prompt",
            onSelect: openFilePicker,
          },
          {
            id: "mention",
            icon: AtSign,
            label: "Mention a file",
            sub: "Point the model at something in the project",
            onSelect: insertMention,
          },
        ],
      },
      {
        title: "Voice",
        items: [
          {
            kind: "switch",
            id: "duplex",
            icon: AudioLines,
            label: "Voice conversation",
            on: voicePref.enabled,
            disabled: !duplex.supported,
            disabledHint: "This webview cannot capture audio",
            sub: {
              on: "On — start it from the toolbar",
              off: "Off — talk with the model, hands free",
            },
            onChange: (on) => {
              // Switching off must close the microphone, not merely hide the
              // control that was holding it open.
              if (!on) duplex.stop();
              voicePref.setEnabled(on);
            },
          },
        ],
      },
    ],
    [attachments.length, openFilePicker, insertMention, voicePref, duplex],
  );

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div
      className={`ai-chat${isDragOver ? " ai-chat-dragover" : ""}`}
      style={{ position: "relative" }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Drag overlay */}
      {isDragOver && (
        <div className="drag-overlay">
          <div className="drag-overlay-content">
            <Paperclip size={32} />
            <span>Drop files to attach</span>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="chat-header">
        <div className="chat-header-row">
          <div className="chat-header-left">
            <h3 style={{ margin: 0 }}>AI Assistant</h3>
            <HealthDot score={providerHealth} />
            {provider && <span className="chat-provider-label">{provider}</span>}
          </div>
          <div className="chat-header-actions">
            <label
              className="chat-action-btn"
              title={useAgentLoop
                ? "Agent mode is ON — the assistant can plan and run multiple steps (search, edit, run commands) per message, asking for your approval. Click to turn off."
                : "Agent mode is OFF — single reply per message. Click to turn on multi-step actions."}
              style={{
                display: "inline-flex", alignItems: "center", gap: 4,
                cursor: isLoading ? "not-allowed" : "pointer",
                opacity: isLoading ? 0.5 : 1,
                fontWeight: useAgentLoop ? 600 : 400,
                color: useAgentLoop ? "var(--accent-color)" : undefined,
                background: useAgentLoop ? "var(--accent-bg, rgba(96,165,250,0.15))" : undefined,
                borderColor: useAgentLoop ? "var(--accent-color)" : undefined,
                userSelect: "none",
              }}
            >
              <input
                type="checkbox"
                checked={useAgentLoop}
                disabled={isLoading || !onUseAgentLoopChange}
                onChange={(e) => onUseAgentLoopChange?.(e.target.checked)}
                style={{ margin: 0, cursor: "inherit" }}
                aria-label="Toggle agent mode (multi-step actions)"
              />
              Agent {useAgentLoop ? "ON" : "OFF"}
            </label>
            {/* Approval mode only governs agent runs, so it appears with the
                toggle. The backend reads the policy once at run start — hence
                disabled while a run is in flight. */}
            {useAgentLoop && (
              <select
                className="chat-action-btn"
                value={approvalMode}
                disabled={isLoading}
                onChange={(e) => setApprovalMode(e.target.value as ApprovalMode)}
                title={
                  APPROVAL_MODES.find((m) => m.value === approvalMode)?.hint ??
                  "How much the agent may do without asking"
                }
                aria-label="Agent approval mode"
                style={{ cursor: isLoading ? "not-allowed" : "pointer", opacity: isLoading ? 0.5 : 1 }}
              >
                {APPROVAL_MODES.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </select>
            )}
            {isLoading && (
              <button className="chat-action-btn chat-action-stop" onClick={stopMessage} title="Stop generation">
                Stop
              </button>
            )}
            {messages.length > 0 && !isLoading && (
              <button className="chat-action-btn" onClick={() => setMessages([])} title="Clear chat history">
                Clear
              </button>
            )}
          </div>
        </div>
        <p className="chat-subtitle">
          Ask questions about your code. Type <kbd>@</kbd> to inject context, <kbd>/</kbd> for commands. Click the mic for voice.
        </p>
      </div>

      {/* Messages */}
      <div className="chat-messages" ref={messagesContainerRef} onScroll={handleScroll} role="log" aria-live="polite" aria-label="Chat messages" style={{ position: "relative" }}>
        {messages.length === 0 ? (
          <div className="chat-empty">
            <div className="chat-empty-icon">{"</>"}</div>
            <p className="chat-empty-title">AI Coding Assistant</p>
            <p>Ask me anything about your code, or use <kbd>@file:path</kbd> and <kbd>@git</kbd> to inject context.</p>
            <div className="chat-empty-hints">
              <span className="chat-hint" onClick={() => setInput("/fix ")}>
                /fix
              </span>
              <span className="chat-hint" onClick={() => setInput("/explain ")}>
                /explain
              </span>
              <span className="chat-hint" onClick={() => setInput("/test ")}>
                /test
              </span>
              <span className="chat-hint" onClick={() => setInput("/review ")}>
                /review
              </span>
            </div>
          </div>
        ) : (
          messages.map((rawMsg, idx) => (
            <ChatMessageRow
              key={idx}
              rawMsg={rawMsg}
              idx={idx}
              isLast={idx === messages.length - 1}
              copied={copiedIdx === idx}
              canApply={!!onPendingWrite}
              onCopy={handleCopyMessage}
              onApplyCode={applyCodeHandler}
              onApplyAll={handleApplyAll}
              onRetry={retryLastMessage}
              onLightbox={setLightboxSrc}
            />
          ))
        )}

        {/* Agent steps — completed tool executions in the current run.
            Collapsed by default; the label count climbs live as steps land. */}
        {agentSteps.length > 0 && (
          <div className="message message-assistant">
            <div className="message-icon"><span className="assistant-icon">AI</span></div>
            <div className="message-content">
              <WorkSection label={`Work · ${agentSteps.length} step${agentSteps.length !== 1 ? "s" : ""}`}>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {agentSteps.map((step) => (
                  <div
                    key={step.step_num}
                    style={{
                      border: "1px solid var(--border-color, #2a2a2a)",
                      borderRadius: 6,
                      padding: "6px 10px",
                      fontSize: "0.85em",
                      background: step.success ? "rgba(60, 200, 120, 0.06)" : "rgba(220, 80, 80, 0.06)",
                    }}
                  >
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <span style={{ opacity: 0.6 }}>#{step.step_num}</span>
                      <strong>{step.tool_name}</strong>
                      <span style={{ opacity: 0.85 }}>{step.tool_summary}</span>
                      <span style={{ marginLeft: "auto", opacity: 0.6 }}>
                        {step.success ? "✓" : "✗"}
                      </span>
                    </div>
                    {step.output && (
                      <pre
                        style={{
                          margin: "4px 0 0",
                          padding: "4px 6px",
                          background: "rgba(0,0,0,0.25)",
                          borderRadius: 4,
                          maxHeight: 160,
                          overflow: "auto",
                          fontSize: "0.85em",
                          whiteSpace: "pre-wrap",
                        }}
                      >
                        {step.output.length > 800 ? step.output.slice(0, 800) + "\n…" : step.output}
                      </pre>
                    )}
                  </div>
                ))}
              </div>
              </WorkSection>
            </div>
          </div>
        )}

        {/* Verifier card — PASS / NITS / FAIL from the verifier subagent */}
        {verifierResult && (
          <div className="message message-assistant" data-testid="verifier-card">
            <div className="message-icon"><span className="assistant-icon">AI</span></div>
            <div className="message-content">
              <div
                style={{
                  border: `1px solid ${
                    verifierResult.status === "fail"
                      ? "rgba(220, 80, 80, 0.45)"
                      : verifierResult.status === "nits"
                        ? "rgba(220, 180, 60, 0.45)"
                        : "rgba(60, 200, 120, 0.45)"
                  }`,
                  borderRadius: 6,
                  padding: "6px 10px",
                  fontSize: "0.85em",
                  background:
                    verifierResult.status === "fail"
                      ? "rgba(220, 80, 80, 0.08)"
                      : verifierResult.status === "nits"
                        ? "rgba(220, 180, 60, 0.08)"
                        : "rgba(60, 200, 120, 0.08)",
                }}
              >
                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <strong>
                    Verifier:{" "}
                    {verifierResult.status === "pass"
                      ? "✅ PASS"
                      : verifierResult.status === "nits"
                        ? "📝 NITS"
                        : "❌ FAIL"}
                  </strong>
                </div>
                {verifierResult.message && (
                  <div style={{ marginTop: 4, opacity: 0.85, whiteSpace: "pre-wrap" }}>
                    {verifierResult.message}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Streaming message */}
        {isLoading && (
          <div className="message message-assistant">
            <div className="message-icon"><span className="assistant-icon">AI</span></div>
            <div className="message-content">
              {streamingText && !streamingAlreadyCommitted ? (
                <>
                  {/* Streaming thinking block — collapsed so in-progress
                      reasoning doesn't crowd the streaming response. */}
                  {streamingParts?.thinking && (
                    <WorkSection label="Work · thinking">
                      <ThinkingBlock text={streamingParts.thinking} />
                    </WorkSection>
                  )}

                  <div className="msg-rendered">
                    {/* `?? `, not `||`: a turn that is *only* reasoning leaves
                        `cleaned` empty, and falling back to the raw text would
                        print the <thinking> tags verbatim. */}
                    {renderContent(streamingParts ? streamingParts.cleaned : streamingText)}
                    <span className="streaming-cursor" />
                  </div>
                </>
              ) : (
                <div className="typing-indicator">
                  <span></span><span></span><span></span>
                </div>
              )}
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Scroll-to-bottom button */}
      {!isNearBottom && (
        <button
          className="scroll-to-bottom"
          onClick={scrollToBottom}
          title="Scroll to bottom"
          aria-label="Scroll to bottom"
        >
          &#8595;
        </button>
      )}

      {/* Streaming status bar */}
      {isLoading && (streamStatus || tokensPerSec !== null) && (
        <div className="stream-status-bar">
          {streamStatus && <span className="stream-status-text">{streamStatus}</span>}
          {retryInfo && (
            <span className="stream-retry-badge">Attempt {retryInfo.attempt}/{retryInfo.max}</span>
          )}
          <div style={{ flex: 1 }} />
          {tokensPerSec !== null && (
            <span className="stream-metrics">
              {streamTokenCount} tokens &middot; {tokensPerSec} tok/s
              {provider && <> &middot; {provider}</>}
            </span>
          )}
        </div>
      )}

      {/* Agent approval banner — visible only when agent:pending fired */}
      {pendingApproval && (
        <div
          role="alertdialog"
          aria-label="Tool approval required"
          style={{
            margin: "8px 12px",
            padding: "10px 12px",
            border: `1px solid ${pendingApproval.is_destructive ? "#d63b3b" : "#d6a83b"}`,
            borderRadius: 8,
            background: pendingApproval.is_destructive ? "rgba(214, 59, 59, 0.08)" : "rgba(214, 168, 59, 0.08)",
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <strong style={{ color: pendingApproval.is_destructive ? "#d63b3b" : "#d6a83b" }}>
              {pendingApproval.is_destructive ? "⚠ Destructive tool — approval required" : "Tool approval required"}
            </strong>
            <span style={{ marginLeft: "auto", opacity: 0.7, fontSize: "0.85em" }}>
              {pendingApproval.name}
            </span>
          </div>
          <div style={{ fontSize: "0.9em", whiteSpace: "pre-wrap" }}>
            {pendingApproval.summary}
          </div>
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", alignItems: "center" }}>
            <button
              className="chat-action-btn"
              title="Approve this call and all later changes in this chat session"
              onClick={async () => {
                setAutoApprove(true);
                try {
                  await invoke("respond_to_agent_approval", { approved: true });
                } catch (e) {
                  console.error("[AIChat] approve-all failed:", e);
                }
                setPendingApproval(null);
              }}
              style={{ marginRight: "auto" }}
            >
              Approve changes for this session
            </button>
            <button
              className="chat-action-btn"
              onClick={async () => {
                try {
                  await invoke("respond_to_agent_approval", { approved: false });
                } catch (e) {
                  console.error("[AIChat] reject agent approval failed:", e);
                }
                setPendingApproval(null);
              }}
            >
              Reject
            </button>
            <button
              className="chat-action-btn"
              style={{
                background: pendingApproval.is_destructive ? "#d63b3b" : "var(--accent-color, #3b82f6)",
                color: "#fff",
              }}
              onClick={async () => {
                try {
                  await invoke("respond_to_agent_approval", { approved: true });
                } catch (e) {
                  console.error("[AIChat] approve agent approval failed:", e);
                }
                setPendingApproval(null);
              }}
            >
              Approve
            </button>
          </div>
        </div>
      )}

      {/* Input area */}
      <div className="chat-input-card" style={{ position: "relative" }}>
        {pickerQuery !== null && (
          <ContextPicker
            query={pickerQuery}
            onSelect={handlePickerSelect}
            onClose={() => setPickerQuery(null)}
          />
        )}
        {slashQuery !== null && (
          <SlashPalette
            query={slashQuery}
            onSelect={handleSlashSelect}
            onClose={() => setSlashQuery(null)}
          />
        )}
        {isListening && interimText && (
          <div className="voice-interim">
            <span className="voice-interim-dot" />
            {interimText}
          </div>
        )}
        {/* The spoken turn as it happens. The chat log above gets each turn
            once it is complete (`onTurn`); this is the seconds in between,
            which previously showed nothing at all. */}
        <VoiceApproval approval={duplex.approval} onRespond={duplex.respondToApproval} />
      <VoiceTranscript
        state={duplex.state}
        turns={duplex.turns}
        activity={duplex.activity}
        active={duplex.active}
      />
        {/* Loading indicator for file reading */}
        {isAttachLoading && (
          <div className="attachment-loading">
            <Loader2 size={14} className="attachment-spinner" />
            <span>Reading files...</span>
          </div>
        )}
        {/* Attachment preview strip */}
        {attachments.length > 0 && (
          <div className="attachment-strip">
            <div className="attachment-strip-header">
              <Paperclip size={12} />
              <span>{attachments.length} file{attachments.length > 1 ? "s" : ""} attached</span>
              <button className="attachment-clear-all" onClick={() => setAttachments([])} title="Remove all">
                Clear all
              </button>
            </div>
            <div className="attachment-chips">
              {attachments.map((att, i) => {
                const isImage = att.mime_type.startsWith("image/");
                const hasText = !!att.text_content;
                const sizeStr = att.size < 1024 ? `${att.size} B`
                  : att.size < 1024 * 1024 ? `${(att.size / 1024).toFixed(1)} KB`
                  : `${(att.size / (1024 * 1024)).toFixed(1)} MB`;

                return (
                  <div key={i} className={`attachment-chip ${isImage ? "attachment-chip-image" : "attachment-chip-doc"}`}>
                    {isImage && att.previewUrl ? (
                      <img src={att.previewUrl} alt={att.name} className="attachment-thumb" />
                    ) : (
                      <FileText size={14} className="attachment-file-icon" />
                    )}
                    <div className="attachment-info">
                      <span className="attachment-name" title={att.name}>
                        {att.name.length > 25 ? att.name.slice(0, 22) + "..." : att.name}
                      </span>
                      <span className="attachment-meta">
                        {sizeStr}
                        {hasText && " \u00B7 text"}
                        {isImage && " \u00B7 image"}
                      </span>
                    </div>
                    <button className="attachment-remove" onClick={() => removeAttachment(i)} title="Remove">
                      <X size={12} />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
        <textarea
          ref={textareaRef}
          value={input}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder={isListening ? "Listening\u2026" : "Ask anything \u2014 @ for a file, / for commands"}
          rows={3}
        />
        {/* Hidden file input for fallback */}
        <input
          ref={fileInputRef}
          type="file"
          multiple
          style={{ display: "none" }}
          onChange={(e) => { if (e.target.files) addFiles(e.target.files); e.target.value = ""; }}
        />
        <div className="chat-input-toolbar">
          {/* One `+`, not three buttons. What is behind it is occasional; what
              stays on the bar is what you touch every turn. */}
          <div className="vxc-pop" ref={plusRef}>
            {drawerOpen && (
              <ComposerDrawer
                groups={drawerGroups}
                onClose={closeDrawer}
                label="Attach files, mention a file, or turn on voice"
              />
            )}
            <button
              className="chat-toolbar-btn"
              aria-label="Attach files, mention a file, or turn on voice"
              title="Attach files, mention a file, or turn on voice"
              aria-expanded={drawerOpen}
              onClick={() => setDrawerOpen((v) => !v)}
            >
              <Plus size={15} strokeWidth={1.5} />
              {attachments.length > 0 && (
                <span className="attach-badge">{attachments.length}</span>
              )}
            </button>
          </div>

          {/* Agent mode selector */}
          <div className="mode-selector">
            <button
              className={`mode-btn ${agentMode === "fast" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("fast")}
              title="Fast — Quick answers, less context"
              aria-label="Fast"
              aria-pressed={agentMode === "fast"}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
              <span>Fast</span>
            </button>
            <button
              className={`mode-btn ${agentMode === "chat" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("chat")}
              title="Balanced — Default, good context"
              aria-label="Balanced"
              aria-pressed={agentMode === "chat"}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><path d="M8 12h8M12 8v8"/></svg>
              <span>Balanced</span>
            </button>
            <button
              className={`mode-btn ${agentMode === "planning" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("planning")}
              title="Thorough — Deep analysis, max context"
              aria-label="Thorough"
              aria-pressed={agentMode === "planning"}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3"/><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/></svg>
              <span>Thorough</span>
            </button>
          </div>

          <div className="chat-toolbar-spacer" />

          {/* Voice, then send. One group so a narrow pane wraps them together
              rather than leaving the send button alone on a second line — or,
              as it did before, clipped out of the card entirely. */}
          <div className="chat-toolbar-right">
          {/* Voice button */}
          <button
            onClick={toggleVoice}
            title={isTranscribing ? "Transcribing..." : isListening ? "Click to stop" : "Voice input"}
            className={`chat-toolbar-btn mic-icon${isListening ? " listening" : ""}${isTranscribing ? " transcribing" : ""}`}
            disabled={isTranscribing}
            aria-label={isListening ? "Stop voice recording" : "Start voice input"}
          >
            <Mic size={14} strokeWidth={1.5} />
            {isListening && <span className="mic-recording-badge">REC</span>}
          </button>

          {/* Full-duplex conversation — an open mic, interruptible. The
              toolbar carries the live start/stop; the opt-in that decides
              whether this appears at all lives behind the `+`, so a feature
              nobody has turned on costs no space here. */}
          {voicePref.enabled && (
            <DuplexVoiceButton
              compact
              state={duplex.state}
              enabled={voicePref.enabled}
              onEnabledChange={voicePref.setEnabled}
              active={duplex.active}
              supported={duplex.supported}
              onStart={duplex.start}
              onStop={duplex.stop}
              unsupportedHint="This webview cannot capture audio"
            />
          )}

          {/* Send button */}
          <button
            className="chat-toolbar-send"
            onClick={() => sendMessage()}
            disabled={(!input.trim() && attachments.length === 0) || isLoading}
            aria-label="Send message"
            title="Send (Enter)"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
          </button>
          </div>
        </div>
      </div>

      {/* The keyboard contract, quiet and outside the frame. It used to be the
          tail of the placeholder, where a narrow sidebar clipped it — and a
          placeholder disappears the moment you start typing, which is exactly
          when you might want to know what Enter does. */}
      <div className="chat-input-hint">Enter sends · &#8679;Enter for a new line</div>

      {/* Image lightbox overlay */}
      {lightboxSrc && (
        <div className="lightbox-overlay" onClick={() => setLightboxSrc(null)}>
          <div className="lightbox-content" onClick={(e) => e.stopPropagation()}>
            <img src={lightboxSrc} alt="Full size preview" className="lightbox-image" />
            <div className="lightbox-actions">
              <a href={lightboxSrc} download="attachment" className="lightbox-download" title="Download">
                <Download size={16} /> Download
              </a>
              <button className="lightbox-close" onClick={() => setLightboxSrc(null)} title="Close">
                <X size={16} /> Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
