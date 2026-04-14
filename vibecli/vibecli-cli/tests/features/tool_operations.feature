Feature: Pluggable tool I/O via BashOperations and EditOperations
  As a VibeCody tool author
  I want to redirect built-in tools to different backends
  So that the same agent code works locally, over SSH, and inside Docker

  Scenario: DryRunBashOps records commands without executing them
    Given a dry-run bash backend
    When I run the command "rm -rf /important"
    And I run the command "git push --force origin main"
    Then 2 commands should be recorded
    And recorded command 1 should be "rm -rf /important"
    And recorded command 2 should be "git push --force origin main"
    And no files on disk should have changed

  Scenario: MemoryEditOps write then read round-trip
    Given a memory edit backend
    When I write "fn hello() -> &str { \"world\" }" to path "src/hello.rs"
    Then reading "src/hello.rs" should return "fn hello() -> &str { \"world\" }"
    And "src/hello.rs" should exist in the backend

  Scenario: MemoryEditOps patch replaces old text with new text
    Given a memory edit backend seeded with path "src/config.rs" and content "let timeout = 30;"
    When I apply a patch to "src/config.rs" replacing "let timeout = 30;" with "let timeout = 60;"
    Then the patch should succeed
    And reading "src/config.rs" should return "let timeout = 60;"

  Scenario: OpsRegistry dispatches to named backends
    Given a registry with a dry-run bash backend registered as "preview"
    And a registry with a memory edit backend registered as "scratch"
    When I look up bash backend "preview"
    Then the bash backend name should be "dry-run"
    When I look up edit backend "scratch"
    Then the edit backend name should be "memory"

  Scenario: EchoBashOps returns command string as stdout
    Given an echo bash backend
    When I run the command "cargo test --workspace"
    Then the output stdout should equal "cargo test --workspace"
    And the output exit code should be 0
    And the backend name should be "echo"
