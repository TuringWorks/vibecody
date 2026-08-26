import { describe, it, expect } from "vitest";
import { gitStatusCode, gitStatusLabel } from "../gitStatus";

describe("gitStatusCode", () => {
  it("maps every FileStatus variant the backend can send", () => {
    // Mirrors vibe_core::git::FileStatus. If a variant is added there and not
    // here, this is where it should fail.
    expect(gitStatusCode("Modified")).toBe("M");
    expect(gitStatusCode("New")).toBe("N");
    expect(gitStatusCode("Deleted")).toBe("D");
    expect(gitStatusCode("Renamed")).toBe("R");
    expect(gitStatusCode("Ignored")).toBe("I");
    expect(gitStatusCode("Conflicted")).toBe("C");
    expect(gitStatusCode("Unknown")).toBe("?");
  });

  it("gives every known status a distinct letter", () => {
    const known = ["Modified", "New", "Deleted", "Renamed", "Ignored", "Conflicted"];
    const codes = known.map(gitStatusCode);
    expect(new Set(codes).size).toBe(known.length);
  });

  it("falls through to ? rather than inventing a letter", () => {
    // The failure this replaces: charAt(0) would render "Untracked" as "U",
    // which is also what "Unknown" would give — two states, one letter.
    expect(gitStatusCode("Untracked")).toBe("?");
    expect(gitStatusCode("")).toBe("?");
  });
});

describe("gitStatusLabel", () => {
  it("keeps the full word as the accessible name", () => {
    expect(gitStatusLabel("Modified")).toBe("Modified");
  });

  it("says so when it does not recognise the status", () => {
    expect(gitStatusLabel("Frobnicated")).toContain("Frobnicated");
  });
});
