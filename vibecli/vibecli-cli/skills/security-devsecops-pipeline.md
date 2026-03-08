---
triggers: ["DevSecOps", "devsecops", "security pipeline", "shift left security", "security gate", "DefectDojo", "vulnerability SLA", "security CI/CD", "security orchestration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# DevSecOps Pipeline Integration

When working with DevSecOps pipelines:

1. Implement shift-left by embedding security tools at the earliest possible stage: add pre-commit hooks for secrets scanning (`gitleaks`), SAST linting (`semgrep --config=auto`), and IaC checks (`checkov -d .`); developers get immediate feedback before code leaves their workstation, reducing the cost of fixing issues by 10-100x compared to production discovery.

2. Design security gates in CI/CD with clear pass/fail criteria: define gate thresholds like `--fail-on critical` for Trivy, `--severity-threshold high` for Semgrep, and `exit-code 1` for `npm audit --audit-level=high`; implement gates as required GitHub Actions checks or GitLab pipeline stages that block merge when thresholds are exceeded.

3. Orchestrate SAST, DAST, and SCA scanners in parallel CI stages for comprehensive coverage: run Semgrep (SAST), Grype (SCA), and Trivy (container) simultaneously in the build pipeline, then aggregate results in a post-scan step; use GitHub Actions matrix strategy or GitLab parallel jobs to keep pipeline duration under 10 minutes.

4. Aggregate findings in DefectDojo as a central vulnerability management platform: `curl -X POST "$DEFECTDOJO_URL/api/v2/import-scan/" -H "Authorization: Token $DD_TOKEN" -F "scan_type=Semgrep JSON Report" -F "file=@semgrep-results.json" -F "engagement=$ENGAGEMENT_ID" -F "auto_create_context=true"` for each scanner output; DefectDojo deduplicates, tracks status, and provides trend dashboards.

5. Establish a security champion program by assigning one developer per team as the security point of contact: champions attend monthly security training, review security-flagged PRs, triage scanner findings for their team, maintain team-specific `.semgrepignore` and suppression files, and escalate confirmed vulnerabilities to the AppSec team.

6. Define vulnerability SLAs based on severity and exposure: Critical (CVSS 9.0+) = 24 hours for internet-facing / 72 hours for internal, High (7.0-8.9) = 7 days / 14 days, Medium (4.0-6.9) = 30 days / 60 days, Low (0.1-3.9) = 90 days / next release; track SLA compliance with `jq '[.[] | select(.sla_breached == true)] | length' defectdojo-findings.json`.

7. Configure break-the-build policies that balance security and velocity: block merges on new Critical/High findings only (not pre-existing), allow bypass with security team approval via `gh pr review --approve` from a `@security-team` member, and use baseline comparison (`semgrep ci --baseline-commit $BASE_SHA`) to only flag newly introduced issues.

8. Build security dashboards for engineering and executive audiences: use Grafana with DefectDojo as a data source to visualize open vulnerabilities by severity over time, mean-time-to-remediate (MTTR) per team, SLA compliance rates, scanner coverage percentage across repositories, and trend of new vs resolved findings per sprint.

9. Collect compliance evidence automatically from CI/CD artifacts: archive scanner reports, SBOM attestations, and test results as pipeline artifacts with `actions/upload-artifact@v4`; tag releases with evidence bundles including SAST report, SCA scan, container scan, DAST baseline, and signed SBOM for SOC 2, ISO 27001, and FedRAMP audit preparation.

10. Implement automated remediation for common vulnerability patterns: configure Dependabot/Renovate for dependency updates, use Semgrep autofix rules (`fix:` key in rules) to auto-patch code issues, create custom GitHub Actions that auto-apply fixes and open PRs with `gh pr create --title "fix(security): auto-remediate $FINDING"` for review.

11. Measure DevSecOps maturity using the OWASP DSOMM (DevSecOps Maturity Model): assess your organization across dimensions like Build & Deployment, Culture & Organization, Information Gathering, and Test & Verification at levels 1-4; create a radar chart showing current vs target maturity and build a quarterly improvement roadmap.

12. Continuously improve the pipeline by reviewing false positive rates and scanner effectiveness: track `true_positives / (true_positives + false_positives)` per scanner, tune rules to maintain precision above 90%, remove ineffective scanners, add new tools when coverage gaps are identified, and conduct quarterly pipeline reviews with security champions to iterate on gate thresholds and suppression policies.
