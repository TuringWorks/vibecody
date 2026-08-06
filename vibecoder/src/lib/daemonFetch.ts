/**
 * daemonFetch — `fetch` for VibeCLI daemon routes, with bearer auth attached.
 *
 * Almost every daemon route sits behind `require_auth`. Only a small public set
 * is open: `/health`, `/models`, `/web`, `/favicon.svg`, `/webhook/github`,
 * `/pair`, `/acp/v1/capabilities`, `/v1/capabilities`, `/ws/collab/{room}`,
 * `/mobile/beacon`. A panel that calls anything else with a plain `fetch()`
 * gets a blanket 401 — which is what happened to the Background Jobs panel and
 * the tainted-argument confirmation flow: both were completely non-functional,
 * and nothing on screen said why.
 *
 * **Token rotation is the reason this is a helper and not a one-line header.**
 * `vibecli serve` mints a fresh random token on *every* start, so any cached
 * token dies the moment the daemon restarts — and VibeCoder restarts it itself
 * on autostart. So: cache for speed, and on a 401 re-read once and retry. A
 * panel that cached the token at mount would break after the first restart and
 * stay broken until the whole app was relaunched.
 *
 * Use plain `fetch` for the public routes above; use this for everything else.
 */
import { invoke } from "@tauri-apps/api/core";

/** Cached bearer token. Null means "not fetched yet, or unavailable". */
let cachedToken: string | null = null;
/** In-flight read, so concurrent callers share one `invoke`. */
let inFlight: Promise<string | null> | null = null;

async function readToken(): Promise<string | null> {
  try {
    const token = await invoke<string>("daemon_auth_token");
    return typeof token === "string" && token.length > 0 ? token : null;
  } catch {
    // Daemon not running, or no token file yet. The caller still issues the
    // request so the real transport error surfaces rather than a made-up one.
    return null;
  }
}

/**
 * Current daemon token. Pass `force` to bypass the cache after a 401.
 * Concurrent callers share a single read.
 */
export async function getDaemonToken(force = false): Promise<string | null> {
  if (cachedToken !== null && !force) return cachedToken;
  if (!inFlight) {
    inFlight = readToken().finally(() => {
      inFlight = null;
    });
  }
  const token = await inFlight;
  cachedToken = token;
  return token;
}

/** Drop the cached token — exported for tests and for an explicit re-auth. */
export function resetDaemonTokenCache(): void {
  cachedToken = null;
  inFlight = null;
}

function withAuth(init: RequestInit | undefined, token: string | null): RequestInit {
  if (!token) return init ?? {};
  const headers = new Headers(init?.headers);
  headers.set("Authorization", `Bearer ${token}`);
  return { ...init, headers };
}

/**
 * `fetch` with the daemon bearer token attached, retrying once on 401 with a
 * freshly-read token (the daemon rotates it on every restart).
 */
export async function daemonFetch(
  input: string | URL,
  init?: RequestInit
): Promise<Response> {
  const token = await getDaemonToken();
  const res = await fetch(input, withAuth(init, token));
  if (res.status !== 401) return res;

  // 401 with a token we believed was good => the daemon restarted and rotated
  // it. Re-read once. If it hasn't actually changed, don't loop — return the
  // 401 so the caller reports a real auth failure instead of hanging.
  const fresh = await getDaemonToken(true);
  if (!fresh || fresh === token) return res;
  return fetch(input, withAuth(init, fresh));
}
