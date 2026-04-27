Feature: Memory projections — readable USER.md and MEMORY.md surfaces
  Phase 6 of the memory-as-infrastructure redesign. The projection
  renderer turns OpenMemory state into a grouped markdown file for human
  inspection. These files are regenerated overwrites — never sources of
  truth.

  Scenario: Writing projections creates MEMORY.md at the expected path
    Given a fresh workspace
    When projections are written with no home directory
    Then the file ".vibecli/MEMORY.md" exists in the workspace
    And the file ".vibecli/MEMORY.md" starts with "# Project Memory — "

  Scenario: Writing projections with a home directory also emits USER.md
    Given a fresh workspace
    And a fresh home directory
    When projections are written with the home directory
    Then the file ".vibecli/USER.md" exists in the home directory
    And the file ".vibecli/USER.md" starts with "# User Memory"

  Scenario: Projection render is deterministic
    Given a fresh workspace
    When projections are written with no home directory
    And projections are written with no home directory
    Then the MEMORY.md bytes match between the two runs

  Scenario: Empty store produces a friendly hint
    Given a fresh workspace
    When projections are written with no home directory
    Then the file ".vibecli/MEMORY.md" contains "No memories yet"
    And the file ".vibecli/MEMORY.md" contains "Total memories: **0**"

  # Phase 7 quick-win: auto-refresh on OpenMemoryStore::save()
  # so the human-visible MEMORY.md / USER.md don't go stale.

  Scenario: Save without projection refresh leaves MEMORY.md absent
    Given a fresh workspace
    And a project-scoped open memory store
    When a memory "an unprojected fact" is added
    And the store is saved
    Then the file ".vibecli/MEMORY.md" does not exist in the workspace

  Scenario: Save with projection refresh writes MEMORY.md reflecting state
    Given a fresh workspace
    And a project-scoped open memory store
    And projection refresh is enabled with no home
    When a memory "a fact about Rust ownership" is added
    And the store is saved
    Then the file ".vibecli/MEMORY.md" exists in the workspace
    And the file ".vibecli/MEMORY.md" contains "a fact about Rust ownership"

  Scenario: Save with projection refresh and home also writes USER.md
    Given a fresh workspace
    And a fresh home directory
    And a project-scoped open memory store
    And projection refresh is enabled with the home directory
    When a memory "shared project knowledge" is added
    And the store is saved
    Then the file ".vibecli/MEMORY.md" exists in the workspace
    And the file ".vibecli/USER.md" exists in the home directory
