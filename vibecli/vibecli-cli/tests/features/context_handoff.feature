Feature: Cross-provider context handoff
  HandoffContext serializes a full conversation (system prompt, messages,
  tool definitions) to JSON and restores it verbatim, enabling mid-session
  provider switches for cost routing, fallback, and capability routing.

  Scenario: Serialize and deserialize a context with messages and tools
    Given a context from provider "claude" with system prompt "You are a coding assistant"
    And a user message "What is ownership?"
    And an assistant message "Ownership is a memory model."
    And a tool named "read_file" described as "Read a file" with parameters "{}"
    When I serialize the context
    And I deserialize the context
    Then the restored source provider is "claude"
    And the restored message count is 2
    And the restored tool count is 1
    And the restored system prompt is "You are a coding assistant"

  Scenario: Trim to token budget drops oldest messages
    Given a context from provider "openai" with system prompt "sys"
    And 5 user messages prefixed "turn"
    When I trim the context to a budget of 1 token
    Then the message count is less than 5
    And the last remaining message starts with "turn"

  Scenario: for_provider clones context and sets target
    Given a context from provider "claude" with system prompt "Help me code"
    And a user message "Refactor this function"
    When I route the context to provider "gemini"
    Then the routed context has target provider "gemini"
    And the source provider is still "claude"
    And the routed context has 1 messages

  Scenario: HandoffHistory records events and lists unique providers
    Given an empty handoff history
    When I record a handoff from "claude-sonnet" to "claude-haiku" for reason "cost_routing" at message 10
    And I record a handoff from "claude-haiku" to "gemini-flash" for reason "capability_gap" at message 18
    Then the history count is 2
    And the last handoff destination is "gemini-flash"
    And the providers used are "claude-sonnet,claude-haiku,gemini-flash"

  Scenario: Empty context serializes and deserializes cleanly
    Given a context from provider "groq" with no system prompt
    When I serialize the context
    And I deserialize the context
    Then the restored source provider is "groq"
    And the restored message count is 0
    And the restored tool count is 0
