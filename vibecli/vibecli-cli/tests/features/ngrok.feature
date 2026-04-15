Feature: ngrok tunnel auto-detection and startup
  The daemon detects a running ngrok agent at localhost:4040 and reads
  the public HTTPS URL for the daemon port from its local API.

  Scenario: detect_tunnel returns None when nothing listens on 4040
    Given no process is listening on localhost port 4040
    When I call detect_tunnel for port 7878
    Then the result should be None

  Scenario: detect_tunnel returns None for port 0 (never matches)
    Given no process is listening on localhost port 4040
    When I call detect_tunnel for port 0
    Then the result should be None

  Scenario: Parse ngrok API response — matching HTTPS tunnel
    Given a mock ngrok API response with an HTTPS tunnel on port 7878
    When I parse the ngrok API response for port 7878
    Then the extracted URL should be "https://abc123.ngrok.io"

  Scenario: Parse ngrok API response — no matching tunnel on different port
    Given a mock ngrok API response with an HTTPS tunnel on port 9999
    When I parse the ngrok API response for port 7878
    Then the extracted URL should be empty

  Scenario: Parse ngrok API response — HTTP tunnel excluded
    Given a mock ngrok API response with an HTTP-only tunnel on port 7878
    When I parse the ngrok API response for port 7878
    Then the extracted URL should be empty

  Scenario: Parse ngrok API response — empty tunnel list
    Given a mock ngrok API response with no tunnels
    When I parse the ngrok API response for port 7878
    Then the extracted URL should be empty

  Scenario: Parse ngrok API response — malformed JSON
    Given a mock ngrok API response that is invalid JSON
    When I parse the ngrok API response for port 7878
    Then the extracted URL should be empty

  Scenario: TunnelConfig defaults are all off
    When I create a default TunnelConfig
    Then tailscale_funnel should be false
    And ngrok_auto_start should be false
    And ngrok_auth_token should be None

  Scenario: TunnelConfig is serde-roundtrippable
    When I create a TunnelConfig with ngrok_auto_start true and token "tok_abc"
    And I serialise and deserialise the config
    Then ngrok_auto_start should be true
    And ngrok_auth_token should be "tok_abc"
