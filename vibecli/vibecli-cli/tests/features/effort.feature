Feature: Per-request effort / compute knob (gap C5)
  A provider-agnostic effort tier (low|medium|high|xhigh) maps onto each
  provider's native reasoning knob — Claude/Gemini extended-thinking budget,
  OpenAI reasoning_effort — and defaults to high.

  Scenario: Effort labels and aliases parse
    Given the effort string "xhigh"
    When I parse the effort
    Then the effort is "xhigh"

  Scenario: Unknown effort strings do not parse
    Given the effort string "bananas"
    When I parse the effort
    Then the effort does not parse

  Scenario: Low disables Claude thinking and higher tiers escalate the budget
    Then the Claude budget for "low" is none
    And the Claude budget for "xhigh" exceeds the budget for "high"

  Scenario: OpenAI reasoning effort clamps xhigh to high
    Then the OpenAI reasoning effort for "xhigh" is "high"
    And the OpenAI reasoning effort for "low" is "low"

  Scenario: Gemini disables thinking on low and escalates with tier
    Then the Gemini budget for "low" is 0
    And the Gemini budget for "xhigh" exceeds the budget for "medium"
