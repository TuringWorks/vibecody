import { useRef, useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useToast } from "../hooks/useToast";
import { ContextPicker } from "./ContextPicker";
import { flowContext } from "../utils/FlowContext";
import { Mic, User, Paperclip, X, FileText, Loader2, Download, ZoomIn } from "lucide-react";
import "./AIChat.css";

// ── Voice input hook ─────────────────────────────────────────────────────────
// Strategy 1: Web Speech API (webkitSpeechRecognition) — works natively in
//   Chromium-based webviews, no API key needed, real-time interim results.
// Strategy 2: MediaRecorder + Groq Whisper — fallback when SpeechRecognition
//   is unavailable (requires GROQ_API_KEY env var on the Tauri backend).

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;

function useVoiceInput(onTranscript: (text: string) => void) {
 const [isListening, setIsListening] = useState(false);
 const [isTranscribing, setIsTranscribing] = useState(false);
 const [interimText, setInterimText] = useState("");
 // eslint-disable-next-line @typescript-eslint/no-explicit-any
 const recognitionRef = useRef<any>(null);
 const recorderRef = useRef<MediaRecorder | null>(null);
 const chunksRef = useRef<Blob[]>([]);
 const { toast } = useToast();

 useEffect(() => {
   return () => {
     if (recognitionRef.current) {
       try { recognitionRef.current.abort(); } catch { /* ignore */ }
     }
   };
 }, []);

 const toggle = useCallback(async () => {
   if (isListening) {
     if (recognitionRef.current) {
       recognitionRef.current.stop();
     } else if (recorderRef.current) {
       recorderRef.current.stop();
     }
     return;
   }

   if (SpeechRecognition) {
     try {
       const recognition = new SpeechRecognition();
       recognition.continuous = true;
       recognition.interimResults = true;
       recognition.lang = "en-US";
       recognition.maxAlternatives = 1;

       let finalTranscript = "";

       recognition.onresult = (event: { resultIndex: number; results: { length: number; [i: number]: { isFinal: boolean; [j: number]: { transcript: string } } } }) => {
         let interim = "";
         for (let i = event.resultIndex; i < event.results.length; i++) {
           const result = event.results[i];
           if (result.isFinal) {
             finalTranscript += result[0].transcript;
             setInterimText("");
           } else {
             interim += result[0].transcript;
           }
         }
         if (interim) setInterimText(interim);
         if (finalTranscript) {
           onTranscript(finalTranscript);
           finalTranscript = "";
         }
       };

       recognition.onerror = (event: { error: string }) => {
         setIsListening(false);
         setInterimText("");
         recognitionRef.current = null;
         if (event.error === "not-allowed") {
           toast.warn("Microphone access denied. Check browser/system permissions.");
         } else if (event.error !== "aborted") {
           toast.error(`Speech recognition error: ${event.error}`);
         }
       };

       recognition.onend = () => {
         setIsListening(false);
         setInterimText("");
         recognitionRef.current = null;
       };

       recognition.start();
       recognitionRef.current = recognition;
       setIsListening(true);
     } catch (e) {
       toast.error(`Speech recognition failed to start: ${e}`);
     }
     return;
   }

   // Fallback: MediaRecorder + Groq Whisper
   try {
     const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
     const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
       ? "audio/webm;codecs=opus"
       : "audio/webm";
     const recorder = new MediaRecorder(stream, { mimeType });
     chunksRef.current = [];

     recorder.ondataavailable = (e) => {
       if (e.data.size > 0) chunksRef.current.push(e.data);
     };

     recorder.onstop = async () => {
       stream.getTracks().forEach((t) => t.stop());
       setIsListening(false);

       const blob = new Blob(chunksRef.current, { type: mimeType });
       if (blob.size < 100) return;

       setIsTranscribing(true);
       try {
         const arrayBuf = await blob.arrayBuffer();
         const bytes = new Uint8Array(arrayBuf);
         let binary = "";
         for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
         const base64 = btoa(binary);

         const text = await invoke<string>("transcribe_audio_bytes", {
           audioBase64: base64,
           mimeType: mimeType.split(";")[0],
         });
         if (text.trim()) onTranscript(text);
       } catch (e) {
         const msg = String(e);
         if (msg.includes("GROQ_API_KEY")) {
           toast.warn("Set GROQ_API_KEY env var for voice input (Groq Whisper).");
         } else {
           toast.error(`Transcription failed: ${msg}`);
         }
       }
       setIsTranscribing(false);
     };

     recorder.onerror = () => {
       stream.getTracks().forEach((t) => t.stop());
       setIsListening(false);
       toast.error("Microphone recording failed.");
     };

     recorder.start();
     recorderRef.current = recorder;
     setIsListening(true);
   } catch (e) {
     toast.warn(`Microphone access denied: ${e}`);
   }
 }, [isListening, onTranscript, toast]);

 return { isListening, isTranscribing, interimText, toggle };
}

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
}

