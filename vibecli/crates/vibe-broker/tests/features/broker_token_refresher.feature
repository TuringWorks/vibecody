Feature: Daemon token refresher (slice B6.1)
  The refresher periodically mints OAuth2 tokens for each registered
  cloud profile and stuffs them into the SecretStore that the broker's
  hot path reads from. This decouples token IO from request handling.

  Scenario: registered Azure profile lands in the SecretStore on first tick
    Given a stub Azure OAuth endpoint returning access_token "azu-refreshed-1" with expires_in 3600
    And an InMemorySecretStore
    And a TokenRefresher with 50ms interval
    When I register the Azure profile "@workspace.azure_default" against the stub
    And I start the refresher
    And I wait for the first refresh
    Then the SecretStore has an Azure token at "@workspace.azure_default" equal to "azu-refreshed-1"

  Scenario: stopping the refresher halts further mints
    Given a stub Azure OAuth endpoint returning access_token "azu-stopped" with expires_in 3600
    And an InMemorySecretStore
    And a TokenRefresher with 50ms interval
    When I register the Azure profile "@workspace.azure_default" against the stub
    And I start the refresher
    And I wait for the first refresh
    And I stop the refresher
    Then the underlying mint count plateaus
