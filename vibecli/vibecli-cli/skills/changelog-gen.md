# Automated Changelog Generator

Git history → conventional changelog. Matches Copilot Workspace v2.

## When to Use
- Generating a CHANGELOG.md before a release
- Producing a release summary from commit history
- Grouping commits by type (feat, fix, perf, etc.)
- Identifying breaking changes across a version range

## Commands
- `/changelog generate [--from <tag>] [--to <tag>]` — Generate changelog
- `/changelog preview` — Preview from last tag to HEAD
- `/changelog write` — Write CHANGELOG.md
- `/changelog types` — Show commit type groupings

## Output Format
```markdown
## [1.2.0] — 2026-04-12
### Features
- feat(auth): add OAuth2 support (abc1234)
### Bug Fixes
- fix: resolve token expiry race (def5678)
### Performance
- perf(cache): reduce lookup latency (ghi9012)
```

## Commit Convention
Follows [Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<scope>): <description>` — types: feat, fix, perf, refactor, docs, test, chore
