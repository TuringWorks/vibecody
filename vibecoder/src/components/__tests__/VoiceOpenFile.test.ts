/**
 * Source-scan regression test — the spoken "open that file" reaches the editor.
 *
 * Asked to open a file, the assistant read it and described it. Every half
 * worked: the daemon ran `read_file`, the hook delivered the reply, the chat
 * log showed the answer. Nothing opened, because there was no path from a
 * spoken turn to `openFile` at all — `onOpenFile` was threaded to nine panels
 * and stopped at `PanelHost` for the one panel that hosts the conversation.
 *
 * That is a prop-threading gap, and a prop-threading gap is invisible to
 * `tsc`: every link is an optional prop, so omitting one is legal at every
 * step and the feature is simply inert. This walks the chain instead —
 * LazyPanels → ChatComposite → ChatTabManager → AIChat → useVoiceDuplex — and
 * fails on the first link that drops it.
 *
 * The declaration side is checked too. `useVoiceDuplex` tells the daemon the
 * client can open files *because* a handler was supplied, so the daemon only
 * teaches the assistant a tool someone can honour. If those two ever come
 * apart, the assistant says "I've opened that for you" over an editor that
 * did not move — the failure this whole path exists to prevent.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = resolve(__dirname, "..", "..");
const SHARED = resolve(SRC, "..", "..", "packages", "vibe-ui-shared", "src");

const read = (path: string) => readFileSync(path, "utf8");

/** Strip comments: this file's own explanations quote the symbols it scans. */
const code = (text: string) =>
  text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");

const link = (label: string, path: string) => ({ label, source: code(read(path)) });

const CHAIN = [
  link("LazyPanels", resolve(SRC, "components", "LazyPanels.tsx")),
  link("ChatComposite", resolve(SRC, "components", "composite", "ChatComposite.tsx")),
  link("ChatTabManager", resolve(SRC, "components", "ChatTabManager.tsx")),
  link("AIChat", resolve(SRC, "components", "AIChat.tsx")),
];

describe("a spoken open reaches the editor", () => {
  it.each(CHAIN)("$label passes onOpenFile on", ({ label, source }) => {
    expect(
      source.includes("onOpenFile"),
      `${label} drops onOpenFile, so a spoken "open that file" ends there`,
    ).toBe(true);
  });

  it("AIChat hands it to the voice hook, not just to its own props", () => {
    const aiChat = CHAIN[CHAIN.length - 1].source;
    const mount = aiChat.slice(aiChat.indexOf("useVoiceDuplex({"));
    expect(mount.slice(0, mount.indexOf("});"))).toContain("onOpenFile");
  });

  it("the hook declares the capability only when it was given a handler", () => {
    const hook = code(read(resolve(SHARED, "voice", "useVoiceDuplex.ts")));
    expect(hook).toContain("set_capabilities");
    // The guard is the point: an unconditional declaration would have the
    // daemon offer `open_file` to VibeDesk and VibeAIChat, which have no
    // editor to show one in.
    expect(hook).toMatch(/if \(onOpenFile\.current\)/);
  });

  it("the hook acts on the daemon's open_file action", () => {
    const hook = code(read(resolve(SHARED, "voice", "useVoiceDuplex.ts")));
    expect(hook).toContain('m.action === "open_file"');
    expect(hook).toMatch(/onOpenFile\.current\?\.\(m\.path\)/);
  });
});
