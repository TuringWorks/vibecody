Feature: BrokerDaemon entry point (slice B6.3)
  One call assembles the broker, IMDS faker, and token refresher from
  a TOML config, returns a DaemonHandle that exposes the bound
  listener + IMDS addresses, and tears the whole thing down on
  shutdown.

  Scenario: minimal TCP daemon serves traffic and audits to JSONL
    Given a temp dir
    And a policy file in the temp dir with one rule for "api.example.com"
    And a broker config in the temp dir with TCP listener and the policy + audit JSONL
    When I start the daemon from the config
    Then the daemon listener address is a real bound port
    When I send "GET http://api.example.com/healthz" through the daemon
    Then the daemon response status is 200
    When I shut the daemon down
    Then the audit JSONL file has at least 1 line

  Scenario: IMDS section spawns the faker bound to the configured port
    Given a temp dir
    And a broker config in the temp dir with TCP listener, no policy, IMDS section bound to 127.0.0.1:0
    When I start the daemon from the config
    Then the daemon IMDS address is a real bound port

  Scenario: refresher with azure profile registers and ticks at least once
    Given a temp dir
    And a stub Azure OAuth endpoint returning access_token "azu-from-daemon" with expires_in 3600
    And a broker config in the temp dir with TCP listener, refresher 50ms, azure profile pointed at the stub
    When I start the daemon from the config
    And I wait up to 2 seconds for the SecretStore to have an Azure token at "@workspace.azure_default"
    Then the SecretStore Azure token equals "azu-from-daemon"
