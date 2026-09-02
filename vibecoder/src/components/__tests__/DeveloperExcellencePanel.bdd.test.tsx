/**
 * Developer Excellence panel — the behaviours that must not regress.
 *
 * The whole point of this subsystem is that it does not report numbers nobody
 * measured. That property lives in three places — the measurement, the API, and
 * the panel — and the panel is the easiest of the three to break silently: drop
 * the `unmeasured` block from the render and the screen goes back to looking
 * like a complete scorecard, with nothing failing.
 *
 * So these tests assert on what reaches the screen for an *absent* metric, on
 * the request the panel sends, and on the caveat that keeps a false negative
 * from reading as a finding. They deliberately do not assert that the four
 * tiles are pretty.
 */
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockDaemonFetch = vi.fn();
vi.mock("../../lib/daemonFetch", () => ({
  daemonFetch: (...args: unknown[]) => mockDaemonFetch(...args),
  getDaemonToken: async () => "test-token",
}));

import { DeveloperExcellencePanel } from "../DeveloperExcellencePanel";

const json = (body: unknown, status = 200): Response =>
  ({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body),
  }) as unknown as Response;

/** A scorecard where deployments were found but nothing was ever reverted. */
const SCORECARD = {
  headline: "Delivery B on 3/4 DORA keys; practice maturity 2.1/3 detected across 10 practices.",
  dora_coverage: 0.75,
  delivery_grade: "B",
  dora: {
    repo: "/src/demo",
    window_days: 90,
    since: 0,
    generated_at: 0,
    release_marker: "version-tags",
    release_marker_description: "version-like git tags",
    band_source: "DORA State of DevOps performance bands",
    deployment_frequency: {
      value: 2.5,
      unit: "deployments/week",
      band: "high",
      sample_size: 32,
      proxy: "version-like git tags",
    },
    lead_time_for_changes: {
      value: 18.4,
      unit: "hours (p50)",
      band: "elite",
      sample_size: 210,
      proxy: "commit author-time → time of the release that first contained it",
      percentiles: [
        { label: "p50", value: 18.4 },
        { label: "p75", value: 60.2 },
      ],
    },
    change_failure_rate: {
      value: 0,
      unit: "percent of deployments",
      band: "elite",
      sample_size: 32,
      proxy: "a deployment followed by a revert/hotfix/rollback commit before the next one",
    },
    // time_to_restore is absent on purpose — a clean window.
    unmeasured: [
      {
        metric: "time_to_restore",
        reason:
          "32 deployment(s) observed and none was followed by a revert, hotfix or rollback commit — there is no restoration to time. A clean window is not a restore time of zero.",
        to_measure_this: "record incident start and resolution in the incident tool and feed it in",
      },
    ],
    deployments: [
      { name: "v1.4.0", commit: "abc", at: 1_700_000_000, followed_by_remediation: false },
    ],
    commits_in_window: 210,
    authors_in_window: 12,
    notes: [],
  },
  practices: {
    workspace: "/src/demo",
    generated_at: 0,
    mean_level: 2.1,
    max_detectable_level: 3,
    scope_note:
      "Levels are DETECTED from files present, capped at 3 ('defined'). Level 4 ('optimizing') is attested by people, never by a scan.",
    practices: [
      {
        key: "automated-testing",
        title: "Automated testing",
        pillar: "Global Practices Program",
        found: 1,
        expected: 3,
        level: 1,
        level_name: "initial",
        next_step: "Publish the one command that runs the whole suite.",
        detection_caveat:
          "Detected by path only. Languages that colocate tests with source have no test directory to find.",
        signals: [
          { name: "test directory", found: false },
          { name: "coverage configuration", found: false },
          { name: "test command documented", found: true, path: "Makefile" },
        ],
      },
    ],
  },
};

const ONBOARDING = {
  repo: "/src/demo",
  window_days: 90,
  generated_at: 0,
  readiness: [
    { name: "one-command bootstrap", found: true, path: "Makefile" },
    { name: "reproducible environment", found: false },
    { name: "getting-started guide", found: true, path: "README.md" },
  ],
  readiness_found: 2,
  readiness_expected: 3,
  new_contributors: [],
  not_measured: [
    {
      metric: "time_to_first_commit",
      reason:
        "git records a contributor's first commit but not the day they joined, so the interval the target is about has no start.",
      to_measure_this: "join the HR/IdP start date to the first-commit date",
    },
  ],
  notes: ["2/3 bootstrap signals present."],
};

