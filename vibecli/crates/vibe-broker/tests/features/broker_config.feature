Feature: BrokerConfig — TOML wiring (slice B6.2)
  The daemon reads one TOML file that wires policy, listener, TLS,
  audit sink, IMDS server, and token-refresher cloud profiles into
  a ready-to-start Broker / ImdsServer / TokenRefresher trio.

  Scenario: minimal TCP listener config parses
    Given a config TOML with TCP listener "127.0.0.1:8080" and policy_id "skill:test"
    When I parse the config
    Then the parsed listener kind is "tcp"
    And the parsed listener address is "127.0.0.1:8080"
    And the parsed policy_id is "skill:test"

  Scenario: UDS listener with TLS + audit + IMDS sections parses
    Given a full config TOML with UDS listener, tls dir, jsonl audit, IMDS section
    When I parse the config
    Then the parsed listener kind is "uds"
    And the parsed tls_ca_dir is "/var/run/vibe-ca"
    And the parsed audit jsonl path is "/var/log/vibe-audit.jsonl"
    And the parsed IMDS role_name is "vibe-broker-role"
    And the parsed IMDS listen_tcp is "127.0.0.1:8181"

  Scenario: refresher with multiple cloud profiles parses
    Given a config TOML with one azure profile and one gcp profile
    When I parse the config
    Then the parsed refresher has 1 azure profiles
    And the parsed refresher has 1 gcp profiles
    And the parsed first azure tenant is "tenant42"
    And the parsed first gcp client_email is "sa@example.iam.gserviceaccount.com"

  Scenario: malformed TOML is rejected with a structured error
    Given a malformed config TOML
    When I parse the config
    Then the parse result is an error
