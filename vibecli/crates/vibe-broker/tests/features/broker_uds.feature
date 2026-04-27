Feature: Broker UDS transport (slice B1.6)
  Sandboxed processes on Linux / macOS reach the broker via a Unix
  domain socket bind-mounted into the sandbox at /run/vibe-broker.sock.
  The same policy + SSRF + forwarding pipeline applies — only the
  acceptor changes.

  Scenario: deny default — UDS client gets 451 on unmatched host
    Given a running broker on a UDS path with empty policy
    When I send "GET http://api.openai.com/v1/messages" through the UDS broker
    Then the UDS broker response status is 451
    And the UDS broker response header "x-vibe-broker-reason" is "policy_denied"

  Scenario: allow rule on UDS broker returns stub 200
    Given a running broker on a UDS path with a rule allowing "api.example.com" methods "GET"
    When I send "GET http://api.example.com/healthz" through the UDS broker
    Then the UDS broker response status is 200

  Scenario: SSRF blocking still applies on UDS transport
    Given a running broker on a UDS path with a rule allowing "*" methods "GET"
    When I send "GET http://10.0.0.1/internal" through the UDS broker
    Then the UDS broker response status is 451
    And the UDS broker response header "x-vibe-broker-reason" is "ssrf_blocked"

  Scenario: bound UDS path is removed on broker shutdown
    Given a running broker on a UDS path with empty policy
    When I shut the broker down
    Then the UDS path no longer exists
