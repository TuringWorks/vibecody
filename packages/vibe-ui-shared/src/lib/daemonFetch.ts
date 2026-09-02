/**
 * `fetch` for VibeCLI daemon routes — one implementation for all three shells.
 *
 * Nearly every daemon route sits behind `require_auth`. Only a small public set
 * is open (`/health`, `/models`, `/web`, `/pair`, `/v1/capabilities`, …), so a
 * plain `fetch` to anything else is a silent, permanent 401.
 *
 * # Why this file exists rather than a header on each call
 *
 * Three copies of the same eight-line "resolve the port, resolve the token,
 * set the header" block lived in `useVoiceSettings`, `useVoiceDuplex` and
 * `HarnessSection`. None of them retried on a 401, and `vibecli serve` mints a
 * fresh token on **every** start — which the shells trigger themselves by
 * autostarting the daemon. So a daemon restart broke each of those panels until
 * the whole app was relaunched.
 *
 * # And why a 401 is explained rather than reported
 *
 * A retry only helps when the token *changed*. It does not help when the token
 * file is simply wrong — which is what a shared, port-agnostic `daemon.token`
 * produced: a daemon on another port overwrote it and exited, and every client
 * on the machine authenticated against a live daemon with a dead one's
 * credential. For two days the user was told "Could not read speech settings
 * from the daemon (401). Is it running?" about a daemon that was.
 *
 * So on a 401 that a re-read does not fix, ask the backend what is actually
 * wrong — {@link daemonReadiness} compares the token's fingerprint against the
 * one `/health` publishes — and report *that*.
 */
import { invoke } from "@tauri-apps/api/core";

/** Default when the host registers no `daemon_port` command. */
const DEFAULT_PORT = 7878;

/**
 * How the daemon's bearer relates to the daemon actually on the port.
 *
 * Four states, not a boolean. `stale` and `missing` both fail every
 * authenticated request, and they have opposite fixes: `stale` needs the daemon
 * restarted, `missing` needs it *started*. Collapsing them is how one became
 * reported as the other.
 */
export type TokenState = "valid" | "stale" | "missing" | "unverifiable";

export interface DaemonReadiness {
  port: number;
  /** The daemon answered **and** we hold a credential it accepts. */
  ready: boolean;
  daemonRunning: boolean;
  daemonVersion: string | null;
  clientVersion: string;
  /**
   * Null when the daemon did not answer. `false` is the explanation for a route
   * returning 404 from a panel that is otherwise correct: the installed daemon
   * predates it.
   */
  versionMatches: boolean | null;
  tokenState: TokenState;
  /** `/health.features`, the daemon's own account of what it can do. */
  features: Record<string, unknown> | null;
  /** Ready to show a user, naming the fix. Never a bare failure. */
  message: string;
}

/** The daemon's base URL, honouring a host-configured port. */
export async function daemonUrl(): Promise<string> {
  try {
    return `http://127.0.0.1:${await invoke<number>("daemon_port")}`;
  } catch {
    return `http://127.0.0.1:${DEFAULT_PORT}`;
  }
}

/**
 * Read the bearer from whichever command the host shell registers.
 *
 * `daemon_token_effective` is the shared one (`vibe-desktop-voice`, registered
 * by all three shells); `daemon_auth_token` is VibeCoder's older name, kept as a
 * fallback so this one implementation serves every shell rather than each
 * keeping its own copy for the sake of a command name.
 *
 * A failure is not an error path: the daemon may be running without auth, and
 * the caller still issues the request so the real transport error surfaces
 * instead of an invented auth one.
 */
async function readToken(): Promise<string | null> {
  for (const attempt of [
    () => invoke<string | null>("daemon_token_effective", { explicit: null }),
    () => invoke<string | null>("daemon_auth_token"),
  ]) {
    try {
      const token = await attempt();
      if (typeof token === "string" && token.length > 0) return token;
      return null;
    } catch {
      /* try the next command name */
    }
  }
  return null;
}

/** Cached bearer. `undefined` means "not read yet"; `null` means "none". */
let cachedToken: string | null | undefined;
/** In-flight read, so concurrent callers share one `invoke`. */
let inFlight: Promise<string | null> | null = null;

/**
 * Current daemon token, or null when there is none. `force` bypasses the cache
 * after a 401.
 */
