import { describe, it, expect } from 'vitest';
import { extractFencedBlock, prepareDrawioXml, stripModuleSyntax } from '../llmOutput';

describe('extractFencedBlock', () => {
  it('returns empty for empty input', () => {
    expect(extractFencedBlock('')).toBe('');
  });

  it('extracts a ```tsx block', () => {
    expect(extractFencedBlock('```tsx\nconst A = 1;\n```')).toBe('const A = 1;');
  });

  it('extracts a ```xml block (drawio diagram case)', () => {
    const xml = '<mxGraphModel><root><mxCell id="0"/></root></mxGraphModel>';
    expect(extractFencedBlock('```xml\n' + xml + '\n```')).toBe(xml);
  });

  it('discards prose surrounding the fenced block', () => {
    const input = [
      "Since you haven't specified a project, here's a generic diagram:",
      '',
      '```xml',
      '<mxGraphModel><root/></mxGraphModel>',
      '```',
      '',
      'If you want a specific one, let me know.',
    ].join('\n');
    const out = extractFencedBlock(input);
    expect(out).toBe('<mxGraphModel><root/></mxGraphModel>');
    expect(out).not.toContain("Since you haven't");
    expect(out).not.toContain('let me know');
  });

  it('handles CRLF line endings', () => {
    expect(extractFencedBlock('```ts\r\nconst A = 1;\r\n```')).toBe('const A = 1;');
  });

  it('returns first block when several are present', () => {
    const input = '```ts\nconst First = 1;\n```\n\n```ts\nconst Second = 2;\n```';
    expect(extractFencedBlock(input)).toBe('const First = 1;');
  });

  it('falls back to a defensive sweep when no closed fence exists', () => {
    expect(extractFencedBlock('```tsx\nconst Half = (')).toBe('const Half = (');
  });

  it('returns input unchanged when there are no fences', () => {
    expect(extractFencedBlock('const A = 1;')).toBe('const A = 1;');
  });
});

describe('prepareDrawioXml', () => {
  const wellFormed =
    '<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/></root></mxGraphModel>';

  it('rejects empty input', () => {
    const r = prepareDrawioXml('');
    expect(r.ok).toBe(false);
    expect(r.warning).toMatch(/empty/i);
  });

  it('rejects non-drawio XML', () => {
    const r = prepareDrawioXml('<svg><rect/></svg>');
    expect(r.ok).toBe(false);
    expect(r.warning).toMatch(/<mxGraphModel>/);
  });

  it('flags truncated XML missing the closing tag (the AI-truncation case)', () => {
    const truncated = '<mxGraphModel><root><mxCell id="0"/><mxCell id="user" value="User"';
    const r = prepareDrawioXml(truncated);
    expect(r.ok).toBe(false);
    expect(r.warning).toMatch(/truncated/i);
    expect(r.warning).toMatch(/max-token/i);
  });

  it('accepts well-formed mxGraphModel and wraps it in mxfile', () => {
    const r = prepareDrawioXml(wellFormed);
    expect(r.ok).toBe(true);
    expect(r.warning).toBeUndefined();
    expect(r.prepared).toMatch(/^<mxfile\b/);
    expect(r.prepared).toContain('<diagram');
    expect(r.prepared).toContain(wellFormed);
    expect(r.prepared).toMatch(/<\/mxfile>$/);
  });

  it('passes through XML that is already in mxfile form', () => {
    const file = `<mxfile><diagram name="P">${wellFormed}</diagram></mxfile>`;
    const r = prepareDrawioXml(file);
    expect(r.ok).toBe(true);
    expect(r.prepared).toBe(file);
  });

  it('trims surrounding whitespace before validating', () => {
    const r = prepareDrawioXml('   \n' + wellFormed + '\n  ');
    expect(r.ok).toBe(true);
  });
});

describe('stripModuleSyntax', () => {
  it('removes side-effect imports', () => {
    expect(stripModuleSyntax("import './styles.css';\nconst A = 1;").trim())
      .toBe('const A = 1;');
  });

  it('removes named/default/star imports', () => {
    const cases = [
      "import React from 'react';",
      "import { useState } from 'react';",
      "import * as React from 'react';",
      "import React, { useState, useEffect } from 'react';",
    ];
    for (const c of cases) {
      const out = stripModuleSyntax(c + '\nconst A = 1;');
      expect(out).not.toContain('import');
      expect(out).toContain('const A = 1;');
    }
  });

  it('strips export keyword from declarations', () => {
    expect(stripModuleSyntax('export const A = 1;')).toBe('const A = 1;');
    expect(stripModuleSyntax('export function App() {}')).toBe('function App() {}');
  });

  it('strips export-list and re-export forms', () => {
    const cases = [
      'export { Foo };',
      'export { Foo, Bar };',
      'export { Foo as Bar };',
      "export { Foo } from './foo';",
      "export * from './foo';",
      "export * as Ns from './foo';",
    ];
    for (const c of cases) {
      const input = 'const Foo = 1;\n' + c;
      expect(stripModuleSyntax(input).trim()).toBe('const Foo = 1;');
    }
  });
});
