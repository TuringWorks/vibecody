import { describe, it, expect } from "vitest";
import { sanitizeEpubHtml } from "../DocumentViewer";

/**
 * EPUB chapter HTML is T5 (attacker-controlled) per docs/security/threat-model.md
 * — the user opened a file off disk that originated elsewhere on the internet.
 * sanitizeEpubHtml() is the only thing between that markup and the DOM, and it
 * had no test coverage at all, which is how a config bug survived: setting
 * `USE_PROFILES` made DOMPurify ignore ALLOWED_TAGS/ALLOWED_ATTR entirely, so
 * the curated allow-list was inert and a much broader default set was in force.
 *
 * These tests pin the behaviour the config intends, so that regression is
 * caught rather than reasoned about.
 */

const hasExecutableMarkup = (html: string) =>
  /<script|<iframe|<object|<embed|<base|<meta|on\w+\s*=|javascript:|srcset|<form|<input|<style/i.test(html);

describe("sanitizeEpubHtml — strips executable markup", () => {
  it.each([
    ["script tag", "<p>hi</p><script>alert(1)</script>"],
    ["iframe", '<iframe src="//evil"></iframe>'],
    ["object", '<object data="x"></object>'],
    ["embed", '<embed src="x">'],
    ["base tag", '<base href="//evil">'],
    ["meta refresh", '<meta http-equiv="refresh" content="0;url=//evil">'],
    ["inline handler", '<p onclick="alert(1)">x</p>'],
    ["unquoted handler", "<p onmouseover=alert(1)>x</p>"],
    ["javascript: href", '<a href="javascript:alert(1)">x</a>'],
    ["style attribute", '<p style="background:url(javascript:alert(1))">x</p>'],
    ["style tag", '<style>@import "//evil";</style>'],
    ["form and input", '<form action="//evil"><input name="a"></form>'],
    ["srcset", '<img src="a.png" srcset="//evil 1x">'],
    ["svg onload", '<svg onload="alert(1)"></svg>'],
    ["mXSS via noscript", '<noscript><p title="</noscript><img src=x onerror=alert(1)>">'],
    ["mXSS via style/annotation", '<svg></p><style><a id="</style><img src=x onerror=alert(1)>">'],
  ])("removes %s", (_name, html) => {
    expect(hasExecutableMarkup(sanitizeEpubHtml(html))).toBe(false);
  });
});

describe("sanitizeEpubHtml — enforces the allow-list", () => {
  // Each of these rendered while USE_PROFILES was set, despite not appearing
  // in ALLOWED_TAGS. The tag must not survive; DOMPurify keeps inner text by
  // default (KEEP_CONTENT), which is fine — it is inert.
  it.each([
    ["video", '<video src="x" controls>v</video>'],
    ["audio", '<audio src="x">a</audio>'],
    ["canvas", "<canvas>c</canvas>"],
    ["marquee", "<marquee>m</marquee>"],
    ["progress", '<progress value="1">p</progress>'],
    ["dialog", "<dialog open>d</dialog>"],
    ["slot", "<slot>s</slot>"],
  ])("drops <%s>, which is not in ALLOWED_TAGS", (tag, html) => {
    expect(sanitizeEpubHtml(html)).not.toContain(`<${tag}`);
  });

  it("drops attributes outside ALLOWED_ATTR", () => {
    const out = sanitizeEpubHtml('<p tabindex="3" contenteditable="true">x</p>');
    expect(out).not.toContain("tabindex");
    expect(out).not.toContain("contenteditable");
  });
});

describe("sanitizeEpubHtml — preserves legitimate chapter content", () => {
  it("keeps text-level structure", () => {
    const out = sanitizeEpubHtml("<h1>Chapter</h1><p>hello <em>world</em></p>");
    expect(out).toContain("<h1>Chapter</h1>");
    expect(out).toContain("<em>world</em>");
  });

  it("keeps images with an allowed src and alt", () => {
    const out = sanitizeEpubHtml('<img src="images/ch1.png" alt="figure 1">');
    expect(out).toContain('src="images/ch1.png"');
    expect(out).toContain('alt="figure 1"');
  });

  it("keeps tables", () => {
    const out = sanitizeEpubHtml("<table><tbody><tr><td>cell</td></tr></tbody></table>");
    expect(out).toContain("<td>cell</td>");
  });

  // Regression: every SVG tag in ALLOWED_TAGS was silently dropped while
  // USE_PROFILES was set, because svg lives in a separate DOMPurify profile.
  it("keeps inline SVG ornaments, which ALLOWED_TAGS permits", () => {
    const out = sanitizeEpubHtml(
      '<svg viewBox="0 0 10 10"><path d="M0 0 L10 10" fill="none"/><circle cx="5" cy="5" r="2"/></svg>',
    );
    expect(out).toContain("<svg");
    expect(out).toContain("<path");
    expect(out).toContain("<circle");
    expect(out).toContain('viewBox="0 0 10 10"');
  });
});

/**
 * `href` and `xlink:href` were added to ALLOWED_ATTR on 2026-08-24, because
 * without them every link in every book rendered dead and SVG-wrapped covers
 * rendered blank. That widens the allow-list, so what the widening does *not*
 * permit is pinned here.
 */
describe("sanitizeEpubHtml — links, now that href is allowed", () => {
  it("keeps a relative link, which is how a book navigates itself", () => {
    const out = sanitizeEpubHtml('<a href="ch2.xhtml#later">forward</a>');
    expect(out).toContain('href="ch2.xhtml#later"');
  });

  it("keeps an http link", () => {
    expect(sanitizeEpubHtml('<a href="https://example.com">out</a>')).toContain(
      'href="https://example.com"',
    );
  });

  it.each([
    ["javascript:", '<a href="javascript:alert(1)">x</a>'],
    ["vbscript:", '<a href="vbscript:msgbox(1)">x</a>'],
    ["mixed-case javascript:", '<a href="JaVaScRiPt:alert(1)">x</a>'],
    ["whitespace-obfuscated javascript:", '<a href="java\tscript:alert(1)">x</a>'],
  ])("drops a %s href while keeping the text", (_name, html) => {
    const out = sanitizeEpubHtml(html);
    expect(out.toLowerCase()).not.toContain("script:");
    expect(out).not.toContain("href=");
    expect(out).toContain("x");
  });

  it("keeps xlink:href on an SVG image, which is how covers are wrapped", () => {
    const out = sanitizeEpubHtml(
      '<svg viewBox="0 0 10 10"><image xlink:href="cover.jpg" width="10" height="10"/></svg>',
    );
    expect(out).toContain("cover.jpg");
  });

  it("drops a script URL in xlink:href", () => {
    const out = sanitizeEpubHtml('<svg><image xlink:href="javascript:alert(1)"/></svg>');
    expect(out.toLowerCase()).not.toContain("javascript:");
  });

  it("still drops a data: URL that would carry markup", () => {
    // `data:` is permitted on media elements, where it cannot execute — never
    // as a navigable link target.
    const out = sanitizeEpubHtml('<a href="data:text/html,<script>alert(1)</script>">x</a>');
    expect(hasExecutableMarkup(out)).toBe(false);
  });
});

