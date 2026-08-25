/**
 * How a markdown *document* is rendered in the Markdown panel — on screen and
 * in an exported file, from one definition, so the two cannot drift.
 *
 * It lives outside `MarkdownPanel.tsx` because that module is lazy-loaded as a
 * panel and may export nothing but components.
 */
import type { ReactNode } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import ReactMarkdown, { type Components } from "react-markdown";
import { MarkdownWithDetails } from "./MarkdownDetails";
import { htmlToMarkdown } from "../lib/markdownHtml";

const previewComponents: Components = {
 h1: ({ children }) => <h1 style={{ fontSize: 28, fontWeight: 700, borderBottom: "1px solid var(--border-color)", paddingBottom: 8, marginBottom: 16 }}>{children}</h1>,
 h2: ({ children }) => <h2 style={{ fontSize: 22, fontWeight: 600, marginTop: 28, marginBottom: 10 }}>{children}</h2>,
 h3: ({ children }) => <h3 style={{ fontSize: 18, fontWeight: 600, marginTop: 20, marginBottom: 8 }}>{children}</h3>,
 p: ({ children }) => <p style={{ margin: "0 0 16px" }}>{children}</p>,
 code: ({ className, children }) => {
 const isBlock = className?.startsWith("language-");
 return isBlock
 ? <code style={{ display: "block", background: "var(--bg-secondary)", padding: "16px 16px", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", overflowX: "auto", margin: "12px 0", whiteSpace: "pre" }}>{children}</code>
 : <code style={{ background: "var(--bg-secondary)", padding: "1px 4px", borderRadius: 3, fontSize: "0.9em", fontFamily: "var(--font-mono)" }}>{children}</code>;
 },
 pre: ({ children }) => <>{children}</>,
 blockquote: ({ children }) => <blockquote style={{ borderLeft: "3px solid var(--accent-color)", margin: "16px 0", paddingLeft: 16, color: "var(--text-secondary)", fontStyle: "italic" }}>{children}</blockquote>,
 ul: ({ children }) => <ul style={{ paddingLeft: 24, margin: "12px 0" }}>{children}</ul>,
 ol: ({ children }) => <ol style={{ paddingLeft: 24, margin: "12px 0" }}>{children}</ol>,
 li: ({ children }) => <li style={{ marginBottom: 4 }}>{children}</li>,
 a: ({ href, children }) => <a href={href} target="_blank" rel="noreferrer" style={{ color: "var(--text-info)" }}>{children}</a>,
 hr: () => <hr style={{ border: "none", borderTop: "1px solid var(--border-color)", margin: "24px 0" }} />,
 table: ({ children }) => <table style={{ borderCollapse: "collapse", width: "100%", margin: "16px 0" }}>{children}</table>,
 th: ({ children }) => <th style={{ border: "1px solid var(--border-color)", padding: "8px 12px", background: "var(--bg-secondary)", fontWeight: 600 }}>{children}</th>,
 td: ({ children }) => <td style={{ border: "1px solid var(--border-color)", padding: "8px 12px" }}>{children}</td>,
 img: ({ src, alt }) => <img src={src} alt={alt ?? ""} style={{ maxWidth: "100%", borderRadius: "var(--radius-sm)" }} />,
};

// A <summary> is a single line; paragraphs would push the label onto its own.
const summaryComponents: Components = { ...previewComponents, p: ({ children }) => <>{children}</> };

export const renderBlock = (markdown: string) => (
 <ReactMarkdown components={previewComponents}>{htmlToMarkdown(markdown)}</ReactMarkdown>
);

export const renderSummary = (markdown: string) => (
 <ReactMarkdown components={summaryComponents}>{htmlToMarkdown(markdown)}</ReactMarkdown>
);

/**
 * Render a tree to an HTML string through a detached root.
 *
 * `react-dom/server` would say this in one call and cost 187 kB (57 kB gzipped)
 * on this panel's chunk — measured — for the sake of one export button. The
 * client renderer is already loaded.
 */
function renderToHtml(node: ReactNode): string {
 const host = document.createElement("div");
 const root = createRoot(host);
 try {
 flushSync(() => root.render(node));
 return host.innerHTML;
 } finally {
 root.unmount();
 }
}

const escapeHtml = (text: string): string =>
 text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] ?? c);

/**
 * Export the document as HTML.
 *
 * This used to drop the markdown *source* into the `<body>` — the exported
 * file showed `# Title` and `- item` as literal text, the same defect as raw
 * HTML showing up in the preview, and the comment claiming it snapshotted the
 * rendered pane described something the code never did. It renders through the
 * preview's own components now, so the export is what is on screen,
 * disclosures included.
 */
export function renderDocumentHtml(markdown: string, title: string): string {
 const body = renderToHtml(
 <MarkdownWithDetails source={markdown} renderBlock={renderBlock} renderInline={renderSummary} />,
 );
 return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>${escapeHtml(title)}</title>
<style>
 /* The panel's components style themselves with the app's theme variables;
    outside the app those resolve to nothing, so the export carries a light
    palette under the same names rather than a second set of components. */
 :root{--border-color:#d0d7de;--bg-secondary:#f6f8fa;--bg-tertiary:#f6f8fa;--text-primary:#24292f;--text-secondary:#57606a;--text-info:#0969da;--accent-color:#0969da;--accent-blue:#0969da;--radius-sm:6px;--font-mono:ui-monospace,SFMono-Regular,Menlo,monospace;--font-size-base:0.9em}
 body{max-width:720px;margin:40px auto;font-family:system-ui,sans-serif;line-height:1.7;color:#24292f}
 pre{background:#f6f8fa;padding:16px;border-radius:6px;overflow:auto}
 code{background:#f6f8fa;padding:2px 5px;border-radius:4px;font-size:.9em}
 blockquote{border-left:4px solid #d0d7de;margin:0;padding:0 16px;color:#57606a}
 img{max-width:100%}
 table{border-collapse:collapse;width:100%}
 th,td{border:1px solid #d0d7de;padding:8px 12px}
 .md-details{border:1px solid #d0d7de;border-radius:6px;margin:16px 0;background:#f6f8fa}
 .md-details__summary{cursor:pointer;padding:8px 12px;font-weight:600}
 .md-details__body{padding:0 12px;border-top:1px solid #d0d7de}
</style>
</head>
<body>
${body}
</body>
</html>`;
}