interface AIChatProps {
 provider: string;
 context?: string;
 fileTree?: string[];
 currentFile?: string | null;
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
}

// ── Slash commands ───────────────────────────────────────────────────────────

interface SlashCommand {
  command: string;
  label: string;
  description: string;
  prefix: string;
}

const SLASH_COMMANDS: SlashCommand[] = [
  { command: "/fix",      label: "Fix",        description: "Fix errors in the current file",    prefix: "Fix the following errors:\n" },
  { command: "/explain",  label: "Explain",    description: "Explain selected code",             prefix: "Explain the following code in detail:\n" },
  { command: "/test",     label: "Test",       description: "Generate tests",                    prefix: "Generate comprehensive tests for:\n" },
  { command: "/doc",      label: "Doc",        description: "Generate documentation",            prefix: "Generate documentation for:\n" },
  { command: "/refactor", label: "Refactor",   description: "Refactor code",                     prefix: "Refactor the following code for better readability, performance, and maintainability:\n" },
  { command: "/review",   label: "Review",     description: "Code review",                       prefix: "Perform a thorough code review of:\n" },
  { command: "/compact",  label: "Compact",    description: "Summarize conversation",            prefix: "Summarize our conversation so far into key points and action items:\n" },
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

/** Extract <thinking>...</thinking> blocks from content. Returns [cleanedContent, thinkingText]. */
function extractThinking(content: string): [string, string] {
  const thinkingRegex = /<thinking>([\s\S]*?)<\/thinking>/g;
  let thinkingText = "";
  let match: RegExpExecArray | null;
  while ((match = thinkingRegex.exec(content)) !== null) {
    thinkingText += (thinkingText ? "\n" : "") + match[1].trim();
  }
  const cleaned = content.replace(thinkingRegex, "").trim();
  return [cleaned, thinkingText];
}

/** Parse tool XML tags from content into ToolCallInfo[], return cleaned content. */
function parseToolCalls(content: string): [string, ToolCallInfo[]] {
  const tools: ToolCallInfo[] = [];
  let cleaned = content;

  // <write_file path="...">...</write_file>
  cleaned = cleaned.replace(/<write_file path="([^"]+)">([\s\S]*?)<\/write_file>/g, (_m, path, output) => {
    tools.push({ tool: "write_file", path, status: "success", output: output.trim() });
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

/** Detect file paths in text and return segments. */
function parseFileReferences(text: string): Array<{ type: "text" | "file"; value: string }> {
  const fileRegex = /(?:^|\s)((?:\.{0,2}\/)?(?:[a-zA-Z0-9_-]+\/)*[a-zA-Z0-9_-]+\.[a-zA-Z0-9]{1,10})(?=\s|$|[),;:])/g;
  const segments: Array<{ type: "text" | "file"; value: string }> = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = fileRegex.exec(text)) !== null) {
    const filePath = match[1];
    const start = match.index + (match[0].length - filePath.length);
    if (start > lastIndex) {
      segments.push({ type: "text", value: text.slice(lastIndex, start) });
    }
    segments.push({ type: "file", value: filePath });
    lastIndex = start + filePath.length;
  }

  if (lastIndex < text.length) {
    segments.push({ type: "text", value: text.slice(lastIndex) });
  }

  return segments.length > 0 ? segments : [{ type: "text", value: text }];
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

