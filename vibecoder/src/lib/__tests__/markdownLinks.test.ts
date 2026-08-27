import { describe, it, expect } from 'vitest';
import { classifyMarkdownLink, resolveRelativePath, slugifyHeading } from '../markdownLinks';

const README = '/work/repo/README.md';

describe('classifyMarkdownLink', () => {
    it('resolves a relative link against the document directory', () => {
        expect(classifyMarkdownLink('docs/README.md', README)).toEqual({
            kind: 'file',
            path: '/work/repo/docs/README.md',
            fragment: null,
        });
    });

    it('keeps the fragment of a cross-file link', () => {
        expect(classifyMarkdownLink('docs/features.md#status', README)).toEqual({
            kind: 'file',
            path: '/work/repo/docs/features.md',
            fragment: 'status',
        });
    });

    it('climbs out of the directory with ..', () => {
        const link = classifyMarkdownLink('../AGENTS.md', '/work/repo/docs/guide.md');
        expect(link).toEqual({ kind: 'file', path: '/work/repo/AGENTS.md', fragment: null });
    });

    it('treats a file whose name ends in a TLD-looking suffix as a file', () => {
        // The bug this guards: `CHANGELOG.md` read as a host in the `.md`
        // domain, so the click left the app instead of opening the file.
        expect(classifyMarkdownLink('CHANGELOG.md', README)).toEqual({
            kind: 'file',
            path: '/work/repo/CHANGELOG.md',
            fragment: null,
        });
    });

    it('upgrades a bare web domain to https', () => {
        expect(classifyMarkdownLink('github.com/anthropics', README)).toEqual({
            kind: 'external',
            url: 'https://github.com/anthropics',
        });
        // The TLD is the last label of the host, not the first: `docs.example.com`
        // is a host, while `docs.rs` is not on the list and stays a file.
        expect(classifyMarkdownLink('docs.example.com/serde', README)).toEqual({
            kind: 'external',
            url: 'https://docs.example.com/serde',
        });
        expect(classifyMarkdownLink('docs.rs/serde/latest', README)?.kind).toBe('file');
    });

    it('passes through real schemes', () => {
        expect(classifyMarkdownLink('https://example.com/x')).toEqual({
            kind: 'external',
            url: 'https://example.com/x',
        });
        expect(classifyMarkdownLink('mailto:a@b.com')).toEqual({
            kind: 'external',
            url: 'mailto:a@b.com',
        });
        expect(classifyMarkdownLink('//cdn.example.com/x')).toEqual({
            kind: 'external',
            url: 'https://cdn.example.com/x',
        });
    });

    it('reads a file: URL as a path, not as a page for the browser', () => {
        expect(classifyMarkdownLink('file:///work/repo/AGENTS.md', README)).toEqual({
            kind: 'file',
            path: '/work/repo/AGENTS.md',
            fragment: null,
        });
    });

    it('keeps an in-document anchor inside the document', () => {
        expect(classifyMarkdownLink('#answer-style', README)).toEqual({
            kind: 'anchor',
            fragment: 'answer-style',
        });
    });

    it('has no destination for an empty href or a bare hash', () => {
        expect(classifyMarkdownLink('', README)).toBeNull();
        expect(classifyMarkdownLink('#', README)).toBeNull();
    });

    it('decodes a percent-encoded path', () => {
        expect(classifyMarkdownLink('docs/my%20notes.md', README)).toEqual({
            kind: 'file',
            path: '/work/repo/docs/my notes.md',
            fragment: null,
        });
    });

    it('leaves a relative link relative when the document location is unknown', () => {
        // Guessing a workspace root here would answer a question nobody asked.
        expect(classifyMarkdownLink('docs/README.md')).toEqual({
            kind: 'file',
            path: 'docs/README.md',
            fragment: null,
        });
    });
});

describe('resolveRelativePath', () => {
    it('takes an absolute target as given', () => {
        expect(resolveRelativePath(README, '/etc/hosts')).toBe('/etc/hosts');
    });

    it('cannot climb above the root', () => {
        expect(resolveRelativePath('/a/b.md', '../../../x.md')).toBe('/x.md');
    });

    it('resolves against a Windows path', () => {
        expect(resolveRelativePath('C:\\work\\repo\\README.md', 'docs\\a.md')).toBe(
            'C:/work/repo/docs/a.md'
        );
        expect(resolveRelativePath('C:/work/repo/README.md', 'D:/other/a.md')).toBe('D:/other/a.md');
    });

    it('drops . and empty segments', () => {
        expect(resolveRelativePath('/a/b/c.md', './d//e.md')).toBe('/a/b/d/e.md');
    });
});

describe('slugifyHeading', () => {
    it('matches the ids GitHub gives headings', () => {
        expect(slugifyHeading('Answer Style — dense, compact, caveman')).toBe(
            'answer-style--dense-compact-caveman'
        );
        expect(slugifyHeading('Zero-Config First')).toBe('zero-config-first');
        expect(slugifyHeading('  Trailing space  ')).toBe('trailing-space');
    });
});
