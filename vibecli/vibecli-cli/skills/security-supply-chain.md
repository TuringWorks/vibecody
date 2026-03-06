---
triggers: ["dependency audit", "lockfile", "SBOM", "CVE", "supply chain", "npm audit", "cargo audit", "dependabot"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Supply Chain Security

When managing dependency security:

1. Always commit lockfiles (`Cargo.lock`, `package-lock.json`, `poetry.lock`) to version control
2. Run `cargo audit`, `npm audit`, `pip-audit` in CI — fail builds on critical/high CVEs
3. Pin exact dependency versions in production — use ranges only in libraries
4. Enable Dependabot or Renovate for automated dependency update PRs
5. Review new dependencies before adding: check maintenance status, download counts, license
6. Use `--ignore-scripts` for npm install in CI — prevent install-time code execution
7. Generate SBOM (Software Bill of Materials) with `syft` or `cdxgen` — attach to releases
8. Verify package integrity: check SHA hashes, use `npm ci` over `npm install` in CI
9. Use signed commits and tags for releases — verify GPG signatures
10. Monitor advisories: subscribe to GitHub Security Advisories for your dependencies
11. Minimize dependency count — each dependency is an attack surface
12. Use `socket.dev` or `snyk` for real-time supply chain threat detection
