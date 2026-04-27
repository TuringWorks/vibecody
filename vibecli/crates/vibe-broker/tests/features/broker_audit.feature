Feature: Audit emission for plain HTTP traffic (slice B5.1)
  Every broker decision on the plain-HTTP path emits a structured
  AuditEvent into the configured AuditSink. Tests use MemoryAuditSink
  to assert what landed; production uses a JSONL file sink (B5.3).

  Scenario: deny default emits an egress.request with PolicyDenied outcome
    Given a broker with an in-memory audit sink and empty policy
    When I send "GET http://api.openai.com/v1/messages" through the broker
    Then the audit sink recorded 1 event
    And the audit event 0 outcome is "policy_denied"
    And the audit event 0 host is "api.openai.com"
    And the audit event 0 method is "GET"

  Scenario: SSRF block emits an egress.request with SsrfBlocked outcome
    Given a broker with an in-memory audit sink and a rule allowing "*" methods "GET"
    When I send "GET http://10.0.0.1/internal" through the broker
    Then the audit sink recorded 1 event
    And the audit event 0 outcome is "ssrf_blocked"
    And the audit event 0 host is "10.0.0.1"

  Scenario: allowed stub returns 200 and emits Ok outcome with matched_rule_index
    Given a broker with an in-memory audit sink and a rule allowing "api.example.com" methods "GET"
    When I send "GET http://api.example.com/healthz" through the broker
    Then the audit sink recorded 1 event
    And the audit event 0 outcome is "ok"
    And the audit event 0 host is "api.example.com"
    And the audit event 0 matched_rule_index is 0

  Scenario: malformed request emits an UpstreamError outcome
    Given a broker with an in-memory audit sink and empty policy
    When I send raw bytes "NOTHTTP\r\n\r\n" to the broker
    Then the audit sink recorded 1 event
    And the audit event 0 outcome is "upstream_error"
