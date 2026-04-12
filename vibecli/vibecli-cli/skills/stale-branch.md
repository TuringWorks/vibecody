# Stale Branch Detection

Classify git branches by staleness (Active/Dormant/Stale/Zombie), recommend cleanup actions (Keep/Delete/Archive/Review), and assess branch freshness against a base branch using configurable policies.

## When to Use
- Identifying abandoned branches for repository cleanup
- Recommending delete vs archive based on unmerged commits
- Protecting main/master/develop from cleanup operations
- Assessing whether a branch needs rebasing before merge
- Generating cleanup reports sorted by age

## Staleness Labels
| Label | Age | Action |
|---|---|---|
| Active | < 14 days OR has open PR | Keep |
| MergedCleanup | Merged, no PR | Delete |
| Dormant | 14–60 days | Review (if ahead) or Delete |
| Stale | 60–180 days | Archive (if ahead) or Delete |
| Zombie | > 180 days | Delete |

## Freshness Policies
- **WarnOnly** — notify but proceed
- **Block** — prevent action on stale branch
- **AutoRebase** — rebase automatically
- **AutoMergeForward** — merge base into branch

## Commands
- `/branches stale` — List all stale/zombie branches
- `/branches classify <branch>` — Show staleness label and recommended action
- `/branches cleanup` — Show all deletion candidates
- `/branches freshness <branch>` — Assess commits-behind / activity-age
- `/branches archive <branch>` — Tag and archive a stale branch
- `/branches protect <branch>` — Mark a branch as protected

## Examples
```
/branches stale
# feat/old-feature: zombie (245 days), action: delete
# feat/wip-auth: stale (72 days, 3 ahead), action: archive

/branches classify feat/my-branch
# label: dormant, age: 18 days, ahead: 2, action: review

/branches cleanup
# 4 branches recommended for deletion (0 unmerged commits)
```
