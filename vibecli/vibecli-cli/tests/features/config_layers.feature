Feature: Local configuration override layer
  Three-level config: user < project < local. Deep merging preserves
  base keys while letting overlays win on conflicts.

  Scenario: Local overrides project which overrides user
    Given user config with model "gpt-4"
    And project config with model "claude-sonnet"
    And local config with model "ollama"
    When I merge all layers
    Then the merged model should be "ollama"

  Scenario: Deep merge preserves base keys not in overlay
    Given user config with keys x=1 and y=2 in object "a"
    And project config overriding y=3 in object "a"
    When I merge all layers
    Then "a.x" should be 1
    And "a.y" should be 3

  Scenario: Invalid config value reports the correct layer name
    Given a non-object value in the project layer
    When I validate the project layer
    Then the validation error should reference "project"
