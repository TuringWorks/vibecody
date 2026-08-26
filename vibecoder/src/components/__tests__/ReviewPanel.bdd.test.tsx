/**
 * BDD: the code reviewer renders whatever the model actually said.
 *
 * `run_code_review` hands back the model's JSON verbatim — the Rust side parses
 * it as `serde_json::Value` and injects two refs. The TypeScript interface it is
 * cast to is a compile-time story about that reply, and the model is under no
 * obligation to match it: the prompt asks for `critical|warning|info` and gets
 * "high", "blocker", "nit", or nothing at all.
 *
 * That gap crashed the app. `SEVERITY_STYLES[issue.severity].border` threw on an
 * unlisted word, out of a render, inside the Source Control sidebar — which had
 * no error boundary — so a review with the wrong word for "high" replaced the
 * whole window with the crash screen. These scenarios are that bug and its
 * neighbours: every field of the reply read through one parser at the edge.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { ReviewPanel, ReviewControls, useCodeReview } from "../ReviewPanel";

beforeEach(() => mockInvoke.mockReset());

/**
 * The two halves wired together, as GitPanel wires them.
 *
 * The control and the body now live on different tabs and share state through
 * `useCodeReview`, so a test that drove the panel's own toolbar no longer has
 * one. Rendering both against a single hook instance is closer to production
 * than before: it exercises the join as well as the body.
 */
function ReviewHost({ workspacePath = "/ws" }: { workspacePath?: string }) {
  const review = useCodeReview(workspacePath);
  return (
    <>
      <ReviewControls review={review} workspacePath={workspacePath} />
      <ReviewPanel review={review} />
    </>
  );
}

/** Run a review whose reply is `report`, and wait for it to land. */
async function review(report: unknown) {
  mockInvoke.mockImplementation((cmd: string) =>
    cmd === "run_code_review" ? Promise.resolve(report) : Promise.resolve(null),
  );
  render(<ReviewHost />);
  fireEvent.click(screen.getByRole("button", { name: /Run Review/ }));
  await waitFor(() => expect(screen.getByText(/Quality Score/)).toBeTruthy());
}

const issue = (over: Record<string, unknown>) => ({
  file: "src/a.ts",
  line: 3,
  severity: "critical",
  category: "security",
  description: "hardcoded token",
  ...over,
});

describe("Given a severity the schema never offered", () => {
  it("When the review lands, Then the issue renders with the model's own word", async () => {
    await review({ summary: "s", issues: [issue({ severity: "high" })], score: {} });

    // The badge says what the reviewer said. Relabelling "high" as "critical"
    // would be putting a word in its mouth.
    expect(screen.getByText("high")).toBeTruthy();
    expect(screen.getByText("hardcoded token")).toBeTruthy();
  });

  it("When the word is one nobody recognises, Then it still renders", async () => {
    await review({ summary: "s", issues: [issue({ severity: "banana" })], score: {} });

    expect(screen.getByText("banana")).toBeTruthy();
  });

  it("When there is no severity at all, Then the badge says so", async () => {
    await review({ summary: "s", issues: [issue({ severity: undefined })], score: {} });

    // Not blank, and not promoted to a bucket it was never put in.
    expect(screen.getByText("unlabelled")).toBeTruthy();
  });

  it("When it is a synonym, Then it is filtered under the bucket it means", async () => {
    await review({
      summary: "s",
      issues: [issue({ severity: "blocker" }), issue({ severity: "nit", description: "spacing" })],
      score: {},
    });

    // "blocker" filed under Critical, "nit" under Info — otherwise a review's
    // worst finding hides behind a filter tab that reads (0).
    expect(screen.getByRole("button", { name: "Critical (1)" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Info (1)" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Critical (1)" }));
    expect(screen.getByText("hardcoded token")).toBeTruthy();
    expect(screen.queryByText("spacing")).toBeNull();
  });
});

describe("Given a reply missing the fields the panel reads", () => {
  it("When issues are absent, Then the panel renders the rest", async () => {
    await review({ summary: "nothing to report", score: { overall: 9 } });

    expect(screen.getByText("nothing to report")).toBeTruthy();
    expect(screen.getByText(/Overall: 9.0 \/ 10/)).toBeTruthy();
  });

  it("When a score is absent, Then it reads as not scored rather than zero", async () => {
    await review({ summary: "s", issues: [], score: { overall: 8, correctness: 7 } });

    // A missing security score shown as 0.0/10 is an accusation nobody made,
    // and as 10 it is a clean bill of health nobody gave.
    expect(screen.getAllByText("not scored").length).toBe(3);
    expect(screen.queryByText("0.0")).toBeNull();
  });

  it("When the whole score object is absent, Then the header drops the overall", async () => {
    await review({ summary: "s", issues: [] });

    expect(screen.getByText("Quality Score")).toBeTruthy();
    expect(screen.queryByText(/Overall:/)).toBeNull();
  });

  it("When the reply is an empty object, Then the panel still renders", async () => {
    // The floor: a model that answered `{}` must not take the window down.
    await review({});

    expect(screen.getByText(/Quality Score/)).toBeTruthy();
  });
});
