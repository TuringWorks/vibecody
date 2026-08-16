Feature: VibeDesk plugins take effect, not merely appear

  The Plugins panel in VibeDesk installs from a catalog compiled into the
  binary. Every scenario here exists because the panel makes a claim — "Added",
  "N live components", "enabled" — and a claim about a plugin is only true if
  the agent run changes. Listing a rule and obeying one are two different
  facts, and the panel can only observe the first.

  So the scenarios come in three groups: the catalog is shippable, the panel's
  claims match the workspace, and an installed plugin reaches the agent.

  # ── The catalog is shippable ────────────────────────────────────────────

  Scenario: Every catalog plugin installs and writes its component files
    Given a fresh workspace
    When I install every catalog plugin
    Then every install succeeds
    And every declared component has a non-empty file on disk

  Scenario: Every catalog skill parses under the agent's own skill loader
    Given a fresh workspace
    When I install every catalog plugin
    Then the agent's skill loader parses every installed skill

  Scenario: Every catalog skill declares at least one trigger
    Given a fresh workspace
    When I install every catalog plugin
    Then every installed skill declares at least one trigger

  Scenario: Every catalog skill's declared name equals its file stem
    Then every catalog skill name equals its file stem

  Scenario: Every bundle's includes name real catalog plugins
    Then every bundle include resolves to a catalog plugin

  Scenario: Every bundle's connectors name real connector-catalog entries
    Then every bundle connector resolves to a connector spec

  Scenario: No two catalog plugins ship a component of the same name
    Then no component name is claimed by two catalog plugins

  Scenario: No catalog plugin ships a component kind that nothing loads
    Then no catalog plugin ships an mcp server, hook or subagent

  # ── The panel's claims match the workspace ─────────────────────────────

  Scenario: An installed plugin's components appear in the panel inventory
    Given a fresh workspace
    When I install the plugin "core-secure-defaults"
    Then the panel inventory lists 2 components
    And the panel inventory credits them to "core-secure-defaults"

  Scenario: Disabling a plugin empties the panel inventory
    Given a fresh workspace
    When I install the plugin "core-test-first"
    And I disable the plugin "core-test-first"
    Then the panel inventory lists 0 components

  Scenario: Re-enabling a disabled plugin restores its components
    Given a fresh workspace
    When I install the plugin "core-test-first"
    And I disable the plugin "core-test-first"
    And I enable the plugin "core-test-first"
    Then the panel inventory lists 1 components

  Scenario: Installing a bundle installs the plugins it includes
    Given a fresh workspace
    When I install the bundle "bundle-engineering"
    Then the plugin "core-test-first" is installed
    And the plugin "core-debugging" is installed
    And the plugin "core-review-standards" is installed

  # ── An installed plugin reaches the agent ──────────────────────────────

  Scenario: An installed rule plugin reaches the agent's system prompt
    Given a fresh workspace
    When I install the plugin "core-secure-defaults"
    Then the agent context carries plugin rules
    And the agent's plugin rules mention "secret-handling"

  Scenario: A disabled rule plugin does not reach the agent's system prompt
    Given a fresh workspace
    When I install the plugin "core-secure-defaults"
    And I disable the plugin "core-secure-defaults"
    Then the agent context carries no plugin rules

  Scenario: A workspace with no plugins carries no plugin rules
    Given a fresh workspace
    Then the agent context carries no plugin rules

  Scenario: An installed skill plugin is found by the agent's skill loader
    Given a fresh workspace
    When I install the plugin "core-test-first"
    Then the agent context carries 1 extra skill directories
    And the agent's skill loader activates "test-first" for the task "write a test for the parser"

  Scenario: A disabled skill plugin is not found by the agent's skill loader
    Given a fresh workspace
    When I install the plugin "core-test-first"
    And I disable the plugin "core-test-first"
    Then the agent context carries 0 extra skill directories

  Scenario: A skill whose triggers do not match the task stays inactive
    Given a fresh workspace
    When I install the plugin "core-test-first"
    Then the agent's skill loader does not activate "test-first" for the task "rename a variable"

  Scenario: Every catalog skill can be activated by some task
    Given a fresh workspace
    When I install every catalog plugin
    Then every installed skill activates for a task quoting one of its triggers
