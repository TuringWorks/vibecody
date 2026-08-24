/**
 * A book's own stylesheet is what makes it look like a book, so it is applied
 * rather than dropped — which makes this module a security surface. EPUB
 * content is T5 (attacker-controlled): the file came from somewhere else.
 *
 * These pin both halves: the typography survives, and every construct that lets
 * CSS or markup reach outside the chapter container does not.
 */
import { describe, it, expect } from 'vitest';

import {
  resolveAgainst,
  rewriteChapterHtml,
  scopeEpubCss,
  scopeSelectors,
  splitHref,
} from '../epubRender';

const SCOPE = '.epub-chapter-body';

/** Resolves exactly one asset, so "unknown reference" is easy to express. */
const resolve = (reference: string) =>
  reference === 'OEBPS/images/fig1.png' || reference === '../images/fig1.png'
    ? 'blob:vibecoder/fig1'
    : undefined;

describe('scopeEpubCss — the book keeps its typography', () => {
  it('scopes ordinary rules to the chapter container', () => {
    const css = scopeEpubCss('p { text-indent: 1.2em } h1, h2 { font-weight: 700 }', SCOPE, resolve);
    expect(css).toContain('.epub-chapter-body p');
    expect(css).toContain('text-indent: 1.2em');
    expect(css).toContain('.epub-chapter-body h1, .epub-chapter-body h2');
  });

  it('treats the book\'s body as the container itself', () => {
    const css = scopeEpubCss('body { line-height: 1.6 } html { color: #222 }', SCOPE, resolve);
    // Not `.epub-chapter-body body` — the chapter has no <body> element of its
    // own, so that rule would match nothing and the book would lose its leading.
    expect(css).toContain('.epub-chapter-body { line-height: 1.6 }');
    expect(css).toContain('.epub-chapter-body { color: #222 }');
  });

  it('keeps media queries and scopes what is inside them', () => {
    const css = scopeEpubCss('@media (max-width: 40em) { p { font-size: 90% } }', SCOPE, resolve);
    expect(css).toContain('@media (max-width: 40em)');
    expect(css).toContain('.epub-chapter-body p');
  });

  it('keeps @font-face and keyframes usable', () => {
    const css = scopeEpubCss(
      '@font-face { font-family: Bookish; src: url("OEBPS/images/fig1.png") }' +
        '@keyframes fade { 0% { opacity: 0 } 100% { opacity: 1 } }',
      SCOPE,
      resolve,
    );
    expect(css).toContain('@font-face');
    expect(css).toContain('blob:vibecoder/fig1');
    // Keyframe selectors are percentages: scoping them would match nothing.
    expect(css).toContain('0% { opacity: 0 }');
    expect(css).not.toContain('.epub-chapter-body 0%');
  });

  it('rewrites url() targets to the resources that came with the chapter', () => {
    const css = scopeEpubCss('div { background: url(../images/fig1.png) }', SCOPE, resolve);
    expect(css).toContain('url("blob:vibecoder/fig1")');
  });
});

describe('scopeEpubCss — what must not survive', () => {
  it('drops @import, which would fetch a stylesheet from anywhere', () => {
    const css = scopeEpubCss('@import url("https://evil.example/x.css"); p { color: red }', SCOPE, resolve);
    expect(css).not.toContain('@import');
    expect(css).not.toContain('evil.example');
    expect(css).toContain('.epub-chapter-body p');
  });

  it('drops a background whose url the book chose but no resource matched', () => {
    // Left alone it would resolve against the application's own origin.
    const css = scopeEpubCss('div { background: url(/absent.png) }', SCOPE, resolve);
    expect(css).not.toContain('absent.png');
  });

  it('drops fixed and sticky positioning, which escape the container', () => {
    const css = scopeEpubCss(
      'div { position: fixed; top: 0; left: 0 } span { position: sticky } p { position: relative }',
      SCOPE,
      resolve,
    );
    expect(css).not.toContain('fixed');
    expect(css).not.toContain('sticky');
    expect(css).toContain('position: relative');
  });

  it('drops expression() and script URLs', () => {
    const css = scopeEpubCss(
      'p { width: expression(alert(1)); background: url(javascript:alert(1)) }',
      SCOPE,
      resolve,
    );
    expect(css.toLowerCase()).not.toContain('expression(');
    expect(css.toLowerCase()).not.toContain('javascript:');
  });

  it('drops @page and unknown at-rules rather than passing them through', () => {
    const css = scopeEpubCss('@page { margin: 0 } @charset "utf-8"; p { color: red }', SCOPE, resolve);
    expect(css).not.toContain('@page');
    expect(css).not.toContain('@charset');
    expect(css).toContain('.epub-chapter-body p');
  });

  it('survives a stylesheet that is truncated mid-rule', () => {
    expect(() => scopeEpubCss('p { color: red } div { background:', SCOPE, resolve)).not.toThrow();
  });
});

