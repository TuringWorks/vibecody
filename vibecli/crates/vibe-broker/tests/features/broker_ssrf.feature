Feature: Egress broker SSRF guard
  The broker rejects requests targeting localhost, RFC1918 ranges,
  link-local (except IMDS faker opt-in), IPv6 ULA, and known cloud-
  metadata hostnames. Ported from agent_executor.rs:21-56 so the same
  rules cover every tier.

  Scenario: localhost is blocked
    Given a fresh SSRF guard
    When I check "http://127.0.0.1/"
    Then the guard verdict is "Block"

  Scenario: RFC1918 is blocked
    Given a fresh SSRF guard
    When I check "http://10.0.0.1/"
    Then the guard verdict is "Block"
    When I check "http://192.168.1.42/"
    Then the guard verdict is "Block"
    When I check "http://172.16.0.5/"
    Then the guard verdict is "Block"

  Scenario: cloud metadata IPs are blocked by default
    Given a fresh SSRF guard
    When I check "http://169.254.169.254/latest/meta-data/"
    Then the guard verdict is "Block"

  Scenario: cloud metadata hostnames are blocked
    Given a fresh SSRF guard
    When I check "http://metadata.google.internal/"
    Then the guard verdict is "Block"

  Scenario: IPv6 loopback is blocked
    Given a fresh SSRF guard
    When I check "http://[::1]/"
    Then the guard verdict is "Block"

  Scenario: public host is allowed
    Given a fresh SSRF guard
    When I check "https://api.openai.com/v1/messages"
    Then the guard verdict is "Allow"

  Scenario: scheme other than http/https is blocked
    Given a fresh SSRF guard
    When I check "file:///etc/passwd"
    Then the guard verdict is "Block"

  Scenario: IMDS opt-in for fake-IMDS only
    Given an SSRF guard with IMDS faker enabled
    When I check "http://169.254.169.254/latest/meta-data/"
    Then the guard verdict is "Allow"
