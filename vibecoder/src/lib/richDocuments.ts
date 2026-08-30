/**
 * richDocuments — the boundary between the editor and the `vibe-docfmt`
 * backend, which opens DOCX, EPUB, PDF and Apple Pages documents as editable
 * text.
 *
 * Every response is *parsed* here, not cast. A Tauri command returns `unknown`,
 * and an interface assertion on a payload that turned out to be shaped
 * differently fails later, inside a render, as a blank panel with a cryptic
 * message. Parsing fails here instead, with the field that was wrong.
 */
import { invoke } from "@tauri-apps/api/core";

/** Formats that open as text and save back into their original container. */
export const RICH_DOCUMENT_EXTENSIONS = ["docx", "epub", "pages", "pdf"] as const;

export type RichDocumentFormat = (typeof RICH_DOCUMENT_EXTENSIONS)[number];

/** The format for a filename, or null when it is not one of ours. */
export function richDocumentFormat(filename: string): RichDocumentFormat | null {
  const ext = filename.split(".").pop()?.toLowerCase() ?? "";
  return (RICH_DOCUMENT_EXTENSIONS as readonly string[]).includes(ext)
    ? (ext as RichDocumentFormat)
    : null;
}

/** Human label for a format badge. */
export function formatLabel(format: RichDocumentFormat): string {
  switch (format) {
    case "docx":
      return "DOCX";
    case "epub":
      return "EPUB";
    case "pages":
      return "Pages";
    case "pdf":
      return "PDF";
  }
}

/** Something the reader or writer could not do faithfully. */
export interface DocumentWarning {
  code: string;
  message: string;
}

/** A document opened as an editable text buffer. */
export interface DocumentTextResponse {
  format: RichDocumentFormat;
  /** Monaco language id: `markdown` or `plaintext`. */
  language: string;
  text: string;
  /** Chapters (EPUB), text storages (Pages) or pages (PDF) in the buffer. */
  sections: number;
  warnings: DocumentWarning[];
  writable: boolean;
}

/** The outcome of saving an edited buffer. */
export interface DocumentWriteResponse {
  format: RichDocumentFormat;
  bytesWritten: number;
  /** Where the pre-edit copy was kept, when the writer made one. */
  backup: string | null;
  warnings: DocumentWarning[];
  verified: boolean;
}

/** A preview image embedded in the document. */
export interface DocumentPreview {
  mime: string;
  base64: string;
}

// ── Parsing ──────────────────────────────────────────────────────────

class ShapeError extends Error {
  constructor(field: string, value: unknown) {
    super(`document response field "${field}" is ${describe(value)}`);
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

function num(source: Record<string, unknown>, field: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new ShapeError(field, value);
  }
  return value;
}

function bool(source: Record<string, unknown>, field: string): boolean {
  const value = source[field];
  if (typeof value !== "boolean") throw new ShapeError(field, value);
  return value;
}

function format(source: Record<string, unknown>): RichDocumentFormat {
  const value = str(source, "format");
  if (!(RICH_DOCUMENT_EXTENSIONS as readonly string[]).includes(value)) {
    throw new ShapeError("format", value);
  }
  return value as RichDocumentFormat;
}

function warnings(source: Record<string, unknown>): DocumentWarning[] {
  const value = source.warnings;
  if (!Array.isArray(value)) throw new ShapeError("warnings", value);
  return value.map((entry, i) => {
    const warning = record(entry, `warnings[${i}]`);
    return { code: str(warning, "code"), message: str(warning, "message") };
  });
}

export function parseDocumentText(value: unknown): DocumentTextResponse {
  const source = record(value, "response");
  return {
    format: format(source),
    language: str(source, "language"),
    text: str(source, "text"),
    sections: num(source, "sections"),
    warnings: warnings(source),
    writable: bool(source, "writable"),
  };
}

export function parseDocumentWrite(value: unknown): DocumentWriteResponse {
  const source = record(value, "response");
  const backup = source.backup;
  if (backup !== null && backup !== undefined && typeof backup !== "string") {
    throw new ShapeError("backup", backup);
  }
  return {
    format: format(source),
    bytesWritten: num(source, "bytes_written"),
    backup: typeof backup === "string" ? backup : null,
    warnings: warnings(source),
    verified: bool(source, "verified"),
  };
}

export function parseDocumentPreview(value: unknown): DocumentPreview | null {
  if (value === null || value === undefined) return null;
  const source = record(value, "preview");
  return { mime: str(source, "mime"), base64: str(source, "base64") };
}

// ── Commands ─────────────────────────────────────────────────────────

/** Open a document as text. */
export async function readDocumentText(path: string): Promise<DocumentTextResponse> {
  return parseDocumentText(await invoke("read_document_text", { path }));
}

/** Save edited text back into the document it came from. */
export async function writeDocumentText(
  path: string,
  text: string,
): Promise<DocumentWriteResponse> {
  return parseDocumentWrite(await invoke("write_document_text", { path, text }));
}

/** The preview image the document embeds, if any. */
export async function readDocumentPreview(path: string): Promise<DocumentPreview | null> {
  return parseDocumentPreview(await invoke("read_document_preview", { path }));
}

/** Turn an error from a command into something worth showing a person. */
export function documentErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