export async function getDaemonToken(force = false): Promise<string | null> {
  if (cachedToken !== undefined && !force) return cachedToken;
  if (!inFlight) {
    inFlight = readToken().finally(() => {
      inFlight = null;
    });
  }
  cachedToken = await inFlight;
  return cachedToken;
}

/** Drop the cached token — for tests and for an explicit re-auth. */
export function resetDaemonTokenCache(): void {
  cachedToken = undefined;
  inFlight = null;
}

/** The daemon's base URL and current bearer, together. */
export async function daemonBase(): Promise<{ url: string; token: string }> {
  const [url, token] = await Promise.all([daemonUrl(), getDaemonToken()]);
  return { url, token: token ?? "" };
}

function withAuth(init: RequestInit | undefined, token: string | null): RequestInit {
  if (!token) return init ?? {};
  const headers = new Headers(init?.headers);
  headers.set("Authorization", `Bearer ${token}`);
  return { ...init, headers };
}

/**
 * `fetch` a daemon route with the bearer attached, retrying once on 401 with a
 * freshly-read token.
 *
 * `path` is daemon-relative (`/voice/settings`); an absolute URL is used as-is.
 */
export async function daemonFetch(path: string, init?: RequestInit): Promise<Response> {
  const url = /^https?:\/\//.test(path) ? path : `${await daemonUrl()}${path}`;
  const token = await getDaemonToken();
  const res = await fetch(url, withAuth(init, token));
  if (res.status !== 401) return res;

  // A 401 on a token we believed was good usually means the daemon restarted
  // and rotated it. Re-read once. An unchanged token is a real auth failure —
  // retrying it would cost a second round-trip to learn the same thing.
  const fresh = await getDaemonToken(true);
  if (!fresh || fresh === token) return res;
  return fetch(url, withAuth(init, fresh));
}

/**
 * Ask the backend what state the daemon and our credential are actually in.
 *
 * Probes; never starts a daemon. A panel polling this must not resurrect one the
 * user deliberately stopped.
 *
 * `null` when the host registers no `daemon_readiness_probe` command — an older
 * shell, where the caller should fall back to its own message rather than
 * claiming to know something it does not.
 */
export async function daemonReadiness(): Promise<DaemonReadiness | null> {
  try {
    const reply = await invoke<unknown>("daemon_readiness_probe");
    return isReadiness(reply) ? reply : null;
  } catch {
    return null;
  }
}

/**
 * Does this reply actually carry a diagnosis?
 *
 * Narrowed, not cast. `invoke` returns whatever the other end sent, and the two
 * fields used to build a user-facing sentence must both really be there: a
 * reply missing `message` rendered as the literal words "Could not read speech
 * settings. undefined" — absent data turned into a claim, which is worse than
 * the status code it replaced.
 */
function isReadiness(v: unknown): v is DaemonReadiness {
  if (typeof v !== "object" || v === null) return false;
  const r = v as Partial<DaemonReadiness>;
  return typeof r.ready === "boolean" && typeof r.message === "string" && r.message.length > 0;
}

/**
 * Turn a failed daemon response into a sentence that names the real problem.
 *
 * `context` is what the caller was trying to do ("read speech settings"), used
 * verbatim so each panel keeps its own voice.
 *
 * The order matters: readiness is consulted **first** on a 401, because the
 * status code alone cannot tell a rotated token from a stale file from a daemon
 * that is genuinely absent, and guessing produced two days of the wrong advice.
 */
export async function describeDaemonFailure(
  context: string,
  res: Response | null,
  cause?: unknown,
): Promise<string> {
  if (res?.status === 401 || res?.status === 403 || !res) {
    const readiness = await daemonReadiness();
    // `ready === false`, explicitly. A reply that does not say is not a reply
    // that says no, and `isReadiness` has already guaranteed a real message.
    if (readiness?.ready === false) return `Could not ${context}. ${readiness.message}`;
  }
  if (res?.status === 404) {
    const readiness = await daemonReadiness();
    if (readiness?.versionMatches === false) {
      return (
        `Could not ${context}: the daemon does not have this route. It is version ` +
        `${readiness.daemonVersion ?? "unknown"} and this app is ${readiness.clientVersion}. ` +
        `Reinstall the daemon (\`cargo install --path vibecli/vibecli-cli\`) and restart it.`
      );
    }
  }
  if (res) return `Could not ${context} (the daemon returned ${res.status}).`;
  const detail = cause instanceof Error ? cause.message : String(cause ?? "unknown error");
  return `Could not ${context}: ${detail}. Is the VibeCLI daemon running?`;
}
