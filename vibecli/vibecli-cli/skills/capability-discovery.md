# capability-discovery

Dynamic agent capability advertisement and negotiation.

## Usage

```
/capabilities list                         # list all known capabilities
/capabilities find <cap1> [cap2 ...]      # find agents with capabilities
/capabilities negotiate <cap1> [cap2 ...] # find best-matching agent
```

## Features

- Agents advertise named capabilities with optional versions and params
- TTL-based expiry — stale advertisements auto-pruned
- Negotiation: Satisfied / Partial (lists missing) / Unsatisfied outcomes
- Deduplication — re-advertisement overwrites previous
- Built-in capability constants: CODE_EDIT, CODE_REVIEW, FILE_READ, SHELL_EXEC, WEB_SEARCH, DATABASE, DEPLOY, TEST_RUN, GIT_OPS

## Example

```
> /capabilities negotiate code_edit git_ops test_run
✓ Satisfied — agent: worker-03 (CODE_EDIT, GIT_OPS, TEST_RUN)

> /capabilities find deploy
Found 2 agents: worker-05, worker-07
```

## Module

`vibecli/vibecli-cli/src/capability_discovery.rs`