/** Render message content: parse code blocks, file references, plain text. */
function renderContent(
  content: string,
  onApply?: (code: string, filename: string) => void,
): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  // Split by code fences: ```lang [filename]\n...\n```
  // Group 1: language, Group 2: optional filename on same line, Group 3: code
  const fenceRegex = /```(\w*)(?:[^\S\n]+(\S+))?\n([\s\S]*?)```/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let key = 0;

  while ((match = fenceRegex.exec(content)) !== null) {
    // Text before this code block
    if (match.index > lastIndex) {
      const textBefore = content.slice(lastIndex, match.index);
      parts.push(<TextSegment key={key++} text={textBefore} />);
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
          key={key++}
          language={unfinishedFence[1] || "text"}
          code={unfinishedFence[3]}
        />
      );
    } else {
      parts.push(<TextSegment key={key++} text={remaining} />);
    }
  }

  return parts;
}

/** Render a text segment with file path chips. */
function TextSegment({ text }: { text: string }) {
  const segments = parseFileReferences(text);
  if (segments.length === 1 && segments[0].type === "text") {
    return <pre className="msg-text">{text}</pre>;
  }
  return (
    <pre className="msg-text">
      {segments.map((seg, i) =>
        seg.type === "file" ? (
          <span key={i} className="file-chip" title={`Open ${seg.value}`}>
            {seg.value}
          </span>
        ) : (
          <span key={i}>{seg.value}</span>
        )
      )}
    </pre>
  );
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

// ── Tool call card ───────────────────────────────────────────────────────────

function ToolCallCard({ call }: { call: ToolCallInfo }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className={`tool-card tool-card-${call.status}`}>
      <div className="tool-card-header" onClick={() => call.output && setExpanded(!expanded)}>
        <span className="tool-card-icon"><ToolIcon tool={call.tool} /></span>
        <span className="tool-card-label">{toolLabel(call.tool, call.path)}</span>
        {call.path && <span className="tool-card-path" title={call.path}>{call.path}</span>}
        <span className="tool-card-status"><ToolStatusIcon status={call.status} /></span>
        {call.duration_ms != null && (
          <span className="tool-card-duration">{call.duration_ms}ms</span>
        )}
        {call.output && (
          <span className="tool-card-expand">{expanded ? "\u25BE" : "\u25B8"}</span>
        )}
      </div>
      {expanded && call.output && (
        <pre className="tool-card-output">{call.output}</pre>
      )}
    </div>
  );
}

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

