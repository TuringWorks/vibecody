---
triggers: ["secrets scanning", "GitLeaks", "TruffleHog", "secret detection", "API key leak", "credential scanning", "git secrets", "secret rotation", "pre-commit secrets"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Secrets Detection and Management

When working with secrets scanning:

1. Scan the current working tree for secrets using GitLeaks: `gitleaks detect --source . --report-format json --report-path gitleaks-report.json` checks all files for API keys, tokens, passwords, and certificates; use `--verbose` to see matched rules and `--no-banner` for clean CI output with exit code 1 on findings.

2. Scan the entire Git history for previously committed secrets: `gitleaks detect --source . --log-opts="--all" --report-format json --report-path gitleaks-history.json` examines every commit in all branches, catching secrets that were committed and later deleted but remain in Git history; this is critical for incident response.

3. Run TruffleHog for high-fidelity secret detection with verified results: `trufflehog git file://. --json > trufflehog-results.json` scans Git history, `trufflehog filesystem . --json` scans current files, and `trufflehog github --org=myorg --json` scans all repos in a GitHub org; TruffleHog verifies secrets against live APIs to confirm they are active.

4. Enable GitHub secret scanning and push protection at the organization level: navigate to Settings > Code security > Secret scanning, enable for all repositories, and activate push protection to block pushes containing detected secrets; developers see inline warnings with `gh secret-scanning list --repo owner/repo` to review alerts.

5. Install pre-commit hooks to catch secrets before they enter Git history: add to `.pre-commit-config.yaml`: `repos: [{repo: https://github.com/gitleaks/gitleaks, rev: v8.18.0, hooks: [{id: gitleaks}]}]` and run `pre-commit install` to activate; developers get immediate feedback on `git commit` if secrets are detected.

6. Configure custom regex patterns for organization-specific secret formats: create `.gitleaks.toml` with `[[rules]]` entries defining `id`, `description`, `regex` (e.g., `(?i)mycompany[_-]?api[_-]?key\s*[:=]\s*['"]?([a-zA-Z0-9]{32,})`), and `secretGroup = 1`; add `[allowlist]` with `paths` and `regexes` for known false positives like test fixtures.

7. Detect high-entropy strings that may be secrets using entropy-based scanning: `trufflehog filesystem . --entropy` applies Shannon entropy analysis to find random-looking strings above threshold, complementing regex-based rules; tune with custom entropy ranges in GitLeaks using `entropy = 4.5` in rule definitions to balance sensitivity.

8. Scan historical commits efficiently by targeting specific time ranges: `gitleaks detect --source . --log-opts="--since=2026-01-01 --until=2026-03-07" --report-format json` limits scope to recent commits for faster daily scans, while scheduling full-history scans weekly with `--log-opts="--all"` to catch secrets in merged branches.

9. Automate secret rotation when a leak is detected: immediately revoke the exposed credential via its provider API (e.g., `aws iam delete-access-key`, `gh auth token --revoke`), generate a new secret, update it in your vault with `vault kv put secret/myapp/api-key value=$NEW_KEY`, redeploy affected services, and notify the security team.

10. Integrate with HashiCorp Vault or AWS Secrets Manager to eliminate hardcoded secrets: replace inline secrets with references like `vault kv get -field=api_key secret/myapp`, use dynamic secrets with `vault read database/creds/myapp-role` for short-lived database credentials, and configure applications to fetch secrets at runtime instead of reading from config files.

11. Build a `.gitignore` and `.gitleaksignore` strategy to prevent common leaks: add `*.pem`, `*.key`, `.env`, `.env.*`, `credentials.json`, `serviceAccountKey.json` to `.gitignore`; create `.gitleaksignore` with fingerprints of verified false positives (e.g., test fixtures, documentation examples) with comments explaining each suppression.

12. Generate secrets scanning metrics and compliance reports: parse scanner JSON output with `jq '[.[] | {rule: .RuleID, file: .File, commit: .Commit}] | group_by(.rule) | map({rule: .[0].rule, count: length})' gitleaks-report.json` to identify most common leak types, track trend of findings over time, and report mean-time-to-rotate for leaked credentials as a key security KPI.
