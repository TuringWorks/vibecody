---
triggers: ["dependency scanning", "npm audit", "cargo audit", "Dependabot", "Renovate", "pip audit", "OSV", "dependency vulnerability", "SCA scanning", "license scanning"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Dependency and SCA Scanning

When working with dependency scanning:

1. Run language-native audit tools as the first line of defense: `npm audit --json > npm-audit.json` for Node.js, `cargo audit --json > cargo-audit.json` for Rust, `pip-audit -f json -o pip-audit.json` for Python, `bundle-audit check --format json` for Ruby, and `go vuln check ./...` for Go; each consults language-specific vulnerability databases.

2. Query OSV.dev for cross-ecosystem vulnerability data: `curl -s -X POST "https://api.osv.dev/v1/query" -d '{"package":{"name":"lodash","ecosystem":"npm"}}' | jq '.vulns[] | {id, summary, affected[0].ranges}'` to find known vulnerabilities across npm, PyPI, crates.io, Go, Maven, and more using a single unified API.

3. Configure Dependabot for automated vulnerability alerts and PRs by creating `.github/dependabot.yml`: set `package-ecosystem` for each language, `schedule.interval: daily`, `open-pull-requests-limit: 10`, and `target-branch: main`; Dependabot will auto-create PRs that bump vulnerable dependencies to patched versions.

4. Use Renovate as a more configurable alternative to Dependabot by adding `renovate.json`: `{"extends": ["config:base", "security:openssf-scorecard"], "vulnerabilityAlerts": {"enabled": true}, "automerge": true, "automergeType": "pr"}` enables auto-merge for patch-level security updates with full lockfile maintenance.

5. Scan lockfiles to catch transitive vulnerabilities that manifest files miss: `grype file:package-lock.json`, `osv-scanner --lockfile=Cargo.lock`, or `trivy fs --scanners vuln --file-patterns "pip:requirements.txt" .` to ensure the full resolved dependency tree is analyzed, not just direct dependencies.

6. Verify lockfile integrity to prevent dependency confusion attacks: `npm ci` (not `npm install`) enforces exact lockfile versions, `cargo build --locked` fails if `Cargo.lock` is stale, and `pip install --require-hashes -r requirements.txt` verifies package checksums match expected values.

7. Analyze transitive dependency chains to understand vulnerability paths: `npm ls vulnerable-package` shows the dependency path, `cargo tree -i vulnerable-crate` shows reverse dependencies, and `pip show -v package | grep Requires` maps the graph; fix by upgrading the nearest direct ancestor that pulls in the vulnerable transitive.

8. Pin dependency versions strictly to prevent unexpected updates: use exact versions in lockfiles (`=1.2.3` not `^1.2.3`), enable `Cargo.lock` in version control for applications (not libraries), and configure `pip-compile --generate-hashes requirements.in > requirements.txt` for reproducible Python builds with integrity verification.

9. Implement automated PR remediation by combining scanner output with PR creation: `npm audit fix --force` auto-patches Node.js, `cargo update -p vulnerable-crate` updates Rust, or script custom remediation with `jq '.advisories[] | .id' cargo-audit.json | xargs -I{} cargo update -p {}` followed by `git checkout -b fix/deps && git commit -am "fix: update vulnerable deps" && gh pr create`.

10. Perform license scanning alongside vulnerability scanning: `syft . -o json | jq '[.artifacts[] | {name, version, licenses: [.licenses[].value]}]'` extracts license data, `licensefinder` provides policy-based approval workflows, and configure deny-lists for copyleft licenses (GPL-3.0, AGPL-3.0) that conflict with your project's licensing.

11. Integrate dependency scanning as a CI gate: add `osv-scanner -r . --format json --output osv-results.json` to your pipeline, check exit code (non-zero = vulnerabilities found), and configure severity thresholds with `--config osv-scanner.toml` containing `[[IgnoredVulns]]` entries for accepted risks with documented justification and expiration dates.

12. Track dependency health metrics over time: count total dependencies (`jq '.packages | length' package-lock.json`), measure percentage with known vulnerabilities, track mean-time-to-update for security patches, monitor dependency age with `npm outdated --json | jq '[.[] | select(.current != .latest)] | length'`, and alert when critical dependencies fall more than one major version behind.