export function AIChat({
  provider,
  context,
  fileTree,
  currentFile,
  onFileAction,
  onPendingWrite,
  pendingInput,
  onPendingInputConsumed,
  messages: controlledMessages,
  onMessagesChange,
  pinnedMemory,
}: AIChatProps) {
  const [agentMode, setAgentMode] = useState<AgentMode>("chat");
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

  // In controlled mode, multiple Tauri events can fire before React
  // re-renders (e.g. chat:complete then chat:metrics). Each call to
  // setMessages reads messagesRef.current for the "prev" value, but
  // that ref is only updated on render. Without tracking, the second
  // caller sees stale data and silently drops the first caller's update.
  //
  // pendingMessagesRef tracks the latest value we've sent to the parent,
  // so rapid-fire updaters always chain off the most recent state.
  const pendingMessagesRef = useRef<Message[] | null>(null);

  // Sync: once React renders with the new prop, clear the pending value.
  useEffect(() => {
    pendingMessagesRef.current = null;
  }, [messages]);

  const setMessages = useCallback((update: Message[] | ((prev: Message[]) => Message[])) => {
    if (onMessagesChangeRef.current) {
      // Use pending (most recent uncommitted) value if available,
      // otherwise fall back to the last rendered prop.
      const current = pendingMessagesRef.current ?? messagesRef.current;
      const next = typeof update === "function" ? update(current) : update;
      pendingMessagesRef.current = next;
      onMessagesChangeRef.current(next);
    } else {
      setLocalMessages(update as Parameters<typeof setLocalMessages>[0]);
    }
  }, []);

  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [streamingText, setStreamingText] = useState("");
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
  const [expandedThinking, setExpandedThinking] = useState<Record<number, boolean>>({});

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
  }, [messages]);

  // ── Auto-compaction ──────────────────────────────────────────────────────────
  // When the conversation grows beyond COMPACTION_THRESHOLD chars, automatically
  // summarise the older half and splice it into a single summary message.
  // Never fires while a response is in-flight.
  const COMPACTION_THRESHOLD = 80_000;
  const COMPACTION_KEEP_LAST = 20;
  const isCompactingRef = useRef(false);
  const lastCompactionLengthRef = useRef(0);

  useEffect(() => {
    if (isLoading) return;                          // never interrupt a stream
    if (isCompactingRef.current) return;
    if (messages.length < COMPACTION_KEEP_LAST + 2) return;
    // Require at least 10k new chars since last compaction to avoid re-triggering
    const totalChars = messages.reduce((s, m) => s + m.content.length, 0);
    if (totalChars < COMPACTION_THRESHOLD) return;
    if (totalChars - lastCompactionLengthRef.current < 10_000) return;

    isCompactingRef.current = true;
    lastCompactionLengthRef.current = totalChars;

    const toSummarise = messages.slice(0, messages.length - COMPACTION_KEEP_LAST);
    const kept = messages.slice(messages.length - COMPACTION_KEEP_LAST);

    const summaryPrompt = "Summarise the following conversation into a concise paragraph (max 300 words) preserving key facts, decisions, and any important code snippets mentioned:\n\n"
      + toSummarise.map((m) => `${m.role}: ${m.content}`).join("\n\n");

    invoke<{ message: string }>("summarise_messages", { content: summaryPrompt })
      .then((res) => {
        const summaryText = res?.message ?? toSummarise.map((m) => `${m.role}: ${m.content.slice(0, 120)}`).join(" | ");
        const summaryMsg: Message = {
          role: "assistant",
          content: `Conversation summary (earlier messages compacted):\n\n${summaryText}`,
          isSummary: true,
          timestamp: Date.now(),
        };
        setMessages([summaryMsg, ...kept]);
      })
      .catch(() => {
        // Backend command not available — do a simple truncation with a notice
        const summaryMsg: Message = {
          role: "assistant",
          content: `[Earlier ${toSummarise.length} messages were compacted to save context.]`,
          isSummary: true,
          timestamp: Date.now(),
        };
        setMessages([summaryMsg, ...kept]);
      })
      .finally(() => {
        isCompactingRef.current = false;
      });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages, isLoading]);

  const { isListening, isTranscribing, interimText, toggle: toggleVoice } = useVoiceInput((transcript) =>
    setInput((prev) => (prev ? prev + " " : "") + transcript)
  );

  const { toast } = useToast();

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
        const [cleanedContent, thinkingText] = extractThinking(response.message);
        const [finalContent, toolCalls] = parseToolCalls(cleanedContent);

        setMessages((prev) => {
          const msg: Message = {
            role: "assistant",
            content: finalContent,
            timestamp: Date.now(),
            thinking: thinkingText || undefined,
            toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
          };
          const updated = [...prev, msg];

          if (response.tool_output && response.tool_output.trim()) {
            updated.push({
              role: "assistant",
              content: "```\n" + response.tool_output.trim() + "\n```",
              timestamp: Date.now(),
            });
          }
          return updated;
        });

        // In controlled mode (ChatTabManager), setMessages updates the parent's
        // state which won't propagate back as a new `messages` prop until the
        // parent re-renders.  If we clear streaming state here, there is a
        // frame where both the streaming text AND the finalized message are
        // absent — causing the response to visually "disappear".
        //
        // Solution: signal that a clear is pending and let the useEffect on
        // `messages` handle the actual cleanup once the prop update arrives.
        if (onMessagesChangeRef.current) {
          pendingClearRef.current += 1;
        } else {
          // Uncontrolled mode — state is local, so clearing is safe immediately
          setStreamingText("");
          setTokensPerSec(null);
          setStreamTokenCount(0);
          setStreamStatus(null);
          setRetryInfo(null);
          setIsLoading(false);
        }

        if (response.pending_write && onPendingWriteRef.current) {
          onPendingWriteRef.current(response.pending_write.path, response.pending_write.content);
        }
        // If the backend wrote files (tool_output mentions "Wrote file"), refresh
        // the explorer so new/modified files appear immediately.
        if (response.tool_output && /Wrote file/i.test(response.tool_output)) {
          window.dispatchEvent(new Event("vibeui:refresh-files"));
        }
        if (onFileActionRef.current) onFileActionRef.current();
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

      // chat:status — retry, thinking, provider_health
      const u4 = await listen<{ type: string; attempt?: number; max_retries?: number; score?: number; message?: string }>("chat:status", (e) => {
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
    })();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [setMessages]);

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
    const messageText = text.trim() || (attachments.length > 0 ? `[Attached ${attachments.length} file(s) — please review]` : "");
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
      content: messageText,
      timestamp: Date.now(),
      attachments: currentAttachments.length > 0 ? currentAttachments : undefined,
    };
    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setAttachments([]);
    setPickerQuery(null);
    setSlashQuery(null);
    setIsNearBottom(true);
    cancelledRef.current = false;
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

    try {
      // Build request with only the fields the backend expects
      const backendMessages = [...messages, userMessage].map(({ role, content }) => ({
        role,
        content,
      }));
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
      };
      console.log("[AIChat] invoke stream_chat_message:", {
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
        console.log("[AIChat] IPC health check OK");
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
  }, [input, provider, context, fileTree, currentFile, messages, backendMode, attachments]);

  const stopMessage = useCallback(async () => {
    cancelledRef.current = true;
    await invoke("stop_chat_stream").catch(() => {});
    setMessages((prev) => {
      if (streamingText) {
        const [cleaned, thinking] = extractThinking(streamingText);
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
    const lastUserMsg = [...messages].reverse().find((m) => m.role === "user");
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
    setInput(cmd.prefix);
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
   * Extract fenced code blocks that have an EXPLICIT filename in the fence info
   * string (e.g. ```typescript src/App.tsx). Language-only blocks are excluded
   * because they cannot be safely applied without a known target path.
   */
  const extractCodeBlocks = useCallback((content: string) => {
    const blocks: { language: string; code: string; filename: string }[] = [];
    // Group 1: language, Group 2: explicit filename token, Group 3: code
    const fenceRegex = /```(\w*)(?:[^\S\n]+(\S+))?\n([\s\S]*?)```/g;
    let match: RegExpExecArray | null;
    while ((match = fenceRegex.exec(content)) !== null) {
      if (!match[2]) continue; // skip language-only blocks — no safe target path
      blocks.push({ language: match[1], code: match[3], filename: match[2] });
    }
    return blocks;
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
  }, [extractCodeBlocks]);

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
    window.addEventListener("vibeui:diff-resolved", onDiffResolved);
    return () => window.removeEventListener("vibeui:diff-resolved", onDiffResolved);
  }, []);

  // ── Streaming content processing ───────────────────────────────────────────

  const streamingParts = useMemo(() => {
    if (!streamingText) return null;
    const [cleaned, thinking] = extractThinking(streamingText);
    return { cleaned, thinking };
  }, [streamingText]);

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
          messages.map((msg, idx) => (
            <div key={idx}>
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
                {/* Thinking block */}
                {msg.thinking && (
                  <div className="thinking-block">
                    <button
                      className="thinking-toggle"
                      onClick={() => setExpandedThinking((prev) => ({ ...prev, [idx]: !prev[idx] }))}
                    >
                      <span className="thinking-icon">{expandedThinking[idx] ? "\u25BE" : "\u25B8"}</span>
                      <span className="thinking-label">Thinking...</span>
                    </button>
                    {expandedThinking[idx] && (
                      <div className="thinking-content">
                        <pre>{msg.thinking}</pre>
                      </div>
                    )}
                  </div>
                )}

                {/* Tool call cards */}
                {msg.toolCalls && msg.toolCalls.length > 0 && (
                  <div className="tool-cards">
                    {msg.toolCalls.map((tc, ti) => (
                      <ToolCallCard key={ti} call={tc} />
                    ))}
                  </div>
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
                                onClick={() => imgSrc && setLightboxSrc(imgSrc)}
                                title="Click to enlarge"
                              />
                              <div className="msg-attachment-image-actions">
                                <span className="msg-attachment-name">{att.name}</span>
                                <button className="msg-attachment-zoom" onClick={() => imgSrc && setLightboxSrc(imgSrc)} title="View full size">
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
                    {renderContent(msg.content, onPendingWrite ? handleApplyCode : undefined)}
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
                        navigator.clipboard.writeText(msg.content).then(() => {
                          setCopiedIdx(idx);
                          setTimeout(() => setCopiedIdx(null), 1500);
                        }).catch(() => {});
                      }}
                      title="Copy response"
                    >
                      {copiedIdx === idx ? "\u2713 Copied" : "Copy"}
                    </button>
                    {!!onPendingWrite && extractCodeBlocks(msg.content).length > 1 && (
                      <button
                        className="msg-apply-all-btn"
                        onClick={() => handleApplyAll(msg.content)}
                        title="Apply all explicitly-named code blocks to their target files"
                      >
                        Apply All ({extractCodeBlocks(msg.content).length} files)
                      </button>
                    )}
                  </div>
                )}

                {/* Error retry button */}
                {msg.isError && idx === messages.length - 1 && (
                  <button className="msg-retry-btn" onClick={retryLastMessage} title="Retry last message">
                    Retry
                  </button>
                )}

                {/* Metrics badge */}
                {msg.metrics && <MetricsBadge metrics={msg.metrics} />}
              </div>
            </div>
            </div>
          ))
        )}

        {/* Streaming message */}
        {isLoading && (
          <div className="message message-assistant">
            <div className="message-icon"><span className="assistant-icon">AI</span></div>
            <div className="message-content">
              {streamingText ? (
                <>
                  {/* Streaming thinking block */}
                  {streamingParts?.thinking && (
                    <ThinkingBlock text={streamingParts.thinking} />
                  )}

                  <div className="msg-rendered">
                    {renderContent(streamingParts?.cleaned || streamingText)}
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
          placeholder={isListening ? "Listening\u2026" : "Ask anything, @ to mention, / for commands. Drop files or paste images."}
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
          {/* Context button */}
          <button
            className="chat-toolbar-btn"
            title="Add context (@file, @web, @git)"
            onClick={() => {
              const ta = textareaRef.current;
              if (ta) {
                const v = input + "@";
                setInput(v);
                ta.focus();
                handleInputChange({ target: { value: v, selectionStart: v.length } } as React.ChangeEvent<HTMLTextAreaElement>);
              }
            }}
          >+</button>

          {/* Attach files button */}
          <button
            className="chat-toolbar-btn"
            title="Attach files, images, or documents"
            onClick={openFilePicker}
          >
            <Paperclip size={14} strokeWidth={1.5} />
            {attachments.length > 0 && (
              <span className="attach-badge">{attachments.length}</span>
            )}
          </button>

          {/* Agent mode selector */}
          <div className="mode-selector">
            <button
              className={`mode-btn ${agentMode === "fast" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("fast")}
              title="Fast — Quick answers, less context"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
              <span>Fast</span>
            </button>
            <button
              className={`mode-btn ${agentMode === "chat" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("chat")}
              title="Balanced — Default, good context"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><path d="M8 12h8M12 8v8"/></svg>
              <span>Balanced</span>
            </button>
            <button
              className={`mode-btn ${agentMode === "planning" ? "mode-btn-active" : ""}`}
              onClick={() => setAgentMode("planning")}
              title="Thorough — Deep analysis, max context"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3"/><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/></svg>
              <span>Thorough</span>
            </button>
          </div>

          <div style={{ flex: 1 }} />

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
