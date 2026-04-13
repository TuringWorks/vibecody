Feature: Prompt cache — static prefix caching
  The prompt cache computes a deterministic FNV-1a key from system prompt,
  tools JSON, and config JSON, then tracks hits and misses.

  Scenario: Same inputs always produce the same cache key
    Given the system prompt "You are an assistant"
    And the tools json "{}"
    And the config json "{}"
    When I compute the cache key twice
    Then both keys should be equal

  Scenario: Hit and miss counting
    Given a fresh prompt cache
    When I call get_or_insert with the same inputs twice
    Then the miss count should be 1
    And the hit count should be 1

  Scenario: Hit rate is 0.5 after one miss and one hit
    Given a fresh prompt cache
    When I call get_or_insert with the same inputs twice
    Then the hit rate should be 0.5

  Scenario: Invalidating a key removes it from the cache
    Given a fresh prompt cache
    And I have inserted a prefix entry
    When I invalidate that entry
    Then the cache should have 0 entries
