Feature: Thinking levels — 6-level reasoning abstraction
  VibeCody exposes a 6-level thinking abstraction (off/minimal/low/medium/high/xhigh)
  that maps to provider-specific parameters and per-level token budgets.
  Users can select a level via the "model:level" shorthand on the CLI.

  Scenario: Parse model:level shorthand with a recognised level
    Given the model shorthand "sonnet:high"
    When I parse the model shorthand
    Then the model name should be "sonnet"
    And the thinking level should be "high"

  Scenario: Parse model shorthand without a level defaults to off
    Given the model shorthand "gpt-4o"
    When I parse the model shorthand
    Then the model name should be "gpt-4o"
    And the thinking level should be "off"

  Scenario: Token budget per level
    Given the thinking level "off"
    Then the token budget for level "off" should be 0
    And the token budget for level "minimal" should be 200
    And the token budget for level "low" should be 1000
    And the token budget for level "medium" should be 5000
    And the token budget for level "high" should be 10000
    And the token budget for level "xhigh" should be 32000

  Scenario: Provider config for Anthropic
    Given the thinking level "high"
    When I build the Anthropic provider config
    Then the config should be enabled
    And the provider param should be "budget_tokens"
    And the config token budget should be 10000

  Scenario: Provider config for OpenAI maps effort tiers
    Given the thinking level "low"
    When I build the OpenAI provider config
    Then the config should be enabled
    And the provider param should contain "reasoning_effort:low"

    Given the thinking level "medium"
    When I build the OpenAI provider config
    Then the provider param should contain "reasoning_effort:medium"

    Given the thinking level "xhigh"
    When I build the OpenAI provider config
    Then the provider param should contain "reasoning_effort:high"

  Scenario: Provider config for Gemini uses thinkingBudget
    Given the thinking level "medium"
    When I build the Gemini provider config
    Then the config should be enabled
    And the provider param should contain "thinkingConfig.thinkingBudget:5000"

  Scenario: default_for_task auto-selects level
    Given the task hint "SimpleEdit"
    Then the auto-selected level should be "minimal"

    Given the task hint "Debugging"
    Then the auto-selected level should be "medium"

    Given the task hint "Architecture"
    Then the auto-selected level should be "high"

    Given the task hint "ComplexReasoning"
    Then the auto-selected level should be "xhigh"

    Given the task hint "Unknown"
    Then the auto-selected level should be "low"

  Scenario: Budget override replaces default for a specific level
    Given a budget override of 3500 tokens for level "medium"
    When I resolve the budget for level "medium"
    Then the resolved budget should be 3500

  Scenario: Budget override does not affect other levels
    Given a budget override of 3500 tokens for level "medium"
    When I resolve the budget for level "high"
    Then the resolved budget should be 10000
