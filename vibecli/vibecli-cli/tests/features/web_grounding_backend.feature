Feature: Web grounding calls real HTTP search providers
  The WebGroundingEngine must route queries through a pluggable SearchBackend
  so that responses come from live providers (SearXNG, Brave, Tavily) and no
  longer from in-memory placeholder strings. Backend-level failures must surface
  as errors, and successful responses must flow through the engine's cache,
  classifier, and metrics pipeline unchanged.

  Scenario: SearXNG backend fetches real results over HTTP
    Given a mock SearXNG server that returns one result titled "Rust async guide"
    And a web grounding engine configured to target that mock server
    When the engine searches for "rust async"
    Then the engine returns 1 result
    And the first result title is "Rust async guide"
    And the first result source is SearXNG
    And the engine cache has 1 entry
    And the engine metrics report 1 total search and 0 cache hits

  Scenario: Repeated search hits the cache instead of the backend
    Given a mock SearXNG server that returns one result titled "Cached guide"
    And a web grounding engine configured to target that mock server
    When the engine searches for "cache test" twice
    Then the engine metrics report 2 total searches and 1 cache hit
    And the mock server received exactly 1 HTTP request

  Scenario: HTTP failure from the backend surfaces as an error
    Given a mock SearXNG server that always returns HTTP 503
    And a web grounding engine configured to target that mock server
    When the engine searches for "fail case"
    Then the engine returns an error containing "503"
    And the engine cache has 0 entries

  Scenario: Brave backend requires an API key
    Given a web grounding engine configured for Brave without an API key
    When the engine searches for "brave no key"
    Then the engine returns an error containing "API key"
