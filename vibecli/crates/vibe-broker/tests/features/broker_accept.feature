Feature: Broker accept loop — end-to-end
  The broker listens on a TCP socket, parses each incoming HTTP request,
  runs SSRF + Policy, and returns 451 + Vibe-Broker-Reason header when
  denied or 200 + stub body when allowed (slice B1.4 — real upstream
  forwarding is B1.5).

  Scenario: deny default — request to a host with no rule returns 451
    Given a running broker with empty policy
    When I send "GET http://api.openai.com/v1/messages" through the broker
    Then the broker response status is 451
    And the broker response header "x-vibe-broker-reason" is "policy_denied"

  Scenario: SSRF target is rejected before policy match
    Given a running broker with a rule allowing "*" methods "GET"
    When I send "GET http://10.0.0.1/internal" through the broker
    Then the broker response status is 451
    And the broker response header "x-vibe-broker-reason" is "ssrf_blocked"

  Scenario: matching rule returns 200 stub
    Given a running broker with a rule allowing "api.example.com" methods "GET"
    When I send "GET http://api.example.com/healthz" through the broker
    Then the broker response status is 200

  Scenario: method outside rule returns 451
    Given a running broker with a rule allowing "api.example.com" methods "GET"
    When I send "POST http://api.example.com/healthz" through the broker
    Then the broker response status is 451

  Scenario: malformed request line is rejected
    Given a running broker with empty policy
    When I send raw bytes "NOTHTTP\r\n\r\n" to the broker
    Then the broker raw response starts with "HTTP/1.1 400"
