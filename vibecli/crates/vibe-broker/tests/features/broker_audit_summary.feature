Feature: Audit aggregator — summary primitive for recap (slice B5.4)
  The recap subsystem renders job-level summaries from broker audit
  events. This feature exercises the AuditSummary primitive that
  produces the structured data the recap will read: totals by outcome,
  by host, by inject type, plus byte counts.

  Scenario: empty event list produces a zeroed summary
    Given a fresh AuditSummary builder
    When I summarize 0 events
    Then the summary total_requests is 0
    And the summary by_outcome has 0 entries
    And the summary by_host has 0 entries

  Scenario: mixed traffic produces a faithful breakdown
    Given a fresh AuditSummary builder
    When I record an Ok event for host "api.openai.com" with bytes_request 100 and bytes_response 200
    And I record an Ok event for host "api.openai.com" with bytes_request 50 and bytes_response 150
    And I record a PolicyDenied event for host "api.evil.com"
    And I record an SsrfBlocked event for host "10.0.0.1"
    And I summarize the recorded events
    Then the summary total_requests is 4
    And the summary by_outcome ok count is 2
    And the summary by_outcome policy_denied count is 1
    And the summary by_outcome ssrf_blocked count is 1
    And the summary by_host "api.openai.com" count is 2
    And the summary by_host "api.evil.com" count is 1
    And the summary by_host "10.0.0.1" count is 1
    And the summary bytes_request_total is 150
    And the summary bytes_response_total is 350

  Scenario: inject type counts roll up by name
    Given a fresh AuditSummary builder
    When I record an Ok event with inject "Bearer" for host "api.openai.com"
    And I record an Ok event with inject "Bearer" for host "api.openai.com"
    And I record an Ok event with inject "AwsSigV4" for host "s3.amazonaws.com"
    And I summarize the recorded events
    Then the summary by_inject "Bearer" count is 2
    And the summary by_inject "AwsSigV4" count is 1

  Scenario: round-trip via JSONL file matches in-memory summary
    Given a fresh JSONL audit sink at a temp path
    When I record an Ok event for host "api.example.com" with bytes_request 10 and bytes_response 20 to the sink
    And I record a PolicyDenied event for host "api.evil.com" to the sink
    And I summarize the JSONL file
    Then the summary total_requests is 2
    And the summary by_outcome ok count is 1
    And the summary by_outcome policy_denied count is 1
