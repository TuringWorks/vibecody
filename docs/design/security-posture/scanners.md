# Security Posture — scanner contracts

Per-scanner design — what it scans, what it emits, where the
adapter lives, and what's deferred.

## 1. vulnerability_db adapter (existing)

**Wraps:** `vibecli_cli::vulnerability_db::VulnerabilityScanner`

**Inputs:** lockfiles (`Cargo.lock`, `package-lock.json`, `yarn.lock`,
`pnpm-lock.yaml`, `Pipfile.lock`, `poetry.lock`, `Gemfile.lock`,
`go.sum`) found in `workspace`. Source files for SAST regex sweep
(any `.rs` / `.ts` / `.js` / `.py` / `.go` / `.java`).

**Output → Category:** `DependencyCve` (lockfile hits), `Sast` (regex
hits), `Other("IaC")` (Terraform / k8s findings).

**Severity mapping:** `vulnerability_db::Severity` → `SecurityFinding::Severity`
1:1 (Critical / High / Medium / Low).

**Adapter location:** `security_posture::adapters::vulnerability_db`

**Fast path:** single-file SAST regex sweep on save (does not re-parse
lockfiles). Lockfile re-scan only on `Cargo.lock` / `package*.json` save.

## 2. sonar_rules adapter (existing)

**Wraps:** `vibe_ui_lib::sonar_rules::scan_content`

**Inputs:** every source file matching a rule's `applies_to`. Existing
builtin_rules covers ~2,600 rule definitions across languages.

**Output → Category:** `Sast` (security category rules), `CodeHealth`
(maintainability / reliability rules).

**Severity mapping:** Sonar `BLOCKER` → Critical, `CRITICAL` → High,
`MAJOR` → Medium, `MINOR` → Low, `INFO` → Info.

**Adapter location:** `security_posture::adapters::sonar`

**Fast path:** single-file `scan_content` on save (it's already
content-not-path so the adapter just hands it back).

## 3. health_score adapter (existing)

**Wraps:** `vibecli_cli::health_score::HealthEngine`

**Inputs:** workspace path. Engine walks the tree itself.

**Output → Category:** `CodeHealth`. Each dimension becomes one finding
(test coverage low, doc coverage low, complexity high, etc.) with the
remediation already populated by the engine.

**Severity mapping:** dimension `score` → severity by threshold:
score < 30 = High, < 60 = Medium, < 80 = Low, ≥ 80 = Info.

**Adapter location:** `security_posture::adapters::health`

**Fast path:** none — health metrics are inherently whole-workspace.
Re-runs only on workspace-open trigger.

## 4. Secret-leak scanner (new)

**Module:** `vibecli/vibecli-cli/src/security_posture_secrets.rs`

**Approach:**
1. Regex set covering the gitleaks default rules + 2026 additions
   (AWS access/secret keys, GitHub PATs, GitHub Apps, GCP service-
   account JSON, Slack tokens, OpenAI keys, Anthropic keys, Stripe
   live + test, generic JWTs, RSA/EC/DSA private keys, npm tokens,
   PyPI tokens, Cloudflare API tokens, Azure storage connection
   strings, Mailgun, SendGrid, Twilio).
2. Shannon entropy heuristic on any 20+ char string that doesn't
   match a known regex (catches custom token formats; threshold
   tuned to suppress base64-encoded code / hex digests).
3. `.gitignore`-aware traversal — `ignore` crate, so `node_modules`,
   `target`, `.venv` are skipped automatically.
4. Single-line context — finding snippet truncates the matched span
   to `<scheme>::***`, never the full value (Slice F invariant).

**Output → Category:** `SecretLeak`.

**Severity:** match-shape based — private-key headers = Critical,
known-issuer prefixes (`AKIA`, `ghp_`, `sk-`) = High, entropy
heuristic = Medium, generic-looking long opaque = Low.

**False-positive controls:**
- Per-line `// noseccode: <reason>` opt-out (mirrors `// nosemgrep:`)
- Workspace-level `.vibecli/secret-leak-allow.toml` for fingerprints
- Common test-fixture paths suppressed automatically (`**/*test*/**`,
  `**/fixtures/**`, `**/__snapshots__/**`)

**Fast path:** single-file regex sweep on save.

## 5. License clash scanner (new)

**Module:** `vibecli/vibecli-cli/src/security_posture_license.rs`

