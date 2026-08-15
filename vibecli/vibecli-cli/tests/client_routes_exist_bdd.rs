//! Contract: every daemon route a client calls is a route the daemon serves.
//!
//! VibeCody is fourteen clients talking to one daemon, joined by URL strings.
//! Nothing checks the join. The daemon declares `.route("/v1/loops")`; a client
//! writes `format!("{}/v1/loops", base)`. Rename or drop the route and every
//! compiler stays quiet — the client 404s at runtime, and the panel blames the
//! daemon. `CLAUDE.md` lists the surfaces to touch when adding a route
//! precisely because nothing enforces it.
//!
//! This walks the join: collect the paths the routers serve, collect the paths
//! the clients call, assert containment.
//!
//! ── What it took to make this trustworthy ───────────────────────────────────
//!
//! The first three versions of this check all reported failures that were not
//! bugs, which is the state in which a test gets deleted rather than fixed:
//!
//!   * Comments. `/// POST /v1/loops — `args` is the `/loop` argument` made
//!     `/loop` look like a called route. Comments are stripped first.
//!   * Interpolation. `${sessionId}`, `$jobId`, `:id` and `{id}` are the same
//!     hole in four languages; all normalise to `{}`. A trailing hole is a
//!     query suffix (`${base}/v1/goals${qs}`), not a path segment.
//!   * Strings that are not URLs at all — `/proc/meminfo`, `/node_modules`,
//!     `/Users/test/.ssh/id_ed25519`. Only paths whose first segment is one
//!     the daemon actually serves are treated as daemon calls.
//!   * Servers inside clients. VibeCoder hosts its own chat gateway and
//!     declares `.route("/api/messages")` in the same file that calls the
//!     daemon. Route *declarations* are not client calls.
//!
//! ── The blind spot, named rather than hidden ────────────────────────────────
//!
//! The watch client builds URLs in a Swift style this scanner does not read,
//! so it contributes zero paths. Zero scanned is not zero wrong, so the scan
//! floor below fails if a client that should contribute paths stops doing so —
//! silence must not read as coverage.

use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // tests/ live in vibecli/vibecli-cli
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn strip_comments(text: &str) -> String {
    let block = Regex::new(r"(?s)/\*.*?\*/").unwrap();
    let line = Regex::new(r"(?m)^\s*(///|//).*$").unwrap();
    line.replace_all(&block.replace_all(text, ""), "").into_owned()
}

/// Collapse every language's parameter hole to `{}`, and drop a trailing hole
/// because that is a query string being appended, not another path segment.
fn normalise(path: &str) -> String {
    let base = path.split('?').next().unwrap_or(path);
    let interp = Regex::new(r"\$\{[^}]*\}").unwrap();
    let braced = Regex::new(r"\{[^}]*\}").unwrap();
    let dollar = Regex::new(r"\$[A-Za-z_][A-Za-z0-9_]*").unwrap();
    let colon = Regex::new(r":[A-Za-z_][A-Za-z0-9_]*").unwrap();

    let p = interp.replace_all(base, "{}");
    let p = braced.replace_all(&p, "{}");
    let p = dollar.replace_all(&p, "{}");
    let p = colon.replace_all(&p, "{}");
    let p = p.trim_end_matches('/').to_string();
    // A trailing hole is a query suffix only when it was glued straight onto a
    // segment (`${base}/v1/goals${qs}`). After a slash it is a real parameter
    // segment — `/stream/{session_id}` is a route, and stripping it collapsed
    // that to `/stream`, which would have matched nothing and reported a bug
    // in the daemon that did not exist.
    let p = match p.strip_suffix("{}") {
        Some(head) if !head.ends_with('/') => head.trim_end_matches('/').to_string(),
        _ => p,
    };
    if p.is_empty() { "/".to_string() } else { p }
}

struct Router {
    paths: HashSet<String>,
    first_segments: HashSet<String>,
}

