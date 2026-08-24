/**
 * The document boundary parses, it does not cast.
 *
 * A Tauri command hands back `unknown`. Asserting an interface onto it moves
 * the failure into a render — a blank panel, a `Cannot read properties of
 * undefined`, and no clue which field was wrong. These pin that a malformed
 * payload fails here, naming the field, and that a well-formed one comes back
 * with the shape the panels rely on.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  RICH_DOCUMENT_EXTENSIONS,
  documentErrorMessage,
  formatLabel,
  parseDocumentPreview,
  parseDocumentText,
  parseDocumentWrite,
  readDocumentText,
  richDocumentFormat,
  writeDocumentText,
} from '../richDocuments';

const textPayload = (over: Record<string, unknown> = {}) => ({
  format: 'docx',
  language: 'markdown',
  text: '# Title\n',
  sections: 1,
  warnings: [{ code: 'docx.code_block_flattened', message: 'code became paragraphs' }],
  writable: true,
  ...over,
});

const writePayload = (over: Record<string, unknown> = {}) => ({
  format: 'pages',
  bytes_written: 2048,
  backup: '/docs/memo.pages.bak',
  warnings: [],
  verified: true,
  ...over,
});

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('richDocumentFormat', () => {
  it('recognises the three editable formats, case-insensitively', () => {
    expect(richDocumentFormat('report.docx')).toBe('docx');
    expect(richDocumentFormat('Book.EPUB')).toBe('epub');
    expect(richDocumentFormat('/a/b/memo.pages')).toBe('pages');
  });

  it('does not claim formats it cannot edit', () => {
    expect(richDocumentFormat('paper.pdf')).toBeNull();
    expect(richDocumentFormat('notes.md')).toBeNull();
    expect(richDocumentFormat('no-extension')).toBeNull();
  });

  it('labels every format it advertises', () => {
    for (const ext of RICH_DOCUMENT_EXTENSIONS) {
      expect(formatLabel(ext)).not.toBe('');
    }
  });
});

describe('parseDocumentText', () => {
  it('returns the parsed response for a well-formed payload', () => {
    const parsed = parseDocumentText(textPayload());
    expect(parsed.format).toBe('docx');
    expect(parsed.language).toBe('markdown');
    expect(parsed.sections).toBe(1);
    expect(parsed.warnings[0].code).toBe('docx.code_block_flattened');
    expect(parsed.writable).toBe(true);
  });

  it('names the field when one is missing', () => {
    const { text: _dropped, ...withoutText } = textPayload();
    expect(() => parseDocumentText(withoutText)).toThrow(/"text"/);
  });

  it('rejects a format this build does not handle', () => {
    expect(() => parseDocumentText(textPayload({ format: 'rtf' }))).toThrow(/"format"/);
  });

  it('rejects a warning list that is not warnings', () => {
    expect(() => parseDocumentText(textPayload({ warnings: ['just a string'] }))).toThrow(
      /warnings\[0\]/,
    );
  });

  it('rejects a null response instead of dereferencing it later', () => {
    expect(() => parseDocumentText(null)).toThrow(/"response"/);
  });
});

describe('parseDocumentWrite', () => {
  it('maps the snake_case wire field to the camelCase one panels use', () => {
    const parsed = parseDocumentWrite(writePayload());
    expect(parsed.bytesWritten).toBe(2048);
    expect(parsed.backup).toBe('/docs/memo.pages.bak');
    expect(parsed.verified).toBe(true);
  });

  it('treats an absent backup as no backup, not as a missing field', () => {
    const parsed = parseDocumentWrite(writePayload({ backup: null }));
    expect(parsed.backup).toBeNull();
  });

  it('rejects a byte count that is not a number', () => {
    expect(() => parseDocumentWrite(writePayload({ bytes_written: '2048' }))).toThrow(
      /"bytes_written"/,
    );
  });

  it('rejects a verified flag that is not a boolean', () => {
    // `verified: "true"` would read as truthy and turn an unverified write into
    // a reported success — the one thing this whole path exists to prevent.
    expect(() => parseDocumentWrite(writePayload({ verified: 'true' }))).toThrow(/"verified"/);
  });
});

describe('parseDocumentPreview', () => {
  it('returns null when the document embeds no preview', () => {
    expect(parseDocumentPreview(null)).toBeNull();
    expect(parseDocumentPreview(undefined)).toBeNull();
  });

  it('parses an embedded preview', () => {
    const parsed = parseDocumentPreview({ mime: 'image/jpeg', base64: 'AAA' });
    expect(parsed).toEqual({ mime: 'image/jpeg', base64: 'AAA' });
  });
});

describe('commands', () => {
  it('passes the path through and parses what comes back', async () => {
    mockInvoke.mockResolvedValueOnce(textPayload());
    const doc = await readDocumentText('/docs/report.docx');
    expect(mockInvoke).toHaveBeenCalledWith('read_document_text', { path: '/docs/report.docx' });
    expect(doc.text).toBe('# Title\n');
  });

  it('sends the edited text and reports the verified write', async () => {
    mockInvoke.mockResolvedValueOnce(writePayload());
    const report = await writeDocumentText('/docs/memo.pages', 'new text\n');
    expect(mockInvoke).toHaveBeenCalledWith('write_document_text', {
      path: '/docs/memo.pages',
      text: 'new text\n',
    });
    expect(report.verified).toBe(true);
  });

  it('surfaces a backend refusal as its message', async () => {
    mockInvoke.mockRejectedValueOnce(
      'write verification failed: the rewritten document does not read back as the text you saved. Your file has not been changed.',
    );
    await expect(writeDocumentText('/docs/memo.pages', 'x')).rejects.toMatch(
      /has not been changed/,
    );
  });
});

describe('documentErrorMessage', () => {
  it('keeps the backend string as-is', () => {
    expect(documentErrorMessage('parse error: no <w:body>')).toBe('parse error: no <w:body>');
  });

  it('unwraps an Error and stringifies anything else', () => {
    expect(documentErrorMessage(new Error('boom'))).toBe('boom');
    expect(documentErrorMessage({ odd: true })).toBe('[object Object]');
  });
});
