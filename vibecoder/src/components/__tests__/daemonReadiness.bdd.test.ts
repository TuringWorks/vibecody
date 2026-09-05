/**
 * BDD: a failed daemon call names what is actually wrong.
 *
 * The bug behind every case here was live for two and a half days. The daemon
 * on port 7878 was healthy, listening, and answering `/health` — and every
 * authenticated route returned 401, because a second daemon on port 7979 had
 * bound its own free port, written its token into the shared, port-agnostic
 * `~/.vibecli/daemon.token`, and then exited. Nothing rewrote the file. Nothing
 * compared the credential the client held against the one the daemon accepted.
 *
 * What the user was shown was: *"Could not read speech settings from the daemon
 * (daemon returned 401). Is it running?"* — a question with the answer "yes",
 * pointing at the one thing that was not the problem.
 *
 * So these tests are about the difference between a status code and a
 * diagnosis. A 401 has at least three causes with three different fixes, and a
 * 404 from a correct panel usually means the installed daemon predates the
 * route. Each must be said out loud, because a user cannot act on "401".
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  describeDaemonFailure,
  daemonReadiness,
  resetDaemonTokenCache,
  type DaemonReadiness,
} from "@vibe/shared/lib/daemonFetch";

function res(status: number): Response {
  return { ok: status >= 200 && status < 300, status } as Response;
}

/** A readiness reply with the fields a test does not care about filled in. */
function readiness(over: Partial<DaemonReadiness>): DaemonReadiness {
  return {
    port: 7878,
    ready: true,
    daemonRunning: true,
    daemonVersion: "0.5.12",
    clientVersion: "0.5.12",
    versionMatches: true,
    tokenState: "valid",
    features: null,
    message: "VibeCLI daemon 0.5.12 already running",
    ...over,
  };
}

/** Answer `daemon_readiness_probe` with `r`; everything else resolves null. */
function withReadiness(r: DaemonReadiness | null) {
  mockInvoke.mockImplementation((cmd: string) =>
    cmd === "daemon_readiness_probe"
      ? r === null
        ? Promise.reject(new Error("no such command"))
        : Promise.resolve(r)
      : Promise.resolve(null),
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
  resetDaemonTokenCache();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Given a healthy daemon whose token file belongs to a daemon that exited", () => {
  it("When a route 401s, Then the message says restart — not 'is it running'", async () => {
    // The exact observed state: daemon up, credential wrong.
    withReadiness(
      readiness({
        ready: false,
        tokenState: "stale",
        message:
          "The saved daemon token (1b7d…) is not the one the daemon on port 7878 is " +
          "accepting (f392…). Another daemon overwrote it and has since exited. Restart " +
          "the daemon on port 7878 — it rewrites the token on every start.",
      }),
    );

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).toContain("Restart the daemon");
    expect(msg.toLowerCase()).not.toContain("is it running");
    // And it still says which task failed, so the panel keeps its own voice.
    expect(msg).toContain("read speech settings");
  });
});

describe("Given no daemon at all", () => {
  it("When a route fails, Then the message says to start one, not to restart", async () => {
    // `missing` and `stale` both 401 forever and have opposite fixes. This is
    // the distinction the old boolean could not carry.
    withReadiness(
      readiness({
        ready: false,
        daemonRunning: false,
        tokenState: "missing",
        message:
          "No bearer token for the VibeCLI daemon on port 7878. Start it with " +
          "`vibecli --serve --port 7878`; it writes the token on start.",
      }),
    );

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).toContain("vibecli --serve");
    expect(msg).not.toContain("Restart");
  });
});

describe("Given an installed daemon older than the app", () => {
  it("When a route 404s, Then the version gap is named rather than the panel blamed", async () => {
    // Observed with `/harness/profiles` and `/observe/config`: the routes exist
    // in the source tree, the installed binary predates them, and the panel
    // simply appears broken.
    withReadiness(readiness({ daemonVersion: "0.5.9", versionMatches: false }));

    const msg = await describeDaemonFailure("load harness profiles", res(404));

    expect(msg).toContain("0.5.9");
    expect(msg).toContain("does not have this route");
    expect(msg).toContain("cargo install");
  });

  it("When a 404 is a genuine 404, Then no version story is invented", async () => {
    withReadiness(readiness({ versionMatches: true }));

    const msg = await describeDaemonFailure("load harness profiles", res(404));

    expect(msg).toContain("404");
    expect(msg).not.toContain("cargo install");
  });
});

describe("Given a daemon that is fine and a request that failed for another reason", () => {
  it("When the status is 500, Then it is reported as-is without a readiness guess", async () => {
    withReadiness(readiness({}));

    const msg = await describeDaemonFailure("save speech settings", res(500));

    expect(msg).toContain("500");
    // Readiness is only consulted for the codes it can explain; a 500 is the
    // daemon's own failure and inventing a cause would be worse than the code.
    expect(mockInvoke).not.toHaveBeenCalledWith("daemon_readiness_probe");
  });

  it("When a 401 is transient and readiness says ready, Then the code is reported", async () => {
    // Readiness disagreeing with the response is possible — the daemon may have
    // restarted between the two calls. Claiming a diagnosis we no longer have
    // evidence for would be the same substitution in the other direction.
    withReadiness(readiness({ ready: true }));

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).toContain("401");
  });
});

describe("Given a shell too old to answer a readiness probe", () => {
  it("When the command is missing, Then readiness is null rather than a guess", async () => {
    withReadiness(null);
    expect(await daemonReadiness()).toBeNull();
  });

  it("When a call fails, Then the message falls back to the status code", async () => {
    withReadiness(null);

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).toContain("401");
    expect(msg).toContain("read speech settings");
  });
});

describe("Given a readiness reply that is not one", () => {
  it("When the reply has no message, Then no sentence is built from it", async () => {
    // A shell whose `daemon_readiness_probe` answers with something else — an
    // older build, a mocked host, a partial payload. Rendering the missing
    // field produced the literal user-facing text "Could not read speech
    // settings. undefined": absent data promoted to a claim, which is strictly
    // worse than the status code it was meant to improve on.
    mockInvoke.mockImplementation(() => Promise.resolve("tok"));

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).not.toContain("undefined");
    expect(msg).toContain("401");
  });

  it("When the reply does not say ready, Then it is not read as not-ready", async () => {
    // `ready` absent is not `ready: false`. Treating the two alike is the same
    // substitution in the other direction.
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === "daemon_readiness_probe"
        ? Promise.resolve({ message: "something went wrong" })
        : Promise.resolve(null),
    );

    const msg = await describeDaemonFailure("read speech settings", res(401));

    expect(msg).not.toContain("something went wrong");
    expect(msg).toContain("401");
  });

  it("When the reply is well-formed, Then it is used", async () => {
    withReadiness(readiness({ ready: false, tokenState: "stale", message: "Restart the daemon." }));
    expect(await describeDaemonFailure("read speech settings", res(401))).toContain(
      "Restart the daemon.",
    );
  });
});

describe("Given the daemon could not be reached at all", () => {
  it("When there is no response, Then the transport error is named with the daemon", async () => {
    withReadiness(null);

    const msg = await describeDaemonFailure(
      "read speech settings",
      null,
      new TypeError("Failed to fetch"),
    );

    // "Failed to fetch" alone sends people to their network settings for a
    // process that is simply not running on their own machine.
    expect(msg).toContain("Failed to fetch");
    expect(msg).toContain("VibeCLI daemon");
  });
});
