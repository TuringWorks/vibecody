/**
 * Helpers for cleaning raw LLM output before it's fed into a preview /
 * runtime / parser. LLMs habitually wrap code in markdown fences and bracket
 * it with explanatory prose ("Here's a button:\n\n```tsx\n…\n```\n\nLet me
 * know if…"). Every panel that previews generated code or markup must strip
 * the prose + fences before use, or the parser downstream will choke.
 */

/**
 * Extract the body of a markdown-fenced code block, or return the raw input
 * stripped of any stray fence markers if no closed fence exists.
 *
 * - Picks the FIRST fenced block. Multiple blocks are uncommon in this flow;
 *   when present, callers want the primary artifact.
 * - Tolerates CRLF, mixed-case language tags (`TSX`, `XML`), and tags that
 *   contain `+`, `-`, or `_` (`react-tsx`, `c++`).
 * - On a truncated stream (open fence, no close), falls back to a defensive
 *   sweep that just removes stray ``` markers.
 *
 * @param raw The raw model response.
 * @returns The cleaned content, trimmed.
 */
export function extractFencedBlock(raw: string): string {
  if (!raw) return "";
  const fenced = raw.match(/```[A-Za-z0-9+\-_]*\s*\r?\n([\s\S]*?)\r?\n```/);
  if (fenced) return fenced[1].trim();
  return raw
    .replace(/```[A-Za-z0-9+\-_]*\s*\r?\n?/g, "")
    .replace(/\r?\n?```\s*$/g, "")
    .replace(/```/g, "")
    .trim();
}

/**
 * Validate and normalize draw.io XML before handing it to the embed editor.
 *
 * - Confirms the XML appears complete (closing tag present). LLMs hit
 *   max-token limits on long sequence diagrams and silently truncate, which
 *   the embedded editor refuses to render. Catching this here lets the UI
 *   warn the user instead of a silently-empty editor canvas.
 * - Wraps raw `<mxGraphModel>` in an `<mxfile><diagram>...</diagram></mxfile>`
 *   envelope. The drawio embed editor's `load` action accepts both forms,
 *   but the file form is the one drawio actually writes to disk and is
 *   more reliably parsed for non-trivial diagrams.
 */
export interface PreparedDrawioXml {
  ok: boolean;
  prepared: string;
  warning?: string;
}

export function prepareDrawioXml(raw: string): PreparedDrawioXml {
  const trimmed = (raw ?? "").trim();
  if (!trimmed) return { ok: false, prepared: "", warning: "XML is empty." };

  // Truncation detection: an LLM that stopped mid-stream leaves the closing
  // tag missing. Both file form and raw graph form must end with the right
  // closing tag.
  const isFile = /^<mxfile\b/.test(trimmed);
  const isGraph = /^<mxGraphModel\b/.test(trimmed);
  if (!isFile && !isGraph) {
    return {
      ok: false,
      prepared: trimmed,
      warning: "XML does not start with <mxGraphModel> or <mxfile>.",
    };
  }
  const closer = isFile ? "</mxfile>" : "</mxGraphModel>";
  if (!trimmed.endsWith(closer)) {
    return {
      ok: false,
      prepared: trimmed,
      warning: `XML appears truncated — missing ${closer}. The model likely hit a max-token limit; try regenerating with a shorter description.`,
    };
  }

  if (isFile) return { ok: true, prepared: trimmed };

  // Raw mxGraphModel → wrap in canonical mxfile envelope.
  const wrapped =
    `<mxfile host="embed.diagrams.net" modified="${new Date().toISOString()}" agent="vibecody" version="21.0.0">` +
    `<diagram name="Page-1" id="page-1">${trimmed}</diagram>` +
    `</mxfile>`;
  return { ok: true, prepared: wrapped };
}

/**
 * Remove ES-module syntax that an iframe sandbox running code via
 * `new Function(body)` cannot evaluate. `new Function` runs in script scope,
 * so any leftover `export` / `import` throws "Unexpected keyword 'export'".
 * Babel's TypeScript preset preserves these by default — we have to strip
 * them before transpiling.
 */
export function stripModuleSyntax(src: string): string {
  let s = src;
  // `import … from '…';`, `import '…';`, `import * as X from '…';`
  s = s.replace(/^\s*import\s+[^;]*?['"][^'"]*['"]\s*;?\s*$/gm, "");
  s = s.replace(/^\s*import\s*['"][^'"]*['"]\s*;?\s*$/gm, "");
  // `export { Foo, Bar };`, `export { Foo as Bar } from '…';`,
  // `export * from '…';`, `export * as Ns from '…';`
  s = s.replace(/^\s*export\s+(\{[^}]*\}|\*(?:\s+as\s+\w+)?)\s*(?:from\s*['"][^'"]+['"])?\s*;?\s*$/gm, "");
  // `export default <expr>;` — drop the keyword, leave the expression.
  s = s.replace(/^\s*export\s+default\s+/gm, "");
  // `export const|let|var|function|class|interface|type|enum` — keyword only.
  s = s.replace(/^\s*export\s+(?=(?:const|let|var|function|class|interface|type|enum)\s)/gm, "");
  return s;
}