fn routers() -> Router {
    let root = repo_root();
    let sources = [
        "vibecli/vibecli-cli/src/serve.rs",
        "vibecli/vibecli-cli/src/watch_bridge.rs",
    ];
    let route = Regex::new(r#"\.route\(\s*"([^"]+)""#).unwrap();

    let mut paths = HashSet::new();
    let mut first_segments = HashSet::new();
    for rel in sources {
        let file = root.join(rel);
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        for cap in route.captures_iter(&strip_comments(&text)) {
            let raw = &cap[1];
            if let Some(seg) = raw.split('/').nth(1) {
                if !seg.is_empty() {
                    first_segments.insert(seg.to_string());
                }
            }
            paths.insert(normalise(raw));
        }
    }
    Router { paths, first_segments }
}

struct Client {
    name: &'static str,
    file: &'static str,
    /// Fewer than this many daemon paths means the scanner stopped reading
    /// this client, not that the client stopped calling the daemon.
    floor: usize,
}

const CLIENTS: &[Client] = &[
    Client { name: "vibedesk", file: "vibedesk/src-tauri/src/commands.rs", floor: 10 },
    Client { name: "vibecoder", file: "vibecoder/src-tauri/src/commands.rs", floor: 10 },
    Client { name: "vscode", file: "vscode-extension/src/api-client.ts", floor: 20 },
    Client { name: "mobile", file: "vibemobile/lib/services/api_client.dart", floor: 20 },
];

fn called_paths(client: &Client, router: &Router) -> HashSet<String> {
    let file = repo_root().join(client.file);
    let text = std::fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
    let text = strip_comments(&text);

    // A client file may also *declare* routes — VibeCoder hosts its own chat
    // gateway. Those lines are a server, not a call.
    let declaration = Regex::new(r#"\.route\(\s*"[^"]+""#).unwrap();
    let text = declaration.replace_all(&text, "");

    // Anchored at the quote so the path slice of an absolute URL cannot match:
    // `https://api.line.me/v2/...` and `https://discord.com/api/v10/...` are
    // third-party endpoints this client also talks to, and their `/api/...`
    // tail was being read as a daemon route that does not exist.
    let literal = Regex::new(r#"[`"'](?:\$\{[^}]*\})?(/[A-Za-z0-9/_{}$.:%-]*)[`"']"#).unwrap();

    // A daemon call goes through one of these. Requiring a marker nearby is
    // what separates a real call from a path that merely looks like one — and
    // VibeCoder is full of the latter: it *generates* scaffolding, so its
    // command file contains `app.get('/api/users', handler)` and
    // `VITE_API_URL || '/api'` as string data. Those are code samples the app
    // writes into a new project, not routes it calls, and every earlier
    // version of this scan reported them as missing daemon routes.
    const MARKERS: &[&str] = &[
        "daemon_get", "daemon_post", "daemon_delete",
        "baseUrl", "base_url", "authedFetch", "daemonFetch",
        "send_authed", "reqwest", "http.get", "http.post", "_request",
    ];

    text.lines()
        .enumerate()
        .flat_map(|(i, line)| {
            // The URL is regularly on its own line, one or two below the call.
            let start = i.saturating_sub(2);
            let window: String = text.lines().skip(start).take(i - start + 1).collect::<Vec<_>>().join("\n");
            let near_call = MARKERS.iter().any(|m| window.contains(m));
            literal
                .captures_iter(line)
                .map(|c| c[1].to_string())
                .filter(move |_| near_call)
                .collect::<Vec<_>>()
        })
        .filter(|p| {
            p.split('/')
                .nth(1)
                .is_some_and(|seg| router.first_segments.contains(seg))
        })
        .map(|p| normalise(&p))
        .collect()
}

#[test]
fn given_the_daemon_declares_routes_then_the_scan_finds_them() {
    let router = routers();
    // Guards the router parse itself: if `.route(` extraction breaks, every
    // containment assertion below passes vacuously.
    assert!(
        router.paths.len() > 150,
        "router parse found only {} paths — the extractor is broken, not the daemon",
        router.paths.len()
    );
    assert!(router.paths.contains("/health"), "known route missing from parse");
}

#[test]
fn given_a_client_calls_the_daemon_then_the_route_exists() {
    let router = routers();
    let mut failures = Vec::new();

    for client in CLIENTS {
        let called = called_paths(client, &router);

        // Silence is not coverage. A scanner that stops reading a client would
        // otherwise report it perfectly clean.
        assert!(
            called.len() >= client.floor,
            "{}: found only {} daemon paths (floor {}). The scanner stopped \
             reading this client — fix the scan before trusting a pass.",
            client.name,
            called.len(),
            client.floor
        );

        for path in called {
            if !router.paths.contains(&path) {
                failures.push(format!("  {:<10} calls {} — no such route", client.name, path));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "These clients call daemon routes that do not exist. They 404 at \
         runtime, and the panel reports it as a daemon fault:\n{}",
        failures.join("\n")
    );
}

#[test]
fn given_a_route_is_renamed_then_the_check_notices() {
    // Guards the detector, not the code: pins that containment is actually
    // being evaluated against normalised paths, so a renamed route would be
    // reported rather than silently matching something else.
    let router = routers();
    assert!(!router.paths.contains("/v1/loops-renamed-does-not-exist"));

    assert_eq!(normalise("/v1/goals/${goalId}/start"), "/v1/goals/{}/start");
    assert_eq!(normalise("/mobile/dispatch/$taskId/cancel"), "/mobile/dispatch/{}/cancel");
    assert_eq!(normalise("/watch/goals/:id/start"), "/watch/goals/{}/start");
    assert_eq!(normalise("/stream/{session_id}"), "/stream/{}");
    // A trailing hole is a query suffix, not a segment.
    assert_eq!(normalise("/v1/goals${qs}"), "/v1/goals");
}