/** A SPACE frame with three dimensions answered and two named as gaps. */
const SPACE = {
  repo: "/src/demo",
  window_days: 90,
  dimensions_measured: 3,
  outcome_signal: true,
  scope_note:
    "SPACE frame over 90 days. 3 of 5 dimensions have a measure from this repository; the rest name the system that holds their data. There is deliberately NO aggregate SPACE score.",
  dimensions: [
    {
      dimension: "satisfaction",
      key: "satisfaction",
      title: "Satisfaction & wellbeing",
      measures: [],
      unmeasured: [
        {
          metric: "tooling_satisfaction",
          reason: "How people feel about the tools they use cannot be derived from what they committed.",
          to_measure_this: "run the quarterly survey, segment by team, never below five respondents",
        },
      ],
    },
    {
      dimension: "performance",
      key: "performance",
      title: "Performance",
      measures: [
        {
          name: "change failure rate",
          value: 12.5,
          unit: "percent of deployments",
          source: "DORA stability (a deployment followed by a revert)",
          sample_size: 8,
          caveat: "Outcome quality is not visible here.",
        },
      ],
      unmeasured: [],
    },
    {
      dimension: "activity",
      key: "activity",
      title: "Activity",
      measures: [
        { name: "commits", value: 210, unit: "commits / 90d", source: "git history", sample_size: 210, caveat: "A volume, not an outcome." },
        { name: "contributing authors", value: 12, unit: "distinct authors", source: "git history", sample_size: 12 },
      ],
      unmeasured: [],
    },
    {
      dimension: "collaboration",
      key: "collaboration",
      title: "Communication & collaboration",
      measures: [
        { name: "files touched by more than one author", value: 34.2, unit: "percent of files touched", source: "git history", sample_size: 900 },
      ],
      unmeasured: [
        {
          metric: "review_latency",
          reason: "the wait from opening a change to its first substantive review lives in the forge, not in git",
          to_measure_this: "pull it from the forge's pull-request API",
        },
      ],
    },
    {
      dimension: "efficiency",
      key: "efficiency",
      title: "Efficiency & flow",
      measures: [],
      unmeasured: [
        {
          metric: "pipeline_wait",
          reason: "queue and execution time are recorded by the CI system",
          to_measure_this: "export job queue and run durations from CI",
        },
      ],
    },
  ],
};

function routeTo(body: Record<string, unknown>) {
  return (url: string) => {
    if (url.includes("/devex/survey.md")) return Promise.resolve(json("# Engineering experience survey"));
    if (url.includes("/devex/scorecard")) return Promise.resolve(json({ scorecard: body.scorecard }));
    if (url.includes("/devex/onboarding")) return Promise.resolve(json({ onboarding: body.onboarding }));
    if (url.includes("/devex/space")) return Promise.resolve(json({ space: body.space }));
    return Promise.resolve(json({ error: "unexpected route" }, 404));
  };
}

beforeEach(() => {
  mockDaemonFetch.mockReset();
  mockDaemonFetch.mockImplementation(
    routeTo({ scorecard: SCORECARD, onboarding: ONBOARDING, space: SPACE })
  );
});

