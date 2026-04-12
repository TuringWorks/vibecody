Feature: Formal trust resolution policy
  Trust policies govern access to workspace directories. Denied paths
  take precedence over allowed paths. Sibling paths are discriminated.
  Content-source trust resolves file/URL/command provenance.

  # ── Content-source trust scenarios ─────────────────────────────────────────

  Scenario: Workspace file gets project trust
    Given a workspace at "/workspace"
    And a local file at "/workspace/src/main.rs"
    When I resolve trust for the file
    Then the trust level should be "project"
    And execution should not be allowed

  Scenario: File outside workspace is untrusted
    Given a workspace at "/workspace"
    And a local file at "/tmp/script.sh"
    When I resolve trust for the file
    Then the trust level should be "untrusted"
    And execution should not be allowed
    And the decision should not be trusted

  Scenario: Remote URL is untrusted
    Given a workspace at "/workspace"
    When I resolve trust for a remote URL
    Then the trust level should be "untrusted"
    And execution should not be allowed

  Scenario: Agent-generated content is untrusted
    Given a workspace at "/workspace"
    When I resolve trust for agent-generated content
    Then the trust level should be "untrusted"
    And the decision should not be trusted

  Scenario: Project-level file is trusted
    Given a workspace at "/workspace"
    And a local file at "/workspace/src/lib.rs"
    When I resolve trust for the file
    Then the decision should be trusted

  # ── Workspace-directory policy trust scenarios ──────────────────────────────

  Scenario: Trust prompt is detected by phrase matching
    Given text "Do you trust this tool to access the workspace?"
    Then is_trust_prompt should return true

  Scenario: Denied paths override allowed paths
    Given an allowed path "/home/user/project"
    And a denied path "/home/user/project"
    When I resolve "/home/user/project"
    Then the policy should be "deny"

  Scenario: Sibling paths are not granted trust
    Given an allowed path "/home/user/safe"
    When I resolve "/home/user/safe-imposter"
    Then the policy should not be "auto_trust"

  Scenario: Trust events create an audit trail
    Given a trust resolver
    When I record 3 trust events
    Then the event log should contain 3 entries
