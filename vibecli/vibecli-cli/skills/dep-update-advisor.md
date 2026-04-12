# Dependency Update Advisor

SemVer constraint analysis and update safety scoring. Matches Cody 6.0.

## When to Use
- Auditing which dependencies have available updates
- Identifying breaking (major) vs safe (patch) updates before upgrading
- Pinning specific packages to prevent auto-updates
- Generating a prioritised update report sorted by risk

## Safety Levels
| Level | Trigger | Action |
|---|---|---|
| **Safe** | Patch bump (x.y.Z) | Apply automatically |
| **Review Recommended** | Minor bump (x.Y.z) | Review CHANGELOG |
| **Manual Review Required** | Major bump (X.y.z) | Check breaking changes |
| **Unknown** | Pinned or no change | Skip |

## Commands
- `/dep-update analyse` — Analyse all dependencies
- `/dep-update report` — Full markdown report
- `/dep-update summary` — Counts by safety level
- `/dep-update safe` — List only patch-safe updates
- `/dep-update breaking` — List only major/breaking updates
- `/dep-update pin <crate>` — Mark a dependency as pinned

## Example Output
```
# Dependency Update Advisories

## tokio (1.35.0 → 1.36.0)
- Safety: review-recommended
- Bump: Minor
- Review CHANGELOG.

## axum (0.7.0 → 1.0.0)
- Safety: manual-review-required
- Bump: Major
- Check breaking changes.
```
