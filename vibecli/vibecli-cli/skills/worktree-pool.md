# Worktree Pool

Parallel agent execution using git worktrees. Spawn N independent agents, each in its own worktree, to work on separate tasks simultaneously. Merge results back with conflict resolution and quality checks.

## When to Use
- Parallelizing independent code changes across multiple files or modules
- Running multiple refactoring tasks that don't overlap in scope
- Testing different implementation approaches side by side
- Speeding up large batch edits by distributing work across agents
- Implementing feature branches in parallel with automated merge

## Commands
- `/worktree spawn <n> <task-description>` — Spawn N agents with task partitioning
- `/worktree status` — Show all active worktree agents and their progress
- `/worktree merge` — Merge all completed worktree results into current branch
- `/worktree cancel <id>` — Cancel a specific worktree agent
- `/worktree logs <id>` — View logs from a specific worktree agent
- `/worktree pool-size <n>` — Set maximum concurrent worktree agents
- `/worktree cleanup` — Remove stale worktrees and reclaim disk space

## Examples
```
/worktree spawn 4 "Add error handling to all API endpoints"
# Partitioned into 4 tasks (12 files each), spawning agents...
# Agent 1: src/api/users.rs, src/api/auth.rs, ...
# Agent 2: src/api/projects.rs, src/api/teams.rs, ...

/worktree status
# Agent 1: 8/12 files done (67%) | Agent 2: 11/12 (92%)
# Agent 3: 12/12 COMPLETE | Agent 4: 6/12 (50%)

/worktree merge
# Merged 4 worktrees, 0 conflicts, 48 files updated
```

## Best Practices
- Keep pool size at or below CPU core count for best performance
- Ensure tasks are truly independent to avoid merge conflicts
- Use cleanup regularly to free disk space from old worktrees
- Review merge diffs before pushing to catch inconsistencies across agents
- Set timeout limits per worktree to prevent runaway agents
