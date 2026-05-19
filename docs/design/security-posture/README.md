# Security Posture — design

**Status:** in flight 2026-05-18 · **Owner:** security workstream · **Related:** [`threat-model.md`](../../security/threat-model.md)

## What this is

A new VibeUI panel and matching daemon module that **scans every codebase
loaded into VibeCody** for the vulnerability classes catalogued in the
top-20 DREAD ledger, aggregates findings from every available scanner
into a single severity-ranked feed, and lets the user one-click promote
a finding into the existing Goals system as an actionable work item.

This is the *outward-facing* twin of the DREAD ledger. The ledger
protects VibeCody itself; this panel applies the same vocabulary to
the user's own projects.

## Motivation

VibeCody already has six scanner backends totalling ~7.7k lines of
production Rust:

| Module | LOC | What it scans |
|---|---|---|
| `vulnerability_db.rs` | 3,428 | Dependency CVEs, SAST regex rules, IaC findings, license risk, full lockfile parsing |
| `sonar_rules.rs` | 2,619+ | SonarQube-compatible source-code issues per language |
| `health_score.rs` | 1,711 | Code-quality dimensions + remediation suggestions |
| `security_scan.rs` | 1,257 | Top-level scan orchestration |
| `security_hardening.rs` | 821 | Hardening-rule database |
| `security_scanning.rs` | 463 | Scan-result persistence |

And three UI panels touch parts of it:

- `SecurityPanel.tsx` (224 lines) — overview
- `SecurityScanPanel.tsx` (958 lines) — runs vulnerability_db
- `HealthScorePanel.tsx` (197 lines) — runs health_score

Each panel speaks to one scanner. **No surface today gives the user a
single ranked list of every security finding across every scanner**,
and **no surface ties a finding to actionable work**. A user has to
visit three panels, mentally merge the outputs, and manually create
Goals to remember to fix anything.

Security Posture is the missing aggregator + workflow layer.

## Scope (per 2026-05-18 user direction)

Initial scanner set:

1. **Existing-scanner adapters** (vulnerability_db, sonar_rules,
   health_score, security_scan) — wrap their outputs into the
   unified `SecurityFinding` shape.
2. **Secret-leak scanner** — gitleaks-equivalent regex set + Shannon
   entropy heuristic for long opaque strings; `.gitignore`-aware
   traversal.
3. **License clash scanner** — parse Cargo.toml / package.json /
   pyproject.toml / go.mod dependency trees, run each dependency's
   declared SPDX through the existing `classify_license()` helper in
   `vulnerability_db.rs`, flag risk mismatches against the project's
   own declared license.
4. **TS / Python light taint scanner** — tree-sitter parse + intra-
   procedural source → sink dataflow (path traversal, command
   injection, SQL injection). Heavy inter-procedural taint is out of
   scope for v1 — clearly marked as future work in `scanners.md`.

UI:

- One new dedicated panel: `SecurityPosturePanel.tsx`, registered in
  `EnterpriseGovernanceComposite` next to MCP / Plugin Governance.
- Severity-ranked feed, filter by category / file / status, detail
  pane on selection, "Create work item" → Goal, "Suppress with
  reason" → suppression DB with audit trail.

Work-item destination:

- The Goals system (the G8/G9 series shipping in parallel). Every
  "Create work item" mints a Goal with severity / file / CWE
  metadata as goal context.

## Out of scope for v1

- Container-image CVE scanning (would need Trivy or equivalent shell-out)
- Runtime PII redaction patterns (separate workstream)
- Inter-procedural taint analysis (would need a real static analyzer;
  intra-procedural heuristic covers most prompt-injection-relevant cases)
- SBOM generation (already handled by `release.yml::sbom` for VibeCody
  itself; user-project SBOM is a separate feature)

## Architecture (high-level)

