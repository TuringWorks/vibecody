Feature: MCP lazy tool schema loading
  The ToolRegistry provides compact stub context for all tools and only
  fetches full JSON schemas on demand, reducing upfront context overhead.

  Scenario: Register stubs and get compact context
    Given a registry with tools "read_file" "write_file" "bash"
    When I get the stubs context
    Then the stubs context should contain "read_file"
    And the stubs context should contain "write_file"
    And the stubs context should contain "bash"

  Scenario: Load schema on demand for selected tool
    Given a registry with tools "read_file" "write_file" "bash"
    When I load the schema for "read_file"
    Then the loaded count should be 1
    And the schema for "read_file" should be available

  Scenario: Savings percentage when selecting subset
    Given a registry with tools "read_file" "write_file" "bash"
    When I compute savings for selecting 1 tool
    Then the savings percentage should be greater than 0

  Scenario: Hit rate tracking
    Given a fresh registry
    When I record 3 hits and 1 miss
    Then the hit rate should be 0.75
