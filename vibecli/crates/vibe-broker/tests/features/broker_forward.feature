Feature: Broker upstream forwarding (slice B1.5)
  When the policy allows a request, the broker opens a TCP connection to
  the resolved upstream, replays the request, and streams the response
  back to the client. The response carries an X-Vibe-Broker-Forwarded:
  true header so callers can distinguish stub from real forwarding.

  Scenario: matching rule against a live stub upstream returns 200 + body
    Given a stub upstream that replies with "200 OK" body "pong"
    And a running broker with upstream forwarding and a rule for the stub host
    When I send "GET /healthz" through the broker to the stub
    Then the broker response status is 200
    And the broker response header "x-vibe-broker-forwarded" is "true"
    And the broker response body equals "pong"

  Scenario: upstream timeout reports 504 and audit reason
    Given a stub upstream that hangs forever
    And a running broker with upstream forwarding timeout 200 ms and a rule for the stub host
    When I send "GET /slow" through the broker to the stub
    Then the broker response status is 504
    And the broker response header "x-vibe-broker-reason" is "upstream_timeout"
