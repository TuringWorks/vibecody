Feature: AgentContext depth and counter guards for spawn_agent
  The sub-agent system enforces nesting depth and a global concurrency cap
  to prevent runaway recursive agent trees.

  Background:
    Given a workspace directory

  # ── Depth guard ────────────────────────────────────────────────────────────

  Scenario: Root agent has depth 0
    Given an agent context with no parent
    Then the agent depth is 0

  Scenario: Child context increments depth
    Given an agent context with depth 1
    When a child context is created from it
    Then the child depth is 2

  Scenario: Depth limit of 5 is the hard cap
    Given an agent context with depth 5
    And a depth limit of 10
    Then the effective depth limit is 5

  Scenario: Depth limit requested below hard cap is honoured
    Given an agent context with depth 0
    And a depth limit of 2
    Then the effective depth limit is 2

  # ── Counter guard ──────────────────────────────────────────────────────────

  Scenario: New context has no active agent counter
    Given an agent context with no parent
    Then the active agent counter is absent

  Scenario: Counter is shared across sibling contexts
    Given a shared agent counter at 3
    When a child context is created with that counter
    Then the child context counter reads 3

  Scenario: Counter at the 20-agent limit is detected
    Given a shared agent counter at 20
    Then the counter is at or above the global limit of 20

  Scenario: Counter below limit allows spawning
    Given a shared agent counter at 19
    Then the counter is below the global limit of 20

  # ── AgentContext serde roundtrip ───────────────────────────────────────────

  Scenario: AgentContext serialises and deserialises depth correctly
    Given an agent context with depth 3
    When the context is serialised to JSON
    And the context is deserialised from that JSON
    Then the deserialised depth is 3

  Scenario: AgentContext active_agent_counter is skipped in serialisation
    Given an agent context with a counter
    When the context is serialised to JSON
    Then the JSON does not contain "active_agent_counter"

  # ── ApprovalPolicy ─────────────────────────────────────────────────────────

  Scenario: Sub-agents always run in FullAuto mode
    Given an approval policy string "full-auto"
    Then the policy is FullAuto

  Scenario: Unknown policy strings default to Suggest
    Given an approval policy string "banana"
    Then the policy is Suggest
