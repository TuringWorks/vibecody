Feature: End-to-end sandbox + broker on macOS
  A sandboxed /bin/sh writes inside its bound dir, can reach the broker
  via the bind-allowed UDS path, and gets policy-enforced responses
  (200 stub for allowed hosts, 451 for denied). Reads outside the bound
  dir are still kernel-denied. Direct net is also denied.

  Scenario: sandboxed curl through broker — denied host returns 451
    Given a fresh macOS sandbox with a bound rw temp dir
    And a running broker on a UDS path with empty policy
    And the sandbox profile permits outbound to the broker UDS
    When the sandbox runs curl with --unix-socket pointing at the broker, target "GET http://api.openai.com/v1/messages"
    Then the curl HTTP status code captured was 451
    And the curl X-Vibe-Broker-Reason header captured was "policy_denied"

  Scenario: sandboxed curl through broker — allowed host returns 200
    Given a fresh macOS sandbox with a bound rw temp dir
    And a running broker on a UDS path with a rule allowing "api.example.com" methods "GET"
    And the sandbox profile permits outbound to the broker UDS
    When the sandbox runs curl with --unix-socket pointing at the broker, target "GET http://api.example.com/healthz"
    Then the curl HTTP status code captured was 200