```
┌─────────────────────────────────────────────────────────────┐
│ SecurityPosturePanel.tsx  (React)                           │
│  ┌─────────────────┬────────────────────┬─────────────────┐│
│  │  Feed (ranked)  │  Detail            │  Filters        ││
│  │  Critical 12    │  CWE-22 (path)     │  ☐ Suppressed   ││
│  │  High     34    │  src/api/files.rs  │  ☐ Goal-linked  ││
│  │  Medium   89    │  Line 42           │  Scanner: all   ││
│  │  Low      ...   │  [Create Goal]     │  Category: all  ││
│  │                 │  [Suppress]        │                 ││
│  └─────────────────┴────────────────────┴─────────────────┘│
└──────────────────────────────▲──────────────────────────────┘
                               │  Tauri invoke()
                               │
        ┌──────────────────────┴───────────────────────┐
        │  commands.rs                                  │
        │    security_posture_scan(workspace)           │
        │    security_posture_findings(workspace)       │
        │    security_posture_suppress(...)             │
        │    security_posture_create_goal(...)          │
        │    security_posture_decisions_log(...)        │
        └──────────────────────┬───────────────────────┘
                               │
        ┌──────────────────────▼───────────────────────┐
        │  vibecli-cli/src/security_posture.rs          │
        │                                                │
        │  pub struct SecurityFinding { ... }            │
        │  pub trait Scanner { fn scan(&self, ...) }     │
        │  pub fn run_all_scanners(workspace) -> ...     │
        │                                                │
        │     ┌─────────────────┬─────────────────┐     │
        │     ▼                 ▼                  ▼     │
        │  Adapters       New scanners        Suppress   │
        │  ─────────      ─────────────       ────────   │
        │  vulndb         secrets.rs           Workspace │
        │  sonar          license.rs           Store     │
        │  health         taint.rs                       │
        │  hardening                                     │
        └────────────────────────────────────────────────┘
```

## Files added by this workstream

```
docs/design/security-posture/
  README.md         # this file
  scanners.md       # per-scanner contract + future-work TODOs
  panel.md          # UI states, keyboard map, accessibility

vibecli/vibecli-cli/src/
  security_posture.rs              # finding shape + trait + aggregator
  security_posture_secrets.rs      # secret-leak scanner
  security_posture_license.rs      # license clash scanner
  security_posture_taint.rs        # TS/Python intra-procedural taint
  security_posture_store.rs        # WorkspaceStore-backed suppression + goal-link

vibeui/src-tauri/src/
  commands.rs   # +6 Tauri commands (delegating to the daemon module)

vibeui/src/components/
  SecurityPosturePanel.tsx
  composite/EnterpriseGovernanceComposite.tsx   # +1 tab entry
```

## Data model

```rust
/// One security finding, scanner-agnostic. All scanners emit this.
pub struct SecurityFinding {
    /// SHA-256 of (scanner | category | file | line | rule_id),
    /// truncated to 16 hex chars. Used as the suppression key and
    /// the goal-link key — same finding from two scans hashes
    /// identically.
    pub id: String,

    pub severity: Severity,    // Critical / High / Medium / Low / Info
    pub category: Category,    // PromptInjection / PathTraversal / SecretLeak / DepCve / ...
    pub scanner: String,       // "vulnerability_db" / "sonar" / "health" / "secrets" / ...

    pub file: PathBuf,         // workspace-relative
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub snippet: Option<String>,    // ≤ 240 chars, redaction-safe (no payload bytes)

    pub rule_id: String,       // CWE / OWASP / scanner-specific
    pub title: String,
    pub remediation: Option<String>,
    pub references: Vec<String>,    // URLs to CWE / advisory pages

    pub status: FindingStatus,      // Open / Suppressed { reason } / GoalLinked { goal_id } / Fixed
    pub first_seen: i64,            // unix ms — when this finding-id first appeared
    pub last_seen: i64,             // unix ms — most recent scan that produced it
}

pub enum Category {
    /// DREAD #1 vocabulary, applied to user code.
    PromptInjection,
    /// DREAD #2.
    PathTraversal,
    /// New, gitleaks-equivalent.
    SecretLeak,
    /// Existing vulnerability_db output.
    DependencyCve,
    /// Existing sonar_rules output (XSS, SQLi, command-injection, weak-crypto, …).
    Sast,
    /// New, cargo-deny-equivalent.
    LicenseRisk,
    /// Existing health_score output (complexity, doc coverage, test coverage, …).
    CodeHealth,
    /// Catch-all for IaC / DOM sinks / etc.
    Other(String),
}

pub trait Scanner: Send + Sync {
    fn name(&self) -> &'static str;
    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>>;
}
```

## Tauri command surface

