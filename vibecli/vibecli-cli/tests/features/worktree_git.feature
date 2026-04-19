Feature: Git worktree pool — real git worktrees (US-003)
  GitWorktreePool creates real git worktrees on disk, detaches HEAD to a
  named branch, and merges completed work back into the source repo using
  the git CLI. Failed merges are aborted cleanly so the source repo is
  left on a clean HEAD.

  Scenario: Spawning a worktree creates a new branch and checked-out path
    Given a fresh git repo with a single commit on branch "main"
    When the pool spawns a worktree "wt-1" on branch "feat/x"
    Then the path for worktree "wt-1" exists on disk
    And the worktree HEAD is on branch "feat/x"

  Scenario: Removing a worktree deletes the directory
    Given a fresh git repo with a single commit on branch "main"
    And the pool has spawned a worktree "wt-1" on branch "feat/y"
    When the pool removes worktree "wt-1"
    Then the path for worktree "wt-1" no longer exists

  Scenario: Clean merge carries changes back into the target branch
    Given a fresh git repo with a single commit on branch "main"
    And the pool has spawned a worktree "wt-1" on branch "feat/z"
    And a new file "added.txt" with content "added" is committed in worktree "wt-1"
    When the pool merges worktree "wt-1" into branch "main"
    Then the merge succeeds with no conflicts
    And branch "main" contains file "added.txt"

  Scenario: Conflicting merge reports conflicts and leaves a clean source repo
    Given a fresh git repo with a single commit on branch "main"
    And the pool has spawned a worktree "wt-1" on branch "feat/collide"
    And file "hello.txt" is modified to "A" and committed in worktree "wt-1"
    And file "hello.txt" is modified to "B" and committed on branch "main"
    When the pool merges worktree "wt-1" into branch "main"
    Then the merge reports conflicts in "hello.txt"
    And the source repo working tree is clean

  Scenario: Spawning beyond max capacity returns a capacity error
    Given a fresh git repo with a single commit on branch "main"
    And the pool has a max capacity of 1
    And the pool has spawned a worktree "wt-1" on branch "feat/a"
    When the pool attempts to spawn worktree "wt-2" on branch "feat/b"
    Then the spawn returns an error mentioning "capacity"
