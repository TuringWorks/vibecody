import { memo, useCallback, useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Check, Copy } from "lucide-react";

/**
 * Copy-to-clipboard button. Falls back to a hidden textarea + `execCommand`
 * because the Tauri webview does not expose `navigator.clipboard` on every
 * platform — a copy button that silently does nothing is worse than none.
 */
export function CopyButton({ text, label = "Copy", className = "" }: { text: string; label?: string; className?: string }) {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async () => {
    const ok = await writeClipboard(text);
    if (!ok) return;
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }, [text]);

  return (
    <button
      type="button"
      className={`vx-copy ${className}`.trim()}
      onClick={copy}
      aria-label={label}
      title={label}
    >
      {copied ? <Check size={12} /> : <Copy size={12} />}
      <span>{copied ? "Copied" : label}</span>
    </button>
  );
}

async function writeClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    /* fall through to the legacy path */
  }
  try {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.setAttribute("readonly", "");
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(ta);
    return ok;
  } catch {
    return false;
  }
}

/** Flatten a react-markdown child tree back to the raw text of a code block. */
function nodeText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeText).join("");
  if (typeof node === "object" && "props" in node) {
    return nodeText((node as { props: { children?: ReactNode } }).props.children);
  }
  return "";
}

/**
 * Agent output is markdown — headings, lists, tables, and above all fenced code.
 * Rendering it as a raw string (the previous behaviour) made every reply an
 * unreadable wall of asterisks and backticks. GFM is enabled so tables, task
 * lists and strikethrough render; fenced blocks get a language label and a copy
 * button, which is the single most-used affordance in a coding assistant.
 *
 * Links are rendered as plain text spans: a click inside the webview would
 * navigate the app shell away from itself, and there is no browser surface here
 * to hand it to (the Browser quick-action is a later slice).
 */
export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="vx-md">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          pre: ({ children }) => {
            const code = nodeText(children);
            return (
              <div className="vx-md__pre-wrap">
                <div className="vx-md__pre-bar">
                  <CopyButton text={code} label="Copy code" />
                </div>
                <pre className="vx-md__pre">{children}</pre>
              </div>
            );
          },
          code: ({ className, children }) => {
            const lang = /language-(\w+)/.exec(className ?? "")?.[1];
            // Fenced blocks arrive wrapped in <pre>; inline code does not.
            return lang || className?.startsWith("language-") ? (
              <code className={`vx-md__code ${className ?? ""}`.trim()} data-lang={lang}>
                {children}
              </code>
            ) : (
              <code className="vx-md__inline-code">{children}</code>
            );
          },
          a: ({ children, href }) => (
            <span className="vx-md__link" title={href}>
              {children}
            </span>
          ),
          table: ({ children }) => (
            <div className="vx-md__table-wrap">
              <table className="vx-md__table">{children}</table>
            </div>
          ),
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
});
