# Dependency Update Advisor

Analyzes SemVer constraints and assesses update safety. Matches Cody 6.0's dependency intelligence.

## Risk Levels
| Level | Condition | Auto-Update? |
|---|---|---|
| patch (safe) | patch version bump | ✓ |
| minor (safe) | minor version bump | ✓ |
| major (breaking possible) | major version bump | manual |
| unstable | pre-release version | manual |

## Output
- Sorted: major first (needs most attention), then by package name
- Known breaking change notes from a curated registry
- `safe_updates()` — only patch + minor changes

## Commands
- `/deps analyze` — show all outdated packages
- `/deps safe` — show only safe-to-auto-update packages
- `/deps report` — full advisory Markdown report
- `/deps update <package>` — apply a specific update
