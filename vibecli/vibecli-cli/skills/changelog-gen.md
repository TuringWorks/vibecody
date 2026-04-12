# Changelog Generator

Parses conventional commits and produces structured Markdown changelogs.
Matches Copilot Workspace v2's changelog generation.

## Conventional Commit Types
`feat` | `fix` | `docs` | `style` | `refactor` | `perf` | `test` | `chore` | `ci` | `build`

Notable (in changelog): feat, fix, perf, refactor

## Format
```
type[(scope)][!]: description
```
`!` or `BREAKING CHANGE:` in footer marks breaking changes.

## Commands
- `/changelog generate <version>` — generate from recent commits
- `/changelog preview` — preview without writing
- `/changelog since <tag>` — changelog since a git tag

## Examples
```
/changelog generate 2.0.0
# ## [2.0.0] — 2026-04-12
# ### ⚠ BREAKING CHANGES
# - drop old API (abc1234)
# ### Features
# - **ui**: add dark mode (def5678)
```
