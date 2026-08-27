/**
 * What the spoken path is told about the project.
 *
 * Voice gets one round trip and no tools: whatever this block omits, the
 * assistant cannot go and find. Asked to summarise the project it answered
 * "just a collection of directories and files — I couldn't tell what Gbrain
 * is", because the block was a bare path listing and the answer was in a
 * README it had no way to open.
 *
 * These pin the two properties that matter: the block *names* the project and
 * carries the README, and every part of it is bounded — this is prepended to a
 * system prompt on a latency-sensitive path, so an unbounded file would push
 * the question itself out of the window.
 */
import { describe, expect, it } from "vitest";
import {
  buildVoiceContext,
  findReadme,
  VOICE_CONTEXT_LIMITS,
} from "@vibe/shared/voice/voiceContext";

describe("voice context", () => {
  it("names the project, not just its files", () => {
    const ctx = buildVoiceContext({
      root: "/Users/me/src/gbrain",
      tree: ["src/main.rs", "README.md"],
    });
    expect(ctx).toContain("Project: gbrain");
    expect(ctx).toContain("/Users/me/src/gbrain");
  });

  it("carries the README, which is where the answer usually is", () => {
    const ctx = buildVoiceContext({ readme: "# GBrain\n\nA second brain." });
    expect(ctx).toContain("README:");
    expect(ctx).toContain("A second brain.");
  });

  it("sends the open file's contents, not only its path", () => {
    const withText = buildVoiceContext({ openFile: "src/main.rs", openFileText: "fn main() {}" });
    expect(withText).toContain("src/main.rs");
    expect(withText).toContain("fn main() {}");
    // A path with no contents is still worth saying — it is what is on screen.
    expect(buildVoiceContext({ openFile: "src/main.rs" })).toContain("Open file: src/main.rs");
  });

  it("bounds every part, and says when it truncated", () => {
    const ctx = buildVoiceContext({
      readme: "x".repeat(50_000),
      openFileText: "y".repeat(50_000),
      openFile: "big.rs",
      tree: Array.from({ length: 5_000 }, (_, i) => `f${i}.ts`),
    });
    expect(ctx).toContain("(truncated)");
    expect(ctx.length).toBeLessThanOrEqual(VOICE_CONTEXT_LIMITS.total + 32);
    expect(ctx).toContain(`Project files (${VOICE_CONTEXT_LIMITS.treeEntries} of 5000)`);
  });

  it("returns nothing when there is nothing to say", () => {
    // The daemon treats an empty block as "clear it" — a header with no body
    // under it would pin an empty workspace instead.
    expect(buildVoiceContext({})).toBe("");
    expect(buildVoiceContext({ tree: [], readme: "  " })).toBe("");
  });

  it("picks the root README over a deeper one", () => {
    expect(findReadme(["docs/readme.md", "README.md", "vendor/x/README.md"])).toBe("README.md");
    expect(findReadme(["src/main.rs"])).toBeUndefined();
    // Case matters on Linux and not on macOS; both spellings are the same file.
    expect(findReadme(["ReadMe.markdown"])).toBe("ReadMe.markdown");
  });
});
