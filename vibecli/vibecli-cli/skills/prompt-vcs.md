# prompt-vcs

Version-control your prompts — branch, diff, tag, and restore.

## Usage

```
/prompt commit "<message>"         # save current prompt as a new version
/prompt log                        # show version history on current branch
/prompt diff <v0001> <v0002>       # line-level diff between two versions
/prompt checkout <branch>          # switch to a different branch
/prompt branch <name>              # create a new branch from HEAD
/prompt tag <version-id> <label>   # tag a version (e.g. "golden")
/prompt restore <version-id>       # restore a previous version
```

## Features

- Branched version history (default branch: `main`)
- Parent-chain history walk from any version back to root
- LCS-based line diff with Added / Removed / Context hunks
- Tag system for bookmarking golden prompts
- Version IDs are sequential (v0001, v0002, ...)

## Example

```
> /prompt commit "Add JSON output constraint"
  Created v0003 on main (parent: v0002)

> /prompt diff v0001 v0003
  - Please answer in plain text.
  + Please answer in JSON format with keys: summary, steps, confidence.
  (1 line removed, 1 line added, 2 unchanged)

> /prompt tag v0003 golden
  Tagged v0003 as "golden"
```

## Module

`vibecli/vibecli-cli/src/prompt_vcs.rs`