describe("DeveloperExcellencePanel", () => {
  it("renders an unmeasurable key as absent-with-a-reason, never as a value", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);

    await screen.findByText(/Not measured/i);
    // The reason and the remedy both reach the screen: a reader who sees the
    // gap must also see what closes it.
    expect(screen.getByText(/A clean window is not a restore time of zero/i)).toBeTruthy();
    expect(screen.getByText(/record incident start and resolution/i)).toBeTruthy();

    // And the metric never appears as a tile value. If a future change starts
    // rendering absent metrics as 0, this is the assertion that catches it.
    const tiles = screen.queryAllByText("Time to restore");
    const asTile = tiles.filter((el) => el.className?.includes?.("panel-card"));
    expect(asTile.length).toBe(0);
  });

  it("shows a measured zero as a real measurement, distinct from absence", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);

    // Change failure rate genuinely measured 0% over 32 deployments. That is a
    // finding, and it must render as a value — the panel must not collapse
    // "measured zero" and "not measured" into one look.
    const label = await screen.findByText("Change failure rate");
    const tile = label.parentElement;
    expect(tile).toBeTruthy();
    // Read the whole tile rather than a lone text node: the value and its unit
    // are separate children, so an exact-string query would depend on how the
    // markup happens to be split rather than on what the user reads.
    expect(tile?.textContent).toContain("0.00");
    expect(tile?.textContent).toContain("percent of deployments");
    expect(tile?.textContent).toContain("n=32");
    // And it is a tile, not an entry in the "Not measured" list.
    expect(tile?.textContent).not.toContain("Not measured");
  });

  it("carries the proxy and sample size next to every value", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    expect(screen.getByText(/proxy: version-like git tags/)).toBeTruthy();
    expect(screen.getByText(/n=210/)).toBeTruthy();
  });

  it("requires a workspace rather than measuring an unnamed directory", () => {
    render(<DeveloperExcellencePanel workspacePath={null} />);
    expect(screen.getByText(/Open a workspace to measure it/i)).toBeTruthy();
    // Nothing is requested at all — the panel does not ask the daemon to guess.
    expect(mockDaemonFetch).not.toHaveBeenCalled();
  });

  it("sends the selected window and marker on the request", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await waitFor(() => expect(mockDaemonFetch).toHaveBeenCalled());

    const first = String(mockDaemonFetch.mock.calls[0][0]);
    expect(first).toContain("path=%2Fsrc%2Fdemo");
    expect(first).toContain("window=90");
    expect(first).toContain("marker=tags");

    mockDaemonFetch.mockClear();
    fireEvent.change(screen.getByLabelText("Measurement window"), { target: { value: "180" } });
    await waitFor(() => expect(mockDaemonFetch).toHaveBeenCalled());
    expect(String(mockDaemonFetch.mock.calls[0][0])).toContain("window=180");
  });

  it("shows a practice's detection caveat, not just its missing signals", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Practices/));

    await screen.findByText("Automated testing");
    // "missing: test directory" on a repo with thousands of inline tests reads
    // as a finding unless the caveat is on screen beside it.
    expect(
      screen.getByText(/colocate tests with source have no test directory to find/i)
    ).toBeTruthy();
    // And the cap is stated, so a level 3 is never read as "fully mature".
    expect(screen.getByText(/attested by people, never by a scan/i)).toBeTruthy();
  });

  it("says plainly that time-to-first-commit is not derivable from git", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Onboarding/));

    await screen.findByText(/not measured/i);
    expect(screen.getByText(/not the day they joined/i)).toBeTruthy();
  });

  it("clears stale numbers when a re-measurement fails", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");

    // A failure that left the previous window's values on screen would
    // attribute them to a window they did not come from.
    mockDaemonFetch.mockImplementation(() =>
      Promise.resolve(json({ error: "`window` must be 1825 days or fewer" }, 400))
    );
    fireEvent.change(screen.getByLabelText("Measurement window"), { target: { value: "365" } });

    await screen.findByRole("alert");
    expect(screen.getByText(/1825 days or fewer/)).toBeTruthy();
    expect(screen.queryByText("Deployment frequency")).toBeNull();
  });

  it("shows every SPACE dimension, including the ones git cannot answer", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Experience \(SPACE\)/));

    for (const title of [
      "Satisfaction & wellbeing",
      "Performance",
      "Activity",
      "Communication & collaboration",
      "Efficiency & flow",
    ]) {
      expect(await screen.findByText(title)).toBeTruthy();
    }
    // A dimension with no data names the system that holds it, rather than
    // rendering empty or as a zero.
    expect(screen.getByText(/run the quarterly survey/i)).toBeTruthy();
    expect(screen.getByText(/export job queue and run durations from CI/i)).toBeTruthy();
  });

  it("says review latency is absent rather than deriving it from merge times", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Experience \(SPACE\)/));

    await screen.findByText(/Review latency — not measured here/i);
    expect(screen.getByText(/lives in the forge, not in git/i)).toBeTruthy();
  });

  it("labels a SPACE measure with the system it came from, so nothing is double counted", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Experience \(SPACE\)/));

    await screen.findByText("Performance");
    // Change failure rate appears under Performance *sourced from DORA*, not as
    // a second, independently-computed number.
    expect(screen.getByText(/source: DORA stability/i)).toBeTruthy();
  });

  it("warns loudly when there is no outcome signal", async () => {
    // Volume and shape with nothing saying whether what shipped worked is the
    // reading SPACE exists to prevent, so it must be an alert and not a note.
    mockDaemonFetch.mockImplementation(
      routeTo({
        scorecard: SCORECARD,
        onboarding: ONBOARDING,
        space: { ...SPACE, outcome_signal: false, dimensions_measured: 2 },
      })
    );
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Experience \(SPACE\)/));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toMatch(/No outcome signal/i);
    expect(alert.textContent).toMatch(/not a picture of productivity/i);
  });

  it("never renders an aggregate SPACE score", async () => {
    render(<DeveloperExcellencePanel workspacePath="/src/demo" />);
    await screen.findByText("Deployment frequency");
    fireEvent.click(screen.getByText(/Experience \(SPACE\)/));

    await screen.findByText("Activity");
    // The disclaimer *mentions* the phrase, so match on a score being given a
    // value rather than on the words appearing anywhere.
    expect(screen.queryByText(/SPACE score[:\s]*[0-9]/i)).toBeNull();
    expect(screen.queryByText(/overall productivity/i)).toBeNull();
    expect(screen.queryByText(/productivity (score|index|rating)/i)).toBeNull();
    // And the denial is on screen, so a reader cannot assume one was withheld
    // for lack of data.
    expect(screen.getByText(/deliberately NO aggregate SPACE score/i)).toBeTruthy();
  });
});
