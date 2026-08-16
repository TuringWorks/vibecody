Feature: VibeDesk connectors reach the agent, not merely the panel

  The Connectors panel lists MCP servers a workspace can talk to. Every claim
  it makes — "Added", "enabled", "3 credentials" — is only true if the agent's
  own configuration changes to match. A row in a panel and a server the agent
  can call are different facts, and the panel can only observe the first.

  These scenarios cover the second: what `resolve_mcp_configs` hands the agent
  after each action the panel offers. Launching the servers is a separate,
  network-dependent test (`connector_catalog_bdd`), because a suite that needs
  npx and uvx to pass cannot be the one that runs everywhere.

  # ── The catalog is shippable ──────────────────────────────────────────────

  Scenario: Every catalog connector can be added to a workspace
    Given a fresh workspace
    When I add every credential-free catalog connector
    Then every add succeeds
    And each added connector appears in the workspace listing

  Scenario: A connector that needs credentials reports them until they are given
    Given a fresh workspace
    When I add the "github" connector with no credentials
    Then the add is refused because a credential is missing

  Scenario: Credentials given at add time are stored and satisfy the requirement
    Given a fresh workspace
    When I add the "github" connector with credential "GITHUB_PERSONAL_ACCESS_TOKEN" set to "ghp-fixture-not-real"
    Then the add succeeds
    And the connector reports no missing credentials

  # ── The panel's claims match the agent's configuration ────────────────────

  Scenario: An added connector is handed to the agent
    Given a fresh workspace
    When I add the "filesystem" connector with no credentials
    Then the agent's resolved servers include "filesystem"

  Scenario: A disabled connector is withheld from the agent
    Given a fresh workspace
    When I add the "filesystem" connector with no credentials
    And I disable the "filesystem" connector
    Then the agent's resolved servers do not include "filesystem"
    And the workspace listing still shows "filesystem"

  Scenario: Re-enabling hands it back
    Given a fresh workspace
    When I add the "filesystem" connector with no credentials
    And I disable the "filesystem" connector
    And I enable the "filesystem" connector
    Then the agent's resolved servers include "filesystem"

  Scenario: The workspace placeholder is substituted before the agent sees it
    Given a fresh workspace
    When I add the "filesystem" connector with no credentials
    Then no resolved server argument still contains the workspace placeholder
    And the resolved arguments for "filesystem" name the workspace directory

  # ── Secrets live in the encrypted store ───────────────────────────────────

  Scenario: A credential is never written to the workspace in plaintext
    Given a fresh workspace
    When I add the "github" connector with credential "GITHUB_PERSONAL_ACCESS_TOKEN" set to "ghp-fixture-not-real"
    Then no file in the workspace contains "ghp-fixture-not-real"

  Scenario: The agent receives the credential as an environment variable
    Given a fresh workspace
    When I add the "github" connector with credential "GITHUB_PERSONAL_ACCESS_TOKEN" set to "ghp-fixture-not-real"
    Then the resolved environment for "github" carries "GITHUB_PERSONAL_ACCESS_TOKEN"

  Scenario: Removing a connector deletes its stored secrets
    Given a fresh workspace
    When I add the "github" connector with credential "GITHUB_PERSONAL_ACCESS_TOKEN" set to "ghp-fixture-not-real"
    And I remove the "github" connector
    Then the removal reports 1 deleted secret
    And the agent's resolved servers do not include "github"

  # ── Adding twice ──────────────────────────────────────────────────────────

  Scenario: The same connector cannot be added twice
    Given a fresh workspace
    When I add the "filesystem" connector with no credentials
    And I add the "filesystem" connector with no credentials
    Then the second add is refused as already configured

  Scenario: An unknown connector id is refused
    Given a fresh workspace
    When I add the "not-a-real-connector" connector with no credentials
    Then the add is refused as unknown