All commands take `workspace_path: String` and are gated through
`reject_sensitive_path` (the same helper protecting every other
path-input command — see [`threat-model.md`](../../security/threat-model.md)
row #2).

```rust
security_posture_scan(workspace_path)           -> Vec<SecurityFinding>   // runs all enabled scanners
security_posture_findings(workspace_path)       -> Vec<SecurityFinding>   // cached / persisted
security_posture_suppress(workspace_path, id, reason)  -> ()
security_posture_unsuppress(workspace_path, id)        -> ()
security_posture_create_goal(workspace_path, id)       -> String          // returns goal_id
security_posture_decisions_log(workspace_path, limit)  -> Vec<DecisionLogEntry>
```

## Persistence

Stored in the existing `WorkspaceStore` (encrypted SQLite at
`<workspace>/.vibecli/workspace.db`). No schema migration —
new setting keys only:

| Key | Value |
|---|---|
| `posture:finding:<id>` | JSON `SecurityFinding` (last seen) |
| `posture:suppress:<id>` | `{ reason, at, who }` |
| `posture:goal_link:<id>` | `{ goal_id, at }` |
| `posture:decision_log` | append-only JSONL within a single `Vec<DecisionLogEntry>` row, capped at 1000 entries with FIFO eviction |

## Trigger model

- **Workspace open** — fire `security_posture_scan` once the indexer
  is ready (`vibe_core::index` reports complete). Debounced to one
  scan per minute per workspace.
- **File save** — fire single-file fast path on every save (sonar,
  secrets, taint) — full scan kicks in only on explicit "Rescan all".
- **Manual** — "Rescan all" button in the panel header always works.

Suppressions and goal-links survive across scans because the
`SecurityFinding.id` hash is stable for unchanged file content.

## Audit log invariant

Every state-changing operation (`suppress`, `unsuppress`,
`create_goal`) appends a JSONL line to the decision log with the
finding id, the operation, the reason text (suppress only), and a
timestamp. A future security review can read this back to
reconstruct who-suppressed-what-when without diffing the store.

## Threat-model touchpoints

This module **is itself a high-value path-input + LLM-context
surface**. Two specific invariants:

1. The aggregator never sends finding snippets through any LLM
   without the same `Tainted<T>` boundary the rest of the daemon
   uses. Sonar / vulnscan outputs that contain source code excerpts
   are wrapped as `Tainted<String>` with `Provenance::File` at the
   scanner boundary, exactly as `rag_taint.rs` does today.
2. The "Create Goal" bridge surfaces only finding metadata
   (severity, file, CWE, title, remediation) into the goal context
   — never the raw matched snippet. This prevents the "secret-leak
   finding's matched text" from leaking into a goal description
   visible in the next LLM turn.

Both are enforced by type — the bridge function signature takes
`&SecurityFinding` and constructs the goal context from named
fields, not from a `Display` impl.

## Status

| Slice | Scope | Status |
|---|---|---|
| Foundation | Design doc, finding shape, scanner trait, aggregator skeleton, persistence | ✅ shipped 2026-05-18 |
| Adapters | vulnerability_db + health_score wrapped as Scanner impls (sonar adapter deferred — sonar_rules lives Tauri-side, needs promotion to vibe-core first) | ✅ shipped 2026-05-18 |
| Panel | `SecurityPosturePanel.tsx` — two-pane layout, severity-grouped feed, filters, detail pane, suppress / unsuppress / create-goal action wiring | ✅ shipped 2026-05-18 |
| Secret-leak scanner | 30+ regex rules (AWS, GH, OpenAI, Anthropic, Slack, Stripe, GCP, Cloudflare, Twilio, SendGrid, Mailgun, npm, PyPI, Azure, JWT, private keys, hardcoded passwords) + redaction (`KeepPrefix` / `KeepIssuerPrefix` / `Hidden`) + dedup + `// nosecpost:` inline opt-out + test-fixture skip + 17 unit tests | ✅ shipped 2026-05-18 |
| License clash scanner | Manifest license parsing (Cargo / package.json / pyproject) + direct-dep walk + clash rules over `classify_license` (Permissive+GPL = Critical, AGPL = High always, Unknown = High, missing project license = skip) + 11 unit tests | ✅ shipped 2026-05-18 |
| TS / Python taint scanner | Module + Scanner trait impl shipped as fail-safe stub (returns empty findings); tree-sitter intra-procedural source→sink algorithm sketched in `scanners.md` §6 | 🟡 stub shipped 2026-05-18; real impl next slice |
| Goal bridge | `security_posture_create_goal` ↔ Goals system | 🟡 stub returning NotImplemented shipped; full wiring next slice |
| Sonar adapter | Promote `sonar_rules` from `vibeui/src-tauri/src/` to `vibe-core` and wrap as a Scanner impl | next slice |

Promotion gates between slices are tracked in `scanners.md`.
