/**
 * DocumentTextEditor — edits a DOCX, EPUB, PDF or Pages document as text in
 * Monaco and saves it back into the original file.
 *
 * The buffer is Markdown for DOCX and EPUB, plain text for Pages and PDF; the
 * backend decides which and says so, because those two recover the words
 * without the emphasis, and offering Markdown there would mean typing
 * `**bold**` into a document that stores it literally.
 *
 * Two things this panel refuses to fake:
 *   • A save is only reported when the backend re-read the file and found the
 *     text it wrote. Anything else surfaces as an error with the file untouched.
 *   • Every limitation the reader or writer reported is shown, not swallowed.
 *   • An unsaved buffer survives a tab switch. The editor area renders only the
 *     active file, so without the draft store an edit would disappear the
 *     moment another tab was clicked, with the document still showing its old
 *     text and nothing on screen saying so.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import { MONACO_OVERFLOW_OPTIONS } from "../lib/monacoOptions";
import { AlertTriangle, Check, Eye, Info, Save } from "lucide-react";

import { useEditorTheme } from "../hooks/useEditorTheme";
import { clearDraft, getDraft, setDraft } from "../lib/documentDrafts";
import {
  documentErrorMessage,
  formatLabel,
  readDocumentText,
  writeDocumentText,
  type DocumentWarning,
  type RichDocumentFormat,
} from "../lib/richDocuments";
import "./DocumentViewer.css";

interface DocumentTextEditorProps {
  /** Absolute path of the document. */
  filePath: string;
  format: RichDocumentFormat;
  /** Return to the rendered view. */
  onClose: () => void;
}

/** What a format calls the parts its buffer is divided into. */
function sectionNoun(format: RichDocumentFormat, count: number): string {
  const noun =
    format === "pdf" ? "page" : format === "epub" ? "chapter" : "section";
  return count === 1 ? noun : `${noun}s`;
}

/** What the panel is doing, as one value rather than four booleans. */
type Status =
  | { state: "loading" }
  | { state: "ready" }
  | { state: "saving" }
  | { state: "saved"; detail: string }
  | { state: "failed"; message: string };

