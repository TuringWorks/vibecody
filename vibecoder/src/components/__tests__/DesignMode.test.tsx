import { describe, it, expect } from 'vitest';
import { extractGeneratedCode, stripModuleSyntax } from '../DesignMode';

describe('extractGeneratedCode', () => {
  it('returns empty for empty input', () => {
    expect(extractGeneratedCode('')).toBe('');
  });

  it('strips a simple ```tsx fence', () => {
    const input = '```tsx\nconst App = () => <div />;\n```';
    expect(extractGeneratedCode(input)).toBe('const App = () => <div />;');
  });

  it('strips fences with a variety of language tags', () => {
    const cases = ['```ts', '```jsx', '```javascript', '```typescript', '```TSX', '```react-tsx'];
    for (const fence of cases) {
      const out = extractGeneratedCode(`${fence}\nconst A = 1;\n\`\`\``);
      expect(out).toBe('const A = 1;');
    }
  });

  it('strips a fence with no language tag', () => {
    expect(extractGeneratedCode('```\nconst A = 1;\n```')).toBe('const A = 1;');
  });

  it('discards prose surrounding the fenced block', () => {
    const input = [
      "Here's a React component for a button:",
      '',
      '```tsx',
      'const Button = () => <button>Click</button>;',
      '```',
      '',
      'Let me know if you want to tweak the styling.',
    ].join('\n');
    const out = extractGeneratedCode(input);
    expect(out).toBe('const Button = () => <button>Click</button>;');
    expect(out).not.toContain("Here's");
    expect(out).not.toContain('Let me know');
  });

  it('handles CRLF line endings', () => {
    const input = '```tsx\r\nconst A = 1;\r\n```';
    expect(extractGeneratedCode(input)).toBe('const A = 1;');
  });

  it('returns the FIRST fenced block when multiple are present', () => {
    const input = [
      '```tsx',
      'const First = 1;',
      '```',
      '',
      '```tsx',
      'const Second = 2;',
      '```',
    ].join('\n');
    expect(extractGeneratedCode(input)).toBe('const First = 1;');
  });

  it('falls back to stripping stray fence markers when no closed fence exists', () => {
    // Streaming truncated mid-block, no closing ```.
    const input = '```tsx\nconst Half = (';
    const out = extractGeneratedCode(input);
    expect(out).toBe('const Half = (');
    expect(out).not.toContain('```');
  });

  it('returns code unchanged when no fences are present', () => {
    expect(extractGeneratedCode('const A = 1;')).toBe('const A = 1;');
  });

  it('preserves internal whitespace and JSX structure', () => {
    const code = [
      'const App = () => {',
      '  return (',
      '    <div>',
      '      <span>hi</span>',
      '    </div>',
      '  );',
      '};',
    ].join('\n');
    const input = '```tsx\n' + code + '\n```';
    expect(extractGeneratedCode(input)).toBe(code);
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

  it('strips the export keyword from declarations', () => {
    expect(stripModuleSyntax('export const A = 1;')).toBe('const A = 1;');
    expect(stripModuleSyntax('export function App() {}')).toBe('function App() {}');
    expect(stripModuleSyntax('export class App {}')).toBe('class App {}');
    expect(stripModuleSyntax('export interface I {}')).toBe('interface I {}');
    expect(stripModuleSyntax('export type T = number;')).toBe('type T = number;');
    expect(stripModuleSyntax('export enum E { A }')).toBe('enum E { A }');
  });

  it('strips `export default` keyword, keeping the value', () => {
    expect(stripModuleSyntax('export default function App() {}')).toBe('function App() {}');
    // `export default Foo;` becomes a no-op expression statement.
    expect(stripModuleSyntax('export default Foo;')).toBe('Foo;');
  });

  it('removes export-list and re-export forms (the bug that caused "Unexpected keyword export")', () => {
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
      const out = stripModuleSyntax(input).trim();
      expect(out).toBe('const Foo = 1;');
      expect(out).not.toMatch(/\bexport\b/);
    }
  });

  it('handles a realistic LLM emission with imports + named export at the bottom', () => {
    const input = [
      "import React from 'react';",
      "import { useState } from 'react';",
      '',
      'const Counter = () => {',
      '  const [n, setN] = useState(0);',
      '  return <button onClick={() => setN(n + 1)}>{n}</button>;',
      '};',
      '',
      'export { Counter };',
      'export default Counter;',
    ].join('\n');
    const out = stripModuleSyntax(input);
    expect(out).not.toMatch(/\bimport\b/);
    expect(out).not.toMatch(/\bexport\b/);
    expect(out).toContain('const Counter = ()');
  });
});