describe('scopeSelectors', () => {
  it('prefixes each selector in a list', () => {
    expect(scopeSelectors('p, blockquote > cite', '.c')).toBe('.c p, .c blockquote > cite');
  });

  it('handles a combinator that follows body', () => {
    expect(scopeSelectors('body > p', '.c')).toBe('.c > p');
  });
});

describe('rewriteChapterHtml', () => {
  it('points images at the resources that came with the chapter', () => {
    const { html } = rewriteChapterHtml('<p><img src="../images/fig1.png" alt="Figure 1"></p>', resolve);
    expect(html).toContain('src="blob:vibecoder/fig1"');
    expect(html).toContain('alt="Figure 1"');
  });

  it('strips a remote image instead of fetching it', () => {
    // An offline book asking for a remote image is a tracking pixel far more
    // often than a picture.
    const { html, unresolved } = rewriteChapterHtml(
      '<img src="https://tracker.example/pixel.gif">',
      resolve,
    );
    expect(html).not.toContain('tracker.example');
    expect(unresolved).toContain('https://tracker.example/pixel.gif');
  });

  it('leaves an inline data: image alone', () => {
    const source = '<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=">';
    expect(rewriteChapterHtml(source, resolve).html).toContain('data:image/gif');
  });

  it('marks internal links for the viewer and neutralises their href', () => {
    const { html } = rewriteChapterHtml('<a href="ch2.xhtml#later">forward</a>', resolve);
    expect(html).toContain('data-epub-link="ch2.xhtml#later"');
    // A live relative href would navigate the whole webview away from the editor.
    expect(html).toContain('href="#"');
  });

  it('marks external links so the viewer can hand them to the browser', () => {
    const { html } = rewriteChapterHtml('<a href="https://example.com">out</a>', resolve);
    expect(html).toContain('data-epub-external="https://example.com"');
  });

  it('records a reference the book made that the book does not contain', () => {
    const { html, unresolved } = rewriteChapterHtml('<img src="../images/gone.png">', resolve);
    expect(html).not.toContain('gone.png');
    expect(unresolved).toEqual(['../images/gone.png']);
  });
});

describe('resolveAgainst', () => {
  it('resolves the way a browser would, from the chapter\'s own directory', () => {
    expect(resolveAgainst('OEBPS/text/ch1.xhtml', '../images/a.png')).toBe('OEBPS/images/a.png');
    expect(resolveAgainst('OEBPS/text/ch1.xhtml', 'ch2.xhtml')).toBe('OEBPS/text/ch2.xhtml');
    expect(resolveAgainst('OEBPS/text/ch1.xhtml', '/top.xhtml')).toBe('top.xhtml');
    expect(resolveAgainst('a/b.xhtml', 'my%20image.png')).toBe('a/my image.png');
  });

  it('matches the backend resolver, which is what fills the resource map', () => {
    // Both sides resolve; a mismatch would show as every image being missing.
    expect(resolveAgainst('OEBPS/text/ch1.xhtml', './ch2.xhtml#x')).toBe('OEBPS/text/ch2.xhtml');
  });
});

describe('splitHref', () => {
  it('separates the path from the anchor', () => {
    expect(splitHref('ch2.xhtml#later')).toEqual({ path: 'ch2.xhtml', fragment: 'later' });
    expect(splitHref('#top')).toEqual({ path: '', fragment: 'top' });
    expect(splitHref('ch2.xhtml')).toEqual({ path: 'ch2.xhtml', fragment: null });
  });
});