export function DocumentTextEditor({ filePath, format, onClose }: DocumentTextEditorProps) {
  const { themeName, defineTheme } = useEditorTheme();

  const [status, setStatus] = useState<Status>({ state: "loading" });
  const [text, setText] = useState("");
  const [savedText, setSavedText] = useState("");
  const [language, setLanguage] = useState("plaintext");
  const [sections, setSections] = useState(1);
  const [writable, setWritable] = useState(false);
  const [readWarnings, setReadWarnings] = useState<DocumentWarning[]>([]);
  const [writeWarnings, setWriteWarnings] = useState<DocumentWarning[]>([]);

  const fileName = filePath.split(/[/\\]/).pop() || filePath;
  const isDirty = text !== savedText;

  // Saving reads the live buffer, so the shortcut handler must not close over
  // a stale copy of it.
  const latest = useRef({ text, savedText, writable });
  latest.current = { text, savedText, writable };

  useEffect(() => {
    let cancelled = false;
    setStatus({ state: "loading" });
    readDocumentText(filePath)
      .then((doc) => {
        if (cancelled) return;
        // A draft from an earlier visit to this tab wins over the file: it is
        // the edit the person made and has not saved yet.
        setText(getDraft(filePath) ?? doc.text);
        setSavedText(doc.text);
        setLanguage(doc.language);
        setSections(doc.sections);
        setWritable(doc.writable);
        setReadWarnings(doc.warnings);
        setWriteWarnings([]);
        setStatus({ state: "ready" });
      })
      .catch((error) => {
        if (cancelled) return;
        setStatus({ state: "failed", message: documentErrorMessage(error) });
      });
    return () => {
      cancelled = true;
    };
  }, [filePath]);

  const save = useCallback(async () => {
    const { text: current, savedText: saved, writable: canWrite } = latest.current;
    if (!canWrite || current === saved) return;
    setStatus({ state: "saving" });
    try {
      const report = await writeDocumentText(filePath, current);
      setSavedText(current);
      clearDraft(filePath);
      setWriteWarnings(report.warnings);
      const backup = report.backup
        ? ` · original copied to ${report.backup.split(/[/\\]/).pop()}`
        : "";
      setStatus({
        state: "saved",
        detail: `${report.bytesWritten.toLocaleString()} bytes, verified${backup}`,
      });
    } catch (error) {
      setStatus({ state: "failed", message: documentErrorMessage(error) });
    }
  }, [filePath]);

  const handleMount: OnMount = useCallback(
    (editor, monaco) => {
      defineTheme(monaco);
      // Cmd/Ctrl+S saves the document, not the raw container.
      editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
        void save();
      });
    },
    [defineTheme, save],
  );

  const bannerWarnings = useMemo(
    () => dedupeByCode([...readWarnings, ...writeWarnings]),
    [readWarnings, writeWarnings],
  );

  if (status.state === "loading") {
    return (
      <div className="document-viewer">
        <div className="document-viewer-loading">
          <div className="doc-spinner" />
          <span>Reading {formatLabel(format)}…</span>
        </div>
      </div>
    );
  }

  return (
    <div className="document-viewer document-text-editor">
      <div className="document-viewer-toolbar">
        <div className="toolbar-group">
          <button onClick={onClose} title="Back to the rendered document" className="toolbar-btn-wide">
            <Eye size={13} /> View
          </button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <button
            onClick={() => void save()}
            disabled={!writable || !isDirty || status.state === "saving"}
            title={writable ? "Save into the document (⌘S)" : "This document cannot be written"}
            className="toolbar-btn-wide"
          >
            <Save size={13} /> {status.state === "saving" ? "Saving…" : "Save"}
          </button>
        </div>
        <div className="file-info">
          <span className="info-badge">{formatLabel(format)}</span>
          <span className="info-badge">{fileName}</span>
          {isDirty && <span className="info-badge doc-badge-dirty">unsaved</span>}
          {sections > 1 && (
            <span className="info-badge" title="Keep every section marker in the buffer: they route each edit back to its chapter, page or storage.">
              {sections} {sectionNoun(format, sections)}
            </span>
          )}
        </div>
      </div>

      {format === "pdf" && (
        <div className="doc-notice">
          <Info size={13} />
          <span>
            A PDF places every glyph at a fixed position and does not re-flow.
            You can change a line's words or clear the line; adding a line has
            nowhere to go, and a longer line runs past where the original ended.
          </span>
        </div>
      )}

      {status.state === "failed" && (
        <div className="document-viewer-error doc-inline-error">
          <AlertTriangle size={14} className="error-icon" />
          <span className="error-message">{status.message}</span>
        </div>
      )}

      {status.state === "saved" && !isDirty && (
        <div className="doc-notice doc-notice-ok">
          <Check size={13} />
          <span>Saved — {status.detail}</span>
        </div>
      )}

      {bannerWarnings.length > 0 && (
        <div className="doc-notice doc-notice-warn">
          <Info size={13} />
          <ul>
            {bannerWarnings.map((warning) => (
              <li key={warning.code}>{warning.message}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="document-text-editor-surface">
        <Editor
          height="100%"
          /* A `file://` path would collide with the model App.tsx opens for the
             same file; this buffer is a different text than the bytes on disk. */
          path={`vibedoc://${filePath}`}
          language={language}
          theme={themeName}
          value={text}
          onChange={(value) => {
            const next = value ?? "";
            setText(next);
            setDraft(filePath, next, savedText);
          }}
          onMount={handleMount}
          options={{
            ...MONACO_OVERFLOW_OPTIONS,
            readOnly: !writable,
            minimap: { enabled: false },
            fontSize: 14,
            wordWrap: "on",
            lineNumbers: "on",
            scrollBeyondLastLine: false,
            automaticLayout: true,
            renderWhitespace: "selection",
          }}
        />
      </div>
    </div>
  );
}

/** One line per distinct limitation, however many times it was reported. */
function dedupeByCode(warnings: DocumentWarning[]): DocumentWarning[] {
  const seen = new Map<string, DocumentWarning>();
  for (const warning of warnings) {
    if (!seen.has(warning.code)) seen.set(warning.code, warning);
  }
  return [...seen.values()];
}

export default DocumentTextEditor;