**Approach:**
1. Read the project's declared license from `Cargo.toml [package]
   license`, `package.json "license"`, `pyproject.toml [project]
   license`, or `LICENSE` file SPDX-Identifier line.
2. Walk dependency graph from lockfiles (reuse the lockfile parsers
   already in `vulnerability_db.rs`).
3. For each dependency, look up its declared SPDX:
   - Rust: `cargo metadata` is the canonical source; fall back to
     `crates.io` cached snapshot when offline.
   - JS: `package.json` of each installed dep in `node_modules`.
   - Python: `pyproject.toml` or `PKG-INFO` of each installed dist.
   - Go: `go.mod` + each module's `LICENSE` filename heuristic.
4. Run each through `vulnerability_db::classify_license(spdx)` →
   `LicenseRisk` (Permissive / Weak / Strong / Network / Restrictive
   / Unknown).
5. **Clash rules** (configurable in `.vibecli/license-policy.toml`):
   - Permissive project + Strong dep = Critical (MIT-licensed
     project pulling in a GPL dep is the classic "viral license"
     concern)
   - Any project + Unknown dep = High (no license declared)
   - Network-copyleft project + Permissive dep = ok (downgrade rule)
   - Default deny list: `AGPL-3.0`, `SSPL-1.0`, `Commons Clause` —
     surface High regardless of project license

**Output → Category:** `LicenseRisk`.

**Severity:** per clash-rule outcome (above).

**Fast path:** lockfile-save only. Tree walks are slow; debounce 10s.

## 6. TS / Python light taint scanner (new)

**Module:** `vibecli/vibecli-cli/src/security_posture_taint.rs`

**Approach (v1 — intra-procedural):**

1. Tree-sitter parse per file (existing tree-sitter crates already in
   `vibe-core` via the indexer).
2. Tag known **sources** in the AST:
   - TS: `process.argv`, `req.query[*]`, `req.body[*]`, `req.params[*]`,
     `req.headers[*]`, `JSON.parse(req.body)`, function params whose
     name matches `/^(input|user|raw|untrusted|payload|body|query)/i`
   - Python: `request.args[*]`, `request.form[*]`, `request.json`,
     `request.values[*]`, `sys.argv[*]`, `os.environ[*]`,
     `input()`, `Flask.request.data`
3. Tag known **sinks**:
   - Path traversal: `fs.readFile`, `fs.writeFile`, `open()`, `Path()`
     when concatenated, `shutil.*`, `os.path.join` + open
   - Command injection: `child_process.exec`, `os.system`,
     `subprocess.call` w/ `shell=True`, `eval`, `Function()`
   - SQL injection: `db.query(` / `cursor.execute(` with string
     concatenation
   - DOM XSS: `innerHTML =`, `outerHTML =`, `document.write(`,
     `dangerouslySetInnerHTML`
4. **Intra-procedural flow:** within a single function, mark a
   variable as tainted if it's assigned from a source. If a tainted
   variable reaches a sink without an obvious sanitizer between
   (`encodeURIComponent`, `path.normalize`, `path.resolve` +
   workspace-bound check, parameterized query API), emit a finding.
5. **Inter-procedural flow is explicitly out of scope for v1.**
   Documented limitation: if the source-tagged value passes through
   a helper function before reaching the sink, the scanner won't
   follow it. The scanner *does* tag the helper function's
   parameters with the suspicious-name heuristic (rule 2 above) as
   a partial mitigation.

**Output → Category:** `PathTraversal` / `PromptInjection` /
`Other("CommandInjection")` / `Other("SqlInjection")` /
`Other("DomXss")` per sink class.

**Severity:** sink class drives severity — command injection &
SQL injection = Critical, path traversal & DOM XSS = High,
others = Medium.

**Fast path:** single-file re-parse on save.

**Future work (deferred):**
- Inter-procedural dataflow (would need a real symbolic-execution
  engine like Sweetviz or a custom IR — multi-month build)
- Field-sensitive object tracking (req.body.x vs req.body.y)
- Cross-file imports (currently only function-local taint)

## Severity normalization across scanners

```
Critical = score 8.5+ (CVSS-equivalent)  →  red, escalates to top of feed
High     = score 7.0–8.4                 →  orange
Medium   = score 4.0–6.9                 →  yellow
Low      = score < 4.0                   →  blue
Info     = no security weight            →  grey, hidden by default
```

Each scanner's adapter is responsible for mapping its native
severity to one of these five buckets. Hardcoded thresholds live in
each adapter — no central mapping table because the per-scanner
heuristics diverge (sonar's "BLOCKER" is not the same beast as
CVSS 10.0).

## Promotion gates

A scanner moves from "stubbed" → "shipped" when:

1. Adapter / module compiles clean (`cargo check -p vibecli`)
2. Returns valid `SecurityFinding` records on a sample project (the
   VibeCody repo is the dogfooding target — it should produce ≥ 1
   finding from each scanner)
3. Fast-path single-file scan completes in < 200 ms on a typical
   source file
4. Suppression flow round-trips through `WorkspaceStore`
5. No `unwrap()` on parse errors — all scanners must be panic-free
   on malformed input (tested with property-based fuzz at module
   boundaries)
6. One unit test per scanner pinning at least one true positive +
   one true negative

## Anti-goals

- **Not a SARIF exporter** — VibeCody already emits SARIF via
  `vulnerability_db` (covers CI integration). This panel is the
  human-in-the-loop interactive surface, not the machine-readable
  export.
- **Not a vulnerability database** — relies on existing CVE / GHSA
  feeds (already wired into vulnerability_db). Doesn't try to
  maintain its own.
- **Not a code reviewer** — health_score's complexity / maintainability
  findings are surfaced but the panel doesn't second-guess code
  style choices; categories not in the DREAD vocabulary are
  intentionally absent.
