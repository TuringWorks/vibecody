/**
 * epubBook — the boundary between the EPUB viewer and the backend reader.
 *
 * The book used to be parsed in the browser by a hand-rolled ZIP reader whose
 * inflate function returned `null` unconditionally, so every deflate-compressed
 * entry was dropped — which is every chapter of every real EPUB. Reading now
 * happens in Rust, and a chapter arrives with the stylesheets and images it
 * references already resolved against its own directory.
 *
 * Responses are parsed here rather than cast, for the reason in
 * [richDocuments.ts]: a wrong assumption about the payload should fail at the
 * boundary naming the field, not three renders later.
 */
import { invoke } from "@tauri-apps/api/core";

import type { DocumentWarning } from "./richDocuments";

/** A file from inside the book. */
export interface EpubResource {
  /** Container path, e.g. `OEBPS/images/fig1.png`. */
  path: string;
  /** How the chapter referred to it, e.g. `../images/fig1.png`. */
  href: string;
  mime: string;
  base64: string;
}

export interface EpubChapterRef {
  path: string;
  title: string | null;
}

export interface EpubTocEntry {
  label: string;
  path: string;
  fragment: string | null;
  level: number;
}

export interface EpubBook {
  title: string | null;
  authors: string[];
  language: string | null;
  publisher: string | null;
  chapters: EpubChapterRef[];
  toc: EpubTocEntry[];
  cover: EpubResource | null;
  warnings: DocumentWarning[];
}

export interface EpubChapter {
  path: string;
  title: string | null;
  /** Unsanitised body markup — sanitise before it reaches the DOM. */
  html: string;
  css: string;
  resources: EpubResource[];
  warnings: DocumentWarning[];
}

// ── Parsing ──────────────────────────────────────────────────────────

class ShapeError extends Error {
  constructor(field: string, value: unknown) {
    super(`epub response field "${field}" is ${describe(value)}`);
    this.name = "ShapeError";
  }
}

function describe(value: unknown): string {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  return typeof value;
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ShapeError(field, value);
  }
  return value as Record<string, unknown>;
}

function str(source: Record<string, unknown>, field: string): string {
  const value = source[field];
  if (typeof value !== "string") throw new ShapeError(field, value);
  return value;
}

function optionalStr(source: Record<string, unknown>, field: string): string | null {
  const value = source[field];
  if (value === null || value === undefined) return null;
  if (typeof value !== "string") throw new ShapeError(field, value);
  return value;
}

function num(source: Record<string, unknown>, field: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new ShapeError(field, value);
  return value;
}

function list(source: Record<string, unknown>, field: string): unknown[] {
  const value = source[field];
  if (!Array.isArray(value)) throw new ShapeError(field, value);
  return value;
}

function parseWarnings(source: Record<string, unknown>): DocumentWarning[] {
  return list(source, "warnings").map((entry, i) => {
    const warning = record(entry, `warnings[${i}]`);
    return { code: str(warning, "code"), message: str(warning, "message") };
  });
}

function parseResource(value: unknown, field: string): EpubResource {
  const source = record(value, field);
  return {
    path: str(source, "path"),
    href: str(source, "href"),
    mime: str(source, "mime"),
    base64: str(source, "base64"),
  };
}

export function parseEpubBook(value: unknown): EpubBook {
  const source = record(value, "book");
  return {
    title: optionalStr(source, "title"),
    authors: list(source, "authors").map((author, i) => {
      if (typeof author !== "string") throw new ShapeError(`authors[${i}]`, author);
      return author;
    }),
    language: optionalStr(source, "language"),
    publisher: optionalStr(source, "publisher"),
    chapters: list(source, "chapters").map((entry, i) => {
      const chapter = record(entry, `chapters[${i}]`);
      return { path: str(chapter, "path"), title: optionalStr(chapter, "title") };
    }),
    toc: list(source, "toc").map((entry, i) => {
      const item = record(entry, `toc[${i}]`);
      return {
        label: str(item, "label"),
        path: str(item, "path"),
        fragment: optionalStr(item, "fragment"),
        level: num(item, "level"),
      };
    }),
    cover:
      source.cover === null || source.cover === undefined
        ? null
        : parseResource(source.cover, "cover"),
    warnings: parseWarnings(source),
  };
}

export function parseEpubChapter(value: unknown): EpubChapter {
  const source = record(value, "chapter");
  return {
    path: str(source, "path"),
    title: optionalStr(source, "title"),
    html: str(source, "html"),
    css: str(source, "css"),
    resources: list(source, "resources").map((entry, i) =>
      parseResource(entry, `resources[${i}]`),
    ),
    warnings: parseWarnings(source),
  };
}

// ── Commands ─────────────────────────────────────────────────────────

export async function readEpubBook(path: string): Promise<EpubBook> {
  return parseEpubBook(await invoke("read_epub_book", { path }));
}

export async function readEpubChapter(path: string, chapter: string): Promise<EpubChapter> {
  return parseEpubChapter(await invoke("read_epub_chapter", { path, chapter }));
}

// ── Resources ────────────────────────────────────────────────────────

/**
 * Turn a chapter's resources into object URLs the DOM can load.
 *
 * Keyed by both the container path and the href as the chapter wrote it, since
 * markup and stylesheets refer to the same file in different ways.
 */
export function resourceUrls(resources: EpubResource[]): {
  urls: Map<string, string>;
  revoke: () => void;
} {
  const urls = new Map<string, string>();
  const created: string[] = [];
  for (const resource of resources) {
    // An unknown type is not rendered: a book cannot talk the viewer into
    // treating an arbitrary file as something it is not.
    if (resource.mime === "application/octet-stream") continue;
    try {
      const url = URL.createObjectURL(toBlob(resource));
      created.push(url);
      urls.set(resource.path, url);
      urls.set(resource.href, url);
    } catch {
      // A resource that will not decode simply does not render.
    }
  }
  return {
    urls,
    revoke: () => created.forEach((url) => URL.revokeObjectURL(url)),
  };
}

export function toBlob(resource: EpubResource): Blob {
  const binary = atob(resource.base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return new Blob([bytes], { type: resource.mime });
}

/** A `data:` URL for a resource — for an `<img>` that outlives one chapter. */
export function dataUrl(resource: EpubResource): string {
  return `data:${resource.mime};base64,${resource.base64}`;
}
