/**
 * The browser panel previews pages in an `<iframe>`, which constrains what it
 * can show:
 *
 *  * The Tauri CSP must include `frame-src`. Without it `frame-src` falls back
 *    to `default-src 'self'` and **every** cross-origin frame is blocked —
 *    including the `localhost:3000` dev servers the panel exists for. That was
 *    the state until now: the panel rendered blank for every URL, with the
 *    refusal visible only in a devtools console nobody opens.
 *  * Even with `frame-src` allowed, sites sending `X-Frame-Options: deny` or a
 *    `frame-ancestors` CSP still refuse, and the parent frame cannot detect that
 *    cross-origin. So the panel says so for remote URLs instead of guessing.
 */

import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isLocalPreview } from "../BrowserPanel";

describe("isLocalPreview", () => {
  it("recognises local dev servers, which embed happily", () => {
    for (const url of [
      "http://localhost:3000",
      "http://localhost:5173/some/path",
      "http://127.0.0.1:8080",
      "http://0.0.0.0:4000",
      "http://[::1]:3000",
      "http://app.localhost:3000",
      "https://localhost:8443",
    ]) {
      expect(isLocalPreview(url), url).toBe(true);
    }
  });

  it("treats remote sites as possibly-refusing", () => {
    for (const url of [
      "https://www.msn.com",
      "https://github.com",
      "https://example.com/localhost", // a path, not the host
      "https://localhost.evil.com", // suffix attack on a naive check
    ]) {
      expect(isLocalPreview(url), url).toBe(false);
    }
  });

  it("does not throw on an unparseable URL", () => {
    for (const url of ["", "not a url", "localhost:3000"]) {
      expect(() => isLocalPreview(url)).not.toThrow();
      expect(isLocalPreview(url)).toBe(false);
    }
  });
});

describe("Tauri CSP", () => {
  const csp = (): string => {
    const config = JSON.parse(
      readFileSync(
        join(__dirname, "..", "..", "..", "src-tauri", "tauri.conf.json"),
        "utf8",
      ),
    ) as { app?: { security?: { csp?: string } } };
    return config.app?.security?.csp ?? "";
  };

  it("declares frame-src, or the browser panel cannot load any page", () => {
    // `frame-src` falls back to `child-src` then `default-src`. With
    // `default-src 'self'` and neither declared, every iframe is blocked.
    const policy = csp();
    expect(policy).toContain("default-src 'self'");
    expect(
      policy,
      "without an explicit frame-src, default-src 'self' blocks every preview",
    ).toContain("frame-src");
  });

  it("allows the http(s) origins a preview needs", () => {
    const frameSrc = csp()
      .split(";")
      .map((directive) => directive.trim())
      .find((directive) => directive.startsWith("frame-src"));
    expect(frameSrc).toBeDefined();
    // Local dev servers are plain http; hosted previews are https.
    expect(frameSrc).toContain("http:");
    expect(frameSrc).toContain("https:");
  });

  it("still refuses remote script and default sources", () => {
    // Widening frame-src must not have widened the rest of the policy.
    const policy = csp();
    expect(policy).toContain("script-src 'self'");
    expect(policy).toMatch(/default-src 'self'/);
  });
});
