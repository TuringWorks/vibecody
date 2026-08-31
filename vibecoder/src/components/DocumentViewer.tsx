/**
 * DocumentViewer — renders PDF, EPUB, DOCX and Apple Pages files in the editor
 * area, and hands any of them to the text editor on request.
 *
 * PDF:  Pages are drawn to <canvas> with PDF.js, one at a time or two side by
 *       side. They used to go to an <iframe> and be the platform's business,
 *       which is why this panel had no page navigation and no spread.
 *
 * EPUB: Parses the EPUB (ZIP containing XHTML/CSS/images) via the Tauri
 *       backend and renders extracted HTML chapters, with the book's own
 *       stylesheets, in a scrolling pane or as two columns of a screen.
 *
 * Features:
 *   • Page navigation and a two-page spread (PDF); chapter navigation (EPUB)
 *   • A two-column reading spread for the formats that are text, not sheets
 *   • Zoom, font size, page and chapter counts, contents sidebar
 *   • Dark/light theme integration
 */

import { useState, useRef, useCallback, useEffect, useMemo } from "react";
import DOMPurify from "dompurify";
import {
  AlertTriangle,
  BookOpen,
  ChevronLeft,
  ChevronRight,
  FileText,
  Info,
  Pencil,
  Square,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { DocumentTextEditor } from "./DocumentTextEditor";
import { MarkdownPreview } from "./MarkdownPreview";
import {
  getLayout,
  getMode,
  hasDraft,
  setLayout as rememberLayout,
  setMode as rememberMode,
} from "../lib/documentDrafts";
import {
  canTurn,
  fitScale,
  GUTTER,
  pagesInView,
  turn,
  viewLabel,
  viewStart,
  type Layout,
} from "../lib/pageSpread";
import type { PageSize, PdfHandle } from "../lib/pdfDocument";
import {
  dataUrl,
  readEpubBook,
  readEpubChapter,
  resourceUrls,
  type EpubBook,
  type EpubTocEntry,
} from "../lib/epubBook";
import {
  resolveAgainst,
  rewriteChapterHtml,
  scopeEpubCss,
  splitHref,
} from "../lib/epubRender";
import {
  documentErrorMessage,
  formatLabel,
  readDocumentPreview,
  readDocumentText,
  richDocumentFormat,
  type DocumentWarning,
  type RichDocumentFormat,
} from "../lib/richDocuments";
import "./DocumentViewer.css";

// ── Helpers ──────────────────────────────────────────────────────────

const DOCUMENT_EXTENSIONS = new Set(["pdf", "epub", "docx", "pages"]);

/** Check if a filename is a supported document file */
export function isDocumentFile(filename: string): boolean {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  return DOCUMENT_EXTENSIONS.has(ext);
}

/**
 * Whether this document's viewer renders from the file's bytes.
 *
 * Only PDF does — its pages are drawn here, from the file itself. EPUB, DOCX
 * and Pages are parsed by the backend, so reading them into a base64 string on
 * open would move the whole file through a JS string for nothing.
 */
export function needsRawBytes(filename: string): boolean {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  return ext === "pdf";
}

// ── Props ────────────────────────────────────────────────────────────

interface DocumentViewerProps {
  /** Absolute file path */
  filePath: string;
  /** Base64-encoded file content */
  base64Data: string;
}

// ── PDF Viewer Sub-component ─────────────────────────────────────────

/** What the PDF viewer is doing. */
type PdfState =
  | { status: "loading" }
  | { status: "ready"; handle: PdfHandle }
  | { status: "failed"; message: string };

/**
 * PdfViewer — renders the pages itself, one at a time or two side by side.
 *
 * The pages are drawn to canvases through PDF.js rather than handed to the
 * platform's own viewer in an `<iframe>`. That embed was simpler and it is why
 * this panel had no page navigation and no spread: everything inside the frame
 * belonged to the browser, including which page you were looking at.
 */
function PdfViewer({ filePath, base64Data, onEditText }: DocumentViewerProps & EditableProps) {
  const [state, setState] = useState<PdfState>({ status: "loading" });
  const [page, setPage] = useState(1);
  const [layout, setLayoutPreference] = useLayout(filePath);
  // Zoom is either "whatever fits" or a number someone chose. Keeping the two
  // apart is what lets a resized window re-fit without undoing a chosen zoom.
  const [zoom, setZoom] = useState<{ fit: true } | { fit: false; scale: number }>({
    fit: true,
  });
  const [pageSize, setPageSize] = useState<PageSize | null>(null);
  const [paneSize, setPaneSize] = useState<PageSize | null>(null);
  const [renderError, setRenderError] = useState<string | null>(null);
  const canvases = useRef<Array<HTMLCanvasElement | null>>([]);
  const paneRef = useRef<HTMLDivElement>(null);

  const fileName = filePath.split(/[/\\]/).pop() || filePath;
  const pageCount = state.status === "ready" ? state.handle.pageCount : 0;
  const visible = useMemo(
    () => pagesInView(page, pageCount, layout),
    [page, pageCount, layout],
  );

  // ── The document ──────────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    let opened: PdfHandle | null = null;
    setState({ status: "loading" });
    setPage(1);
    if (!base64Data) return;

    // Loaded on demand: PDF.js is a megabyte of parser, and most sessions never
    // open a PDF. Importing it with the viewer would put it in the bundle every
    // window pays for at startup.
    import("../lib/pdfDocument")
      .then(({ openPdf }) => openPdf(base64Data))
      .then((handle) => {
        opened = handle;
        if (cancelled) {
          handle.close();
          return;
        }
        setState({ status: "ready", handle });
      })
      .catch((error) => {
        if (!cancelled) {
          setState({ status: "failed", message: documentErrorMessage(error) });
        }
      });

    return () => {
      cancelled = true;
      opened?.close();
    };
  }, [base64Data]);

  // ── How big a page is, and how much room there is for it ─────────
  useEffect(() => {
    if (state.status !== "ready") return;
    let cancelled = false;
    state.handle
      .naturalSize(visible[0] ?? 1)
      .then((size) => {
        if (!cancelled) setPageSize(size);
      })
      .catch(() => {
        // Not being able to measure a page only costs the fit; the page still
        // draws, at whatever scale is in force.
      });
    return () => {
      cancelled = true;
    };
  }, [state, visible]);

  useEffect(() => {
    const pane = paneRef.current;
    if (!pane) return;
    const measure = () =>
      setPaneSize({ width: pane.clientWidth, height: pane.clientHeight });
    measure();
    if (typeof ResizeObserver !== "function") return;
    const observer = new ResizeObserver(measure);
    observer.observe(pane);
    return () => observer.disconnect();
  }, [state.status]);

  const scale = useMemo(() => {
    if (!zoom.fit) return zoom.scale;
    if (!pageSize || !paneSize || paneSize.width === 0) return 1;
    return fitScale(paneSize, pageSize, visible.length);
  }, [zoom, pageSize, paneSize, visible.length]);

  // ── The pages on screen ───────────────────────────────────────────
  useEffect(() => {
    if (state.status !== "ready") return;
    let cancelled = false;
    setRenderError(null);
    const { handle } = state;

    // Rendered in order rather than in parallel: PDF.js serialises work on one
    // document anyway, and a spread whose right page appears first reads as a
    // flicker.
    void (async () => {
      for (const [slot, number] of visible.entries()) {
        const canvas = canvases.current[slot];
        if (!canvas || cancelled) return;
        try {
          // `null` means a later draw took this canvas over; stop rather than
          // carry on painting pages at a scale nobody is looking at any more.
          if ((await handle.renderPage(number, canvas, scale)) === null) return;
        } catch (error) {
          if (!cancelled) setRenderError(documentErrorMessage(error));
          return;
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [state, visible, scale]);

  const setLayout = useCallback(
    (next: Layout) => {
      setLayoutPreference(next);
      // Stay on the page you were reading rather than on the spread's index.
      setPage((current) => viewStart(current, next));
    },
    [setLayoutPreference],
  );

  const goto = useCallback(
    (direction: 1 | -1) => setPage((current) => turn(current, pageCount, layout, direction)),
    [pageCount, layout],
  );

  // Arrow keys page the document, the way every reader does.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (event.key === "ArrowRight" || event.key === "PageDown") {
        event.preventDefault();
        goto(1);
      } else if (event.key === "ArrowLeft" || event.key === "PageUp") {
        event.preventDefault();
        goto(-1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [goto]);

  if (state.status === "failed") {
    return (
      <div className="document-viewer">
        <div className="document-viewer-error">
          <AlertTriangle size={16} className="error-icon" />
          <span className="error-message">{state.message}</span>
        </div>
      </div>
    );
  }

  if (state.status === "loading") {
    return <LoadingPane format="pdf" />;
  }

  return (
    <div className="document-viewer pdf-viewer">
      <div className="document-viewer-toolbar">
        <div className="toolbar-group">
          <button
            onClick={() => setZoom({ fit: false, scale: Math.max(scale / 1.25, 0.25) })}
            title="Zoom out"
          >
            −
          </button>
          <span className="zoom-label">{Math.round(scale * 100)}%</span>
          <button
            onClick={() => setZoom({ fit: false, scale: Math.min(scale * 1.25, 5) })}
            title="Zoom in"
          >
            +
          </button>
          <button
            onClick={() => setZoom({ fit: true })}
            title="Fit the page (or the spread) to the window"
            className={`toolbar-btn-wide${zoom.fit ? " active" : ""}`}
            aria-pressed={zoom.fit}
          >
            Fit
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <button
            onClick={() => goto(-1)}
            disabled={!canTurn(page, pageCount, layout, -1)}
            title="Previous page (←)"
            aria-label="Previous page"
          >
            <ChevronLeft size={14} />
          </button>
          <span className="zoom-label page-label">{viewLabel(page, pageCount, layout)}</span>
          <button
            onClick={() => goto(1)}
            disabled={!canTurn(page, pageCount, layout, 1)}
            title="Next page (→)"
            aria-label="Next page"
          >
            <ChevronRight size={14} />
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <LayoutToggle layout={layout} onChange={setLayout} unit="page" />
        </div>
        {onEditText && (
          <>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <EditTextButton onEditText={onEditText} />
            </div>
          </>
        )}
        <div className="file-info">
          <span className="info-badge">PDF</span>
          <span className="info-badge">{fileName}</span>
        </div>
      </div>

      {renderError && (
        <div className="document-viewer-error doc-inline-error">
          <AlertTriangle size={14} className="error-icon" />
          <span className="error-message">{renderError}</span>
        </div>
      )}

      <div className="document-viewer-canvas" ref={paneRef}>
        <div
          className="pdf-spread"
          /* The same number the fit is computed from — one place, so the pages
             cannot be measured against a gap the stylesheet does not use. */
          style={{ gap: GUTTER, padding: GUTTER }}
        >
          {visible.map((number, slot) => (
            <div className="pdf-page" key={number}>
              <canvas
                ref={(element) => {
                  canvases.current[slot] = element;
                }}
                aria-label={`Page ${number}`}
              />
              <div className="pdf-page-number">{number}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── EPUB Viewer Sub-component ────────────────────────────────────────

/** A chapter, sanitised and wired to the resources that came with it. */
interface RenderedChapter {
  path: string;
  title: string | null;
  html: string;
  css: string;
  warnings: DocumentWarning[];
}

type BookState =
  | { status: "loading" }
  | { status: "ready"; book: EpubBook }
  | { status: "failed"; message: string };

/**
 * EpubViewer — renders a book the way the book was written: its own markup, its
 * own stylesheets, its own images, its table of contents, and links that work.
 *
 * Reading happens in the backend. The browser-side reader this replaced could
 * not inflate a deflate-compressed ZIP entry — which is every chapter of every
 * real EPUB — so it fell through to a card telling the user to open the file in
 * a different application.
 */
function EpubViewer({ filePath, onEditText }: { filePath: string } & EditableProps) {
  const [state, setState] = useState<BookState>({ status: "loading" });
  const [index, setIndex] = useState(0);
  const [chapter, setChapter] = useState<RenderedChapter | null>(null);
  const [chapterError, setChapterError] = useState<string | null>(null);
  const [fragment, setFragment] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState(17);
  const [showToc, setShowToc] = useState(true);
  const [layout, setLayout] = useLayout(filePath);
  const contentRef = useRef<HTMLDivElement>(null);

  const fileName = filePath.split(/[/\\]/).pop() || filePath;

  // ── The book ──────────────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    setIndex(0);
    setChapter(null);
    readEpubBook(filePath)
      .then((book) => {
        if (!cancelled) setState({ status: "ready", book });
      })
      .catch((error) => {
        if (!cancelled) setState({ status: "failed", message: documentErrorMessage(error) });
      });
    return () => {
      cancelled = true;
    };
  }, [filePath]);

  const book = state.status === "ready" ? state.book : null;
  const chapterPath = book?.chapters[index]?.path;

  // ── The chapter ───────────────────────────────────────────────────
  useEffect(() => {
    if (!chapterPath) return;
    let cancelled = false;
    let revoke: (() => void) | null = null;
    setChapterError(null);

    readEpubChapter(filePath, chapterPath)
      .then((raw) => {
        if (cancelled) return;
        const resources = resourceUrls(raw.resources);
        revoke = resources.revoke;
        const resolve = (reference: string) =>
          resources.urls.get(reference) ??
          resources.urls.get(resolveAgainst(raw.path, reference));

        // Sanitise first, then rewrite references: rewriting only touches
        // attributes, so it cannot put back anything DOMPurify removed.
        const rewritten = rewriteChapterHtml(sanitizeEpubHtml(raw.html), resolve);
        setChapter({
          path: raw.path,
          title: raw.title,
          html: rewritten.html,
          css: scopeEpubCss(raw.css, ".epub-chapter-body", resolve),
          warnings: raw.warnings,
        });
      })
      .catch((error) => {
        if (!cancelled) setChapterError(documentErrorMessage(error));
      });

    return () => {
      cancelled = true;
      // Object URLs outlive the chapter that made them unless they are revoked.
      revoke?.();
    };
  }, [filePath, chapterPath]);

  // Land on the requested anchor, or at the top of a new chapter.
  useEffect(() => {
    if (!chapter) return;
    const container = contentRef.current;
    if (!container) return;
    if (fragment) {
      const target = container.querySelector(`#${cssEscape(fragment)}, [name="${fragment}"]`);
      if (target) {
        // Optional calls: scrolling is a nicety, and not every host implements
        // these (jsdom does not). A missing anchor must not break the chapter.
        target.scrollIntoView?.({ block: "start" });
        return;
      }
    }
    container.scrollTo?.(0, 0);
  }, [chapter, fragment]);

  const goTo = useCallback(
    (path: string, anchor: string | null) => {
      const target = book?.chapters.findIndex((c) => c.path === path) ?? -1;
      if (target < 0) return false;
      setFragment(anchor);
      setIndex(target);
      return true;
    },
    [book],
  );

  /** Internal links navigate the book; external ones leave the app. */
  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      const anchor = (event.target as HTMLElement).closest("a");
      if (!anchor) return;

      const external = anchor.getAttribute("data-epub-external");
      if (external) {
        event.preventDefault();
        openUrl(external).catch(() => {});
        return;
      }
      const link = anchor.getAttribute("data-epub-link");
      if (link === null || !chapter) return;
      event.preventDefault();

      const { path, fragment: anchorName } = splitHref(link);
      if (!path) {
        // A bare `#anchor` stays in this chapter.
        setFragment(anchorName);
        const target = contentRef.current?.querySelector(
          `#${cssEscape(anchorName ?? "")}, [name="${anchorName}"]`,
        );
        target?.scrollIntoView?.({ block: "start" });
        return;
      }
      goTo(resolveAgainst(chapter.path, path), anchorName);
    },
    [chapter, goTo],
  );

  const chapterCount = book?.chapters.length ?? 0;
  const prev = useCallback(() => {
    setFragment(null);
    setIndex((i) => Math.max(0, i - 1));
  }, []);
  const next = useCallback(() => {
    setFragment(null);
    setIndex((i) => Math.min(chapterCount - 1, i + 1));
  }, [chapterCount]);

  if (state.status === "loading") return <LoadingPane format="epub" />;
  if (state.status === "failed") return <ErrorPane message={state.message} />;
  if (!book) return <ErrorPane message="This EPUB has no readable chapters." />;

  const heading = chapter?.title || book.chapters[index]?.title || `Chapter ${index + 1}`;

  return (
    <div className="document-viewer epub-viewer">
      {/* ── Toolbar ──────────────────────────────────────────────── */}
      <div className="document-viewer-toolbar">
        <div className="toolbar-group">
          <button onClick={prev} disabled={index === 0} title="Previous Chapter">
            <ChevronLeft size={14} />
          </button>
          <span className="zoom-label chapter-label">
            {index + 1} / {chapterCount}
          </span>
          <button onClick={next} disabled={index >= chapterCount - 1} title="Next Chapter">
            <ChevronRight size={14} />
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <button onClick={() => setFontSize((s) => Math.max(s - 1, 12))} title="Decrease Font Size">
            A−
          </button>
          <span className="zoom-label font-label">{fontSize}px</span>
          <button onClick={() => setFontSize((s) => Math.min(s + 1, 32))} title="Increase Font Size">
            A+
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <button
            onClick={() => setShowToc((v) => !v)}
            title="Toggle Table of Contents"
            className={`toolbar-btn-wide${showToc ? " active" : ""}`}
          >
            Contents
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <LayoutToggle layout={layout} onChange={setLayout} unit="screen" />
        </div>
        {onEditText && (
          <>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <EditTextButton onEditText={onEditText} />
            </div>
          </>
        )}
        <div className="file-info">
          <span className="info-badge">EPUB</span>
          <span className="info-badge">{book.title || fileName}</span>
        </div>
      </div>

      <WarningNotice warnings={[...book.warnings, ...(chapter?.warnings ?? [])]} />
      {chapterError && (
        <div className="document-viewer-error doc-inline-error">
          <AlertTriangle size={14} className="error-icon" />
          <span className="error-message">{chapterError}</span>
        </div>
      )}

      {/* ── Contents + chapter ───────────────────────────────────── */}
      <div className="epub-content-area">
        {showToc && (
          <div className="epub-toc">
            {book.cover && (
              <img className="epub-cover" src={dataUrl(book.cover)} alt={`Cover of ${book.title ?? fileName}`} />
            )}
            <div className="epub-book-meta">
              <div className="epub-book-title">{book.title || fileName}</div>
              {book.authors.length > 0 && (
                <div className="epub-book-authors">{book.authors.join(" · ")}</div>
              )}
            </div>
            <div className="epub-toc-header">Contents</div>
            {tocEntries(book).map((entry, i) => {
              const active = entry.path === chapterPath;
              return (
                <button
                  key={`${entry.path}#${entry.fragment ?? ""}-${i}`}
                  className={`epub-toc-item${active ? " active" : ""}`}
                  style={{ paddingLeft: 8 + entry.level * 12 }}
                  onClick={() => goTo(entry.path, entry.fragment)}
                  title={entry.label}
                >
                  <span className="toc-title">{entry.label}</span>
                </button>
              );
            })}
          </div>
        )}

        <div
          ref={contentRef}
          className={`epub-chapter-scroll${layout === "spread" ? " reading-paged" : ""}`}
          onClick={handleClick}
        >
          {chapter ? (
            <>
              {/* The book's own stylesheet, scoped to the chapter container. */}
              <style>{chapter.css}</style>
              <div
                className={`epub-chapter-body${
                  layout === "spread" ? " reading-columns" : ""
                }`}
                style={{ fontSize }}
                /* Sanitised by sanitizeEpubHtml() above; rewriteChapterHtml()
                   only edits attributes on what survived. */
                dangerouslySetInnerHTML={{ __html: chapter.html }}
              />
            </>
          ) : (
            <div className="document-viewer-loading">
              <div className="doc-spinner" />
              <span>Loading {heading}…</span>
            </div>
          )}
        </div>
      </div>
      {layout === "spread" && <PageStrip pane={contentRef} />}
    </div>
  );
}

/** The book's own contents list, or the spine when it has none. */
function tocEntries(book: EpubBook): EpubTocEntry[] {
  if (book.toc.length > 0) return book.toc;
  return book.chapters.map((chapter, i) => ({
    label: chapter.title || `Chapter ${i + 1}`,
    path: chapter.path,
    fragment: null,
    level: 0,
  }));
}

/** Escape an id for use in a selector, without assuming `CSS.escape` exists. */
function cssEscape(value: string): string {
  // Called as a method, not lifted out: `CSS.escape` is bound to `CSS` and
  // throws ("Illegal invocation" in browsers, a TypeError in jsdom) when it is
  // detached from it.
  const css = (globalThis as { CSS?: { escape?: (v: string) => string } }).CSS;
  if (typeof css?.escape === "function") {
    try {
      return css.escape(value);
    } catch {
      // Fall through to the manual escape below.
    }
  }
  return value.replace(/[^\w-]/g, "\\$&");
}

// DOMPurify configuration for EPUB chapter HTML.
//
// EPUB content is T5 (attacker-controlled) per docs/security/threat-model.md —
// the user opened a file off disk that originated elsewhere on the internet.
// The earlier ad-hoc regex sanitizer missed common bypasses (javascript: URLs,
// <iframe>/<object>/<embed>/<base>/<meta http-equiv=refresh>, unquoted on*
// handlers, style imports), so route everything through DOMPurify with an
// explicit allow-list of presentational tags + attributes.
//
// FORBID rather than relying on the default deny: tags we explicitly know are
// dangerous (script/iframe/object/embed/link/meta/base/form/input/button)
// stay banned even if a future EPUB extension legitimizes them.
const EPUB_SANITIZE_CONFIG = {
  ALLOWED_TAGS: [
    "a", "abbr", "address", "article", "aside",
    "b", "blockquote", "br",
    "caption", "cite", "code", "col", "colgroup",
    "dd", "details", "dfn", "div", "dl", "dt",
    "em",
    "figcaption", "figure", "footer",
    "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr",
    "i", "img",
    "kbd",
    "li",
    "main", "mark",
    "nav",
    "ol",
    "p", "pre",
    "q",
    "s", "samp", "section", "small", "span", "strong", "sub", "summary", "sup",
    "table", "tbody", "td", "tfoot", "th", "thead", "time", "tr",
    "u", "ul",
    "var",
    // Inline SVG common for typographic ornaments
    "svg", "g", "path", "circle", "rect", "line", "polyline", "polygon", "text", "tspan", "title", "desc",
    // SVG <image>: the standard wrapper for an EPUB cover page. Same risk
    // profile as <img>, which is already allowed — it renders a raster and
    // executes nothing. Deliberately NOT <use>, whose xlink:href has a long
    // history of pulling in and animating foreign content.
    "image",
  ],
  ALLOWED_ATTR: [
    "alt", "class", "colspan", "datetime", "dir", "id", "lang", "rowspan", "src", "title",
    // `href` and `xlink:href` are load-bearing for a book, not a nicety: the
    // table of contents, footnote returns, and every cross-reference are links,
    // and a cover page is usually an <svg><image xlink:href="cover.jpg"/>. They
    // were absent from this list, so every link in every EPUB rendered dead and
    // SVG-wrapped covers rendered blank.
    //
    // What makes them safe is DOMPurify's default ALLOWED_URI_REGEXP, which
    // permits only benign schemes and drops `javascript:` / `vbscript:` / other
    // script URLs from either attribute — see the sanitize tests. The viewer
    // then replaces every surviving href with "#" and navigates itself, so even
    // an allowed URL never becomes a live navigation out of the editor.
    "href", "xlink:href",
    // SVG-specific
    "cx", "cy", "d", "fill", "height", "points", "r", "rx", "ry", "stroke", "transform", "viewBox", "width", "x", "x1", "x2", "y", "y1", "y2",
  ],
  // Belt-and-suspenders against the bypasses the prior regex missed.
  FORBID_TAGS: ["script", "iframe", "object", "embed", "link", "meta", "base", "form", "input", "button", "textarea", "select", "option", "style"],
  FORBID_ATTR: ["style", "srcset", "formaction", "action"],
  // Drop data-* attributes. NOTE: this option governs `data-*` ATTRIBUTES only
  // — it has nothing to do with `data:` URIs. Those are handled by DOMPurify's
  // default ALLOWED_URI_REGEXP, which permits `data:` solely on media tags
  // (img/audio/video/source/track), where it cannot execute. javascript: URLs
  // are stripped by that same default.
  ALLOW_DATA_ATTR: false,
  // Disallow <a target="_blank"> from popping a new context window etc. —
  // EPUB renders inline; we don't need rich link semantics.
  ADD_ATTR: [],
  // NO `USE_PROFILES` HERE — DO NOT ADD IT BACK. Setting USE_PROFILES makes
  // DOMPurify ignore ALLOWED_TAGS and ALLOWED_ATTR entirely and fall back to
  // the profile's much broader default set. While `USE_PROFILES: {html:true}`
  // was set, every list above was inert: <video>, <audio>, <canvas>,
  // <marquee>, <progress>, <dialog> and <slot> all rendered despite not being
  // allowed, and every SVG tag listed was silently dropped despite being
  // allowed (svg lives in a different profile). On*= handlers are removed by
  // default, which is all USE_PROFILES was reached for.
};

/** Sanitize EPUB HTML for safe rendering (DREAD #10). */
export function sanitizeEpubHtml(html: string): string {
  return DOMPurify.sanitize(html, EPUB_SANITIZE_CONFIG);
}

// ── Shared pieces for the text-backed formats ────────────────────────

interface EditableProps {
  /** Switch to the text editor. Absent when the format cannot be edited. */
  onEditText?: () => void;
}

/**
 * One page, or two side by side.
 *
 * `unit` is what this format calls the thing being paired — a PDF has pages,
 * a chapter or a document laid out in columns has screens of text — so the
 * control says what it will actually do rather than borrowing a word from a
 * format that works differently.
 */
function LayoutToggle({
  layout,
  onChange,
  unit,
}: {
  layout: Layout;
  onChange: (next: Layout) => void;
  unit: "page" | "screen";
}) {
  const spread = layout === "spread";
  return (
    <button
      onClick={() => onChange(spread ? "single" : "spread")}
      className={`toolbar-btn-wide${spread ? " active" : ""}`}
      aria-pressed={spread}
      title={
        spread
          ? `Show one ${unit} at a time`
          : `Show two ${unit}s side by side`
      }
    >
      {spread ? <Square size={13} /> : <BookOpen size={13} />} {spread ? "One up" : "Two up"}
    </button>
  );
}

/**
 * The pager under a reflowable document laid out as two pages side by side.
 *
 * A DOCX or a chapter of an EPUB has no pages of its own — it is text, and how
 * much of it fits beside how much depends on the window and the font size. So
 * "two pages" here means two columns of one screen, and moving on means
 * scrolling the pane by exactly its own width. The pane keeps `overflow:
 * hidden` rather than a transform so that everything else still works on it:
 * `scrollIntoView` on a footnote anchor lands on the right column by itself.
 */
function PageStrip({ pane }: { pane: React.RefObject<HTMLDivElement | null> }) {
  const [{ screen, screens }, setPosition] = useState({ screen: 0, screens: 1 });

  useEffect(() => {
    const element = pane.current;
    if (!element) return;

    const measure = () => {
      const width = element.clientWidth || 1;
      // Rounded up, not to nearest: a last screen that is only part full is
      // still a screen, and rounding it away left the end of every chapter
      // unreachable. The few pixels of tolerance are for sub-pixel layout,
      // which otherwise reports a second screen for a document that fits.
      const screens = Math.max(1, Math.ceil((element.scrollWidth - 4) / width));
      const furthest = element.scrollWidth - width;
      // The last screen is usually only part full, so scrolling to it stops
      // short of a whole multiple of the width. Position alone would call that
      // the screen before, and the pager would sit on "1 of 2" forever.
      const screen =
        element.scrollLeft >= furthest - 2
          ? screens - 1
          : Math.round(element.scrollLeft / width);
      setPosition({ screen, screens });
    };
    measure();

    element.addEventListener("scroll", measure, { passive: true });
    // The column count, and so the screen count, changes with the pane's size
    // and with the font size — neither of which fires a scroll event.
    const observer =
      typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    observer?.observe(element);
    return () => {
      element.removeEventListener("scroll", measure);
      observer?.disconnect();
    };
  }, [pane]);

  const go = useCallback(
    (direction: 1 | -1) => {
      const element = pane.current;
      if (!element) return;
      const width = element.clientWidth || 1;
      element.scrollTo?.({ left: (screen + direction) * width, behavior: "smooth" });
    },
    [pane, screen],
  );

  return (
    <div className="reading-pager">
      <button onClick={() => go(-1)} disabled={screen <= 0} aria-label="Previous screen">
        <ChevronLeft size={14} />
      </button>
      <span className="zoom-label page-label">
        Screen {Math.min(screen + 1, screens)} of {screens}
      </span>
      <button
        onClick={() => go(1)}
        disabled={screen + 1 >= screens}
        aria-label="Next screen"
      >
        <ChevronRight size={14} />
      </button>
    </div>
  );
}

/** Remembered per file, so a tab switch does not undo how you chose to read. */
function useLayout(filePath: string): [Layout, (next: Layout) => void] {
  const [layout, setLayoutState] = useState<Layout>(() => getLayout(filePath) ?? "single");
  useEffect(() => setLayoutState(getLayout(filePath) ?? "single"), [filePath]);
  const set = useCallback(
    (next: Layout) => {
      setLayoutState(next);
      rememberLayout(filePath, next);
    },
    [filePath],
  );
  return [layout, set];
}

function EditTextButton({ onEditText }: { onEditText: () => void }) {
  return (
    <button onClick={onEditText} title="Edit this document as text" className="toolbar-btn-wide">
      <Pencil size={13} /> Edit text
    </button>
  );
}

/** What the reader could not represent, shown rather than swallowed. */
function WarningNotice({ warnings }: { warnings: DocumentWarning[] }) {
  if (warnings.length === 0) return null;
  return (
    <div className="doc-notice doc-notice-warn">
      <Info size={13} />
      <ul>
        {warnings.map((warning) => (
          <li key={warning.code}>{warning.message}</li>
        ))}
      </ul>
    </div>
  );
}

/** The document as text, loaded through the backend reader. */
type TextState =
  | { status: "loading" }
  | { status: "ready"; text: string; warnings: DocumentWarning[] }
  | { status: "failed"; message: string };

function useDocumentText(filePath: string): TextState {
  const [state, setState] = useState<TextState>({ status: "loading" });
  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    readDocumentText(filePath)
      .then((doc) => {
        if (!cancelled) setState({ status: "ready", text: doc.text, warnings: doc.warnings });
      })
      .catch((error) => {
        if (!cancelled) setState({ status: "failed", message: documentErrorMessage(error) });
      });
    return () => {
      cancelled = true;
    };
  }, [filePath]);
  return state;
}

function LoadingPane({ format }: { format: RichDocumentFormat }) {
  return (
    <div className="document-viewer">
      <div className="document-viewer-loading">
        <div className="doc-spinner" />
        <span>Reading {formatLabel(format)}…</span>
      </div>
    </div>
  );
}

function ErrorPane({ message }: { message: string }) {
  return (
    <div className="document-viewer">
      <div className="document-viewer-error">
        <AlertTriangle size={16} className="error-icon" />
        <span className="error-message">{message}</span>
      </div>
    </div>
  );
}

// ── DOCX Viewer ──────────────────────────────────────────────────────

/**
 * Word documents render through the same Markdown the text editor exposes, so
 * what is shown and what is editable cannot drift apart. Anything the reader
 * dropped on the way (images, footnotes) is named in the notice above — those
 * parts of the file are preserved on save, just not displayed here.
 */
function DocxViewer({ filePath, onEditText }: { filePath: string } & EditableProps) {
  const state = useDocumentText(filePath);
  const [fontSize, setFontSize] = useState(15);
  const [layout, setLayout] = useLayout(filePath);
  const pane = useRef<HTMLDivElement>(null);
  const fileName = filePath.split(/[/\\]/).pop() || filePath;

  if (state.status === "loading") return <LoadingPane format="docx" />;
  if (state.status === "failed") return <ErrorPane message={state.message} />;

  return (
    <div className="document-viewer docx-viewer">
      <div className="document-viewer-toolbar">
        <div className="toolbar-group">
          <button onClick={() => setFontSize((s) => Math.max(s - 1, 11))} title="Decrease Font Size">
            A−
          </button>
          <span className="zoom-label font-label">{fontSize}px</span>
          <button onClick={() => setFontSize((s) => Math.min(s + 1, 28))} title="Increase Font Size">
            A+
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <LayoutToggle layout={layout} onChange={setLayout} unit="screen" />
        </div>
        {onEditText && (
          <>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <EditTextButton onEditText={onEditText} />
            </div>
          </>
        )}
        <div className="file-info">
          <span className="info-badge">DOCX</span>
          <span className="info-badge">{fileName}</span>
        </div>
      </div>

      <WarningNotice warnings={state.warnings} />

      <div
        ref={pane}
        className={`docx-page${layout === "spread" ? " reading-paged" : ""}`}
        style={{ fontSize }}
      >
        <div className={layout === "spread" ? "reading-columns" : undefined}>
          {/* In a spread the pane does the scrolling, sideways. A preview that
              scrolled itself would be one unbreakable box, and every column
              after the first would be empty. */}
          <MarkdownPreview content={state.text} scrolls={layout === "single"} />
        </div>
      </div>
      {layout === "spread" && <PageStrip pane={pane} />}
    </div>
  );
}

// ── Pages Viewer ─────────────────────────────────────────────────────

/**
 * Apple ships no format specification for `.pages`, so this shows two honest
 * things instead of a fake rendering: the preview image Pages itself embedded
 * (what the document actually looks like), and the text recovered from the
 * archives (what can be edited). Neither pretends to be the other.
 */
function PagesViewer({ filePath, onEditText }: { filePath: string } & EditableProps) {
  const state = useDocumentText(filePath);
  const [preview, setPreview] = useState<string | null>(null);
  const [pane, setPane] = useState<"preview" | "text">("preview");
  const fileName = filePath.split(/[/\\]/).pop() || filePath;

  useEffect(() => {
    let cancelled = false;
    setPreview(null);
    readDocumentPreview(filePath)
      .then((image) => {
        if (cancelled || !image) {
          // No embedded preview: text is all there is, so start there.
          if (!cancelled) setPane("text");
          return;
        }
        setPreview(`data:${image.mime};base64,${image.base64}`);
      })
      .catch(() => {
        if (!cancelled) setPane("text");
      });
    return () => {
      cancelled = true;
    };
  }, [filePath]);

  if (state.status === "loading") return <LoadingPane format="pages" />;
  if (state.status === "failed") return <ErrorPane message={state.message} />;

  const showPreview = pane === "preview" && preview !== null;

  return (
    <div className="document-viewer pages-viewer">
      <div className="document-viewer-toolbar">
        <div className="toolbar-group">
          <button
            onClick={() => setPane("preview")}
            disabled={preview === null}
            title={preview === null ? "This document embeds no preview image" : "Page preview"}
            className={`toolbar-btn-wide${showPreview ? " active" : ""}`}
          >
            Preview
          </button>
          <button
            onClick={() => setPane("text")}
            title="Recovered text"
            className={`toolbar-btn-wide${showPreview ? "" : " active"}`}
          >
            Text
          </button>
        </div>
        {onEditText && (
          <>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <EditTextButton onEditText={onEditText} />
            </div>
          </>
        )}
        <div className="file-info">
          <span className="info-badge">Pages</span>
          <span className="info-badge">{fileName}</span>
        </div>
      </div>

      <WarningNotice warnings={state.warnings} />

      {showPreview ? (
        <div className="pages-preview">
          <img src={preview ?? ""} alt={`Preview of ${fileName}`} />
        </div>
      ) : (
        <div className="pages-text">
          {state.text.split("\n").map((line, i) => (
            <p key={i} className={line.trim() === "" ? "pages-blank" : undefined}>
              {line}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Main Component ───────────────────────────────────────────────────

export function DocumentViewer({ filePath, base64Data }: DocumentViewerProps) {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  const editableFormat = richDocumentFormat(filePath);
  // Where this document was left, so a tab switch returns to the same pane —
  // and never lands on the rendered view while an unsaved edit is waiting.
  const [mode, setModeState] = useState<"view" | "text">(
    () => getMode(filePath) ?? "view",
  );

  const setMode = useCallback(
    (next: "view" | "text") => {
      rememberMode(filePath, next);
      setModeState(next);
    },
    [filePath],
  );

  useEffect(() => {
    setModeState(getMode(filePath) ?? (hasDraft(filePath) ? "text" : "view"));
  }, [filePath]);

  if (mode === "text" && editableFormat) {
    return (
      <DocumentTextEditor
        filePath={filePath}
        format={editableFormat}
        onClose={() => setMode("view")}
      />
    );
  }

  const onEditText = editableFormat ? () => setMode("text") : undefined;

  if (ext === "pdf") {
    return <PdfViewer filePath={filePath} base64Data={base64Data} onEditText={onEditText} />;
  }

  if (ext === "epub") {
    return <EpubViewer filePath={filePath} onEditText={onEditText} />;
  }

  if (ext === "docx") {
    return <DocxViewer filePath={filePath} onEditText={onEditText} />;
  }

  if (ext === "pages") {
    return <PagesViewer filePath={filePath} onEditText={onEditText} />;
  }

  return (
    <div className="document-viewer">
      <div className="document-viewer-error">
        <FileText size={20} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />
        <span className="error-message">Unsupported document format: .{ext}</span>
      </div>
    </div>
  );
}
