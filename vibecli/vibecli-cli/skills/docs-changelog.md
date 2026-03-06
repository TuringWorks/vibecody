---
triggers: ["changelog", "conventional commits", "semver", "release notes", "CHANGELOG.md", "version bump"]
tools_allowed: ["read_file", "write_file", "bash"]
category: documentation
---

# Changelog & Release Notes

When managing changelogs and releases:

1. Use Conventional Commits: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `perf:`, `test:`
2. Breaking changes: `feat!:` or `BREAKING CHANGE:` in footer — triggers major version bump
3. SemVer: MAJOR (breaking) . MINOR (features) . PATCH (fixes) — `1.2.3`
4. Keep a CHANGELOG.md: `## [Unreleased]` at top, then `## [1.2.0] - 2024-01-15`
5. Sections: Added, Changed, Deprecated, Removed, Fixed, Security
6. Write for users, not developers: explain impact, not implementation details
7. Use tools: `conventional-changelog`, `release-please`, `changesets` for automation
8. Tag releases in git: `git tag -a v1.2.0 -m "Release 1.2.0"` — signed tags for production
9. Include upgrade instructions for breaking changes — migration guides in release notes
10. Link PRs and issues: `Fixed login timeout (#123)` — traceability matters
11. Pre-release versions: `1.2.0-beta.1`, `1.2.0-rc.1` — test before stable release
12. Automate: CI generates changelog from commits, creates GitHub Release with assets
