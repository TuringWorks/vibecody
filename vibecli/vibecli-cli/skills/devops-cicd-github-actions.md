---
triggers: ["GitHub Actions", "CI/CD", "workflow yaml", "matrix build", "reusable workflow", "github ci"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# CI/CD with GitHub Actions

When building GitHub Actions workflows:

1. Use `on: push` + `on: pull_request` — run tests on PRs, deploy on merge to main
2. Use matrix strategy for multi-OS/version testing: `matrix: {os: [ubuntu, macos], node: [18, 20]}`
3. Cache dependencies: `actions/cache` with hash of lockfile as key — speeds up installs 5-10x
4. Use `actions/setup-*` for language toolchains — pin versions with `*-version` input
5. Pin action versions by SHA: `uses: actions/checkout@abcdef123` — not `@v4` for supply chain security
6. Use reusable workflows (`workflow_call`) for shared CI logic across repos
7. Use concurrency groups: `concurrency: { group: ${{ github.ref }}, cancel-in-progress: true }`
8. Store secrets in GitHub Secrets — reference with `${{ secrets.API_KEY }}`
9. Use `if: failure()` for notification steps; `if: always()` for cleanup
10. Use `GITHUB_TOKEN` for automated PR comments, releases — scope permissions with `permissions:`
11. Multi-job workflows: `needs: [build, test]` for dependencies between jobs
12. Use `workflow_dispatch` for manual triggers with custom inputs
