/**
 * diagrams — Mermaid and PlantUML, from source text to an SVG on screen.
 *
 * Two diagram languages, two entirely different renderers, one interface:
 *
 * * **Mermaid** is JavaScript, so it runs here. The library is a megabyte, and
 *   most sessions never open a diagram, so it is imported the first time one is
 *   drawn rather than with the bundle.
 * * **PlantUML** is a Java program with no browser port. It is drawn by the
 *   backend, using whatever PlantUML the machine has — see
 *   `vibecoder/src-tauri/src/plantuml.rs`, including why nothing is sent to a
 *   remote renderer.
 *
 * Both outputs are sanitised before they reach the DOM. Diagram source is a
 * file someone opened, which per the threat model came from somewhere else, and
 * SVG is a document format with scripts in it.
 */
import DOMPurify from "dompurify";
import { invoke } from "@tauri-apps/api/core";

/** The diagram languages this build can draw. */
export type DiagramKind = "mermaid" | "plantuml";

/** How each is named to a person. */
export const DIAGRAM_LABELS: Record<DiagramKind, string> = {
  mermaid: "Mermaid",
  plantuml: "PlantUML",
};

/**
 * Extensions that hold one diagram and nothing else.
 *
 * `.wsd` and `.iuml` are PlantUML's own conventions — the first from its web
 * sequence-diagram roots, the second for files meant to be `!include`d.
 */
const FILE_EXTENSIONS: Record<string, DiagramKind> = {
  mmd: "mermaid",
  mermaid: "mermaid",
  puml: "plantuml",
  plantuml: "plantuml",
  pu: "plantuml",
  iuml: "plantuml",
  wsd: "plantuml",
};

/**
 * Info strings that open a fenced diagram inside markdown.
 *
 * The aliases are the ones people actually type; a `” ```puml ` block is a
 * PlantUML diagram whatever the renderer's own name for it is.
 */
const FENCE_LANGUAGES: Record<string, DiagramKind> = {
  mermaid: "mermaid",
  plantuml: "plantuml",
  puml: "plantuml",
  uml: "plantuml",
};

/** The diagram language of a file, or null when it is not a diagram file. */
export function diagramKindForFile(filename: string): DiagramKind | null {
  const extension = filename.split(".").pop()?.toLowerCase() ?? "";
  return FILE_EXTENSIONS[extension] ?? null;
}

/** Whether a path opens in the diagram preview. */
export function isDiagramFile(filename: string): boolean {
  return diagramKindForFile(filename) !== null;
}

/** The diagram language of a fenced code block, or null for ordinary code. */
export function diagramKindForFence(language: string | null | undefined): DiagramKind | null {
  if (!language) return null;
  return FENCE_LANGUAGES[language.toLowerCase()] ?? null;
}

/** Every extension the preview claims, for the file-type registry and tests. */
export const DIAGRAM_EXTENSIONS = Object.keys(FILE_EXTENSIONS);

/**
 * Draw a diagram, returning SVG that is safe to insert.
 *
 * Throws with the renderer's own message — a Mermaid parse error names the line,
 * a missing PlantUML says how to install it — because "the diagram did not
 * render" is not something anyone can act on.
 */
export async function renderDiagram(kind: DiagramKind, source: string): Promise<string> {
  const svg = kind === "mermaid" ? await renderMermaid(source) : await renderPlantUml(source);
  return sanitizeSvg(svg);
}

/** Where PlantUML is coming from on this machine, or null if it is nowhere. */
export async function plantUmlRenderer(): Promise<string | null> {
  const found = await invoke<string | null>("plantuml_renderer");
  return found ?? null;
}

// ── Mermaid ──────────────────────────────────────────────────────────

/** Ids must be unique per render: Mermaid uses them for its own element ids. */
let renderCount = 0;

async function renderMermaid(source: string): Promise<string> {
  const mermaid = (await import("mermaid")).default;
  mermaid.initialize({
    startOnLoad: false,
    // Diagram text is a file's contents, not ours. `strict` escapes it rather
    // than letting a label carry markup into the page.
    securityLevel: "strict",
    // Labels as SVG `<text>`, not HTML in a `<foreignObject>`.
    //
    // Mermaid's default is `<foreignObject><div>…`, which is HTML smuggled into
    // an SVG document — and the sanitiser removes it, namespace and all. That
    // is not a bug to work around by loosening the sanitiser: it produced a
    // diagram with every box and arrow in place and **not one label**, which is
    // worse than an error because it looks like it worked. Text labels do not
    // wrap as prettily; they are also not a second document format inside the
    // first one.
    htmlLabels: false,
    flowchart: { htmlLabels: false },
    class: { htmlLabels: false },
    theme: isDarkTheme() ? "dark" : "default",
    fontFamily: "var(--font-primary, system-ui, sans-serif)",
  });
  renderCount += 1;
  const { svg } = await mermaid.render(`vibecoder-diagram-${renderCount}`, source);
  return svg;
}

/** Which way round the app's palette is, so a diagram is not white-on-white. */
function isDarkTheme(): boolean {
  const mode = document.documentElement.getAttribute("data-theme");
  return mode !== "light";
}

// ── PlantUML ─────────────────────────────────────────────────────────

async function renderPlantUml(source: string): Promise<string> {
  return invoke<string>("render_plantuml", { source });
}

// ── Sanitising ───────────────────────────────────────────────────────

/**
 * Strip anything executable out of rendered SVG.
 *
 * Mermaid escapes the text it is given, and PlantUML draws shapes — but both
 * produce a document that *can* carry `<script>`, `onload`, and `javascript:`
 * links, and the input came from a file. What is kept is SVG and its filters:
 * shapes, paths, text, and the inline stylesheet that colours them.
 */
export function sanitizeSvg(svg: string): string {
  return DOMPurify.sanitize(svg, {
    USE_PROFILES: { svg: true, svgFilters: true },
    // Mermaid puts the diagram's entire styling in an inline `<style>`; without
    // it the picture comes back as black shapes on a black background.
    ADD_TAGS: ["style"],
    // `foreignObject` is how HTML gets inside an SVG. Mermaid is configured not
    // to emit any (see `htmlLabels` above), so anything that turns up in one
    // arrived from the file rather than from the renderer.
    FORBID_TAGS: ["script", "iframe", "object", "embed", "foreignObject"],
    FORBID_ATTR: ["onload", "onerror", "onclick"],
  });
}
